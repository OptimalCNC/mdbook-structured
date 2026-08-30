use std::collections::BTreeMap;
use std::ops::Range;
use std::path::Path;

use mdbook_markdown::pulldown_cmark::{Event, LinkType, Tag};
use mdbook_markdown::{MarkdownOptions, new_cmark_parser};

use crate::destination::{AuthoredDestination, LocalDestination};
use crate::{
    AliasCandidateFact, AppDiagnostic, AppDiagnosticKind, ChapterTarget, LinkResolution,
    LinkRouteMap, LogicalChapterPath, MdBookPathHazardFact,
};

struct DestinationEdit {
    range: Range<usize>,
    replacement: String,
}

struct ScannedDestination {
    range: Range<usize>,
    angle_wrapped: bool,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct DefinitionKey {
    start: usize,
    end: usize,
}

#[derive(Default)]
struct ReferenceUses {
    normal_link: bool,
    image: bool,
}

struct ReferenceDefinition {
    label: String,
    destination: String,
    span: Range<usize>,
}

pub fn rewrite_chapter_links(
    current_path: &LogicalChapterPath,
    markdown: &str,
    routes: &LinkRouteMap,
) -> Result<String, AppDiagnostic> {
    let definition_parser = new_cmark_parser(markdown, &MarkdownOptions::default());
    let reference_definitions = definition_parser.reference_definitions().clone();
    let definitions_by_span: BTreeMap<_, _> = reference_definitions
        .iter()
        .map(|(label, definition)| {
            (
                definition_key(&definition.span),
                ReferenceDefinition {
                    label: label.to_owned(),
                    destination: definition.dest.to_string(),
                    span: definition.span.clone(),
                },
            )
        })
        .collect();
    let mut reference_uses = BTreeMap::<DefinitionKey, ReferenceUses>::new();
    let mut edits = Vec::new();

    let parser = new_cmark_parser(markdown, &MarkdownOptions::default());
    for (event, source_range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Link {
                link_type: LinkType::Inline,
                dest_url,
                ..
            }) => {
                let scanned = scan_inline_destination(markdown, source_range, dest_url.as_ref())?;
                let raw_destination = markdown
                    .get(scanned.range.clone())
                    .ok_or_else(protocol_error)?;
                if let Some(edit) = build_edit(
                    current_path,
                    scanned.range,
                    dest_url.as_ref(),
                    raw_destination,
                    routes,
                )? {
                    edits.push(edit);
                }
            }
            Event::Start(Tag::Link { link_type, id, .. }) if is_reference_link_type(link_type) => {
                record_reference_use(
                    &reference_definitions,
                    id.as_ref(),
                    &mut reference_uses,
                    false,
                )?;
            }
            Event::Start(Tag::Image { link_type, id, .. }) if is_reference_link_type(link_type) => {
                record_reference_use(
                    &reference_definitions,
                    id.as_ref(),
                    &mut reference_uses,
                    true,
                )?;
            }
            _ => {}
        }
    }

    for (key, uses) in reference_uses {
        if !uses.normal_link {
            continue;
        }
        let definition = definitions_by_span.get(&key).ok_or_else(protocol_error)?;
        let scanned =
            scan_reference_destination(markdown, definition.span.clone(), &definition.destination)?;
        let raw_destination = markdown
            .get(scanned.range.clone())
            .ok_or_else(protocol_error)?;
        let Some(edit) = build_edit(
            current_path,
            scanned.range,
            &definition.destination,
            raw_destination,
            routes,
        )?
        else {
            continue;
        };

        if uses.image {
            return Err(AppDiagnostic::from_kind(
                AppDiagnosticKind::MixedReferenceUse {
                    reference_label: definition.label.clone(),
                    destination: definition.destination.clone(),
                },
                "a changed reference definition is shared by a link and an image".to_owned(),
            ));
        }
        edits.push(edit);
    }

    apply_edits(markdown, edits)
}

fn is_reference_link_type(link_type: LinkType) -> bool {
    matches!(
        link_type,
        LinkType::Reference | LinkType::Collapsed | LinkType::Shortcut
    )
}

fn record_reference_use(
    definitions: &mdbook_markdown::pulldown_cmark::RefDefs<'_>,
    id: &str,
    uses: &mut BTreeMap<DefinitionKey, ReferenceUses>,
    image: bool,
) -> Result<(), AppDiagnostic> {
    let definition = definitions.get(id).ok_or_else(protocol_error)?;
    let uses = uses.entry(definition_key(&definition.span)).or_default();
    if image {
        uses.image = true;
    } else {
        uses.normal_link = true;
    }
    Ok(())
}

fn definition_key(span: &Range<usize>) -> DefinitionKey {
    DefinitionKey {
        start: span.start,
        end: span.end,
    }
}

fn build_edit(
    current_path: &LogicalChapterPath,
    range: Range<usize>,
    authored: &str,
    raw_authored: &str,
    routes: &LinkRouteMap,
) -> Result<Option<DestinationEdit>, AppDiagnostic> {
    let AuthoredDestination::Local(destination) =
        AuthoredDestination::parse(current_path, authored)?
    else {
        return Ok(None);
    };

    let target = match routes.resolve(destination.lookup()) {
        LinkResolution::Exact(target) | LinkResolution::UniqueAlias(target) => target,
        LinkResolution::Ambiguous(candidates) => {
            return Err(ambiguous_alias(destination.lookup(), candidates)?);
        }
        LinkResolution::Missing => return Ok(None),
    };
    let replacement = rewritten_destination(current_path, &destination, raw_authored, target)?;
    if replacement == authored {
        return Ok(None);
    }
    if replacement.contains(".md") {
        return Err(AppDiagnostic::from_kind(
            AppDiagnosticKind::MdBookPathHazard {
                fact: MdBookPathHazardFact::RewrittenDestination(replacement),
            },
            "a rewritten Markdown destination contains literal .md".to_owned(),
        ));
    }

    Ok(Some(DestinationEdit { range, replacement }))
}

fn ambiguous_alias(
    authored_path: &LogicalChapterPath,
    candidates: &[ChapterTarget],
) -> Result<AppDiagnostic, AppDiagnostic> {
    let mut facts = candidates
        .iter()
        .map(|candidate| {
            let source_path = candidate
                .source_path()
                .cloned()
                .ok_or_else(protocol_error)?;
            Ok(AliasCandidateFact::new(
                source_path,
                candidate.output_route().clone(),
            ))
        })
        .collect::<Result<Vec<_>, AppDiagnostic>>()?;
    if facts.len() < 2 {
        return Err(protocol_error());
    }

    let additional = facts.split_off(2);
    let second = facts.pop().ok_or_else(protocol_error)?;
    let first = facts.pop().ok_or_else(protocol_error)?;
    Ok(AppDiagnostic::from_kind(
        AppDiagnosticKind::AmbiguousAlias {
            authored_path: authored_path.clone(),
            first,
            second,
            additional,
        },
        "an authored Markdown destination matches multiple README aliases".to_owned(),
    ))
}

fn rewritten_destination(
    current_path: &LogicalChapterPath,
    destination: &LocalDestination<'_>,
    raw_authored: &str,
    target: &ChapterTarget,
) -> Result<String, AppDiagnostic> {
    let current_parent = current_path
        .as_path()
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let output_path = target.output_route().as_path();
    let relative = relative_path(current_parent, output_path);
    let mut rewritten = preserve_percent_spelling(
        current_path,
        destination.raw_path(),
        destination.lookup(),
        output_path,
        &relative,
    )?;
    let (_, raw_query, raw_fragment) = split_raw_destination(raw_authored);
    if raw_query.is_some() != destination.raw_query().is_some()
        || raw_fragment.is_some() != destination.raw_fragment().is_some()
    {
        return Err(protocol_error());
    }
    if let Some(query) = raw_query {
        rewritten.push_str(query);
    }
    if let Some(fragment) = raw_fragment {
        rewritten.push_str(fragment);
    }
    Ok(rewritten)
}

fn split_raw_destination(authored: &str) -> (&str, Option<&str>, Option<&str>) {
    let fragment_start = authored.find('#').unwrap_or(authored.len());
    let before_fragment = &authored[..fragment_start];
    let query_start = before_fragment.find('?').unwrap_or(before_fragment.len());
    (
        &before_fragment[..query_start],
        (query_start < before_fragment.len()).then_some(&authored[query_start..fragment_start]),
        (fragment_start < authored.len()).then_some(&authored[fragment_start..]),
    )
}

fn relative_path(from: &Path, to: &Path) -> String {
    let from: Vec<_> = from.iter().collect();
    let to: Vec<_> = to.iter().collect();
    let shared = from
        .iter()
        .zip(&to)
        .take_while(|(left, right)| left == right)
        .count();
    let mut components = Vec::new();
    components.extend((shared..from.len()).map(|_| "..".to_owned()));
    components.extend(
        to[shared..]
            .iter()
            .map(|component| component.to_string_lossy().into_owned()),
    );
    components.join("/")
}

fn preserve_percent_spelling(
    current_path: &LogicalChapterPath,
    raw_path: &str,
    lookup: &LogicalChapterPath,
    output_path: &Path,
    relative: &str,
) -> Result<String, AppDiagnostic> {
    if !raw_path.contains('%') {
        return Ok(relative.to_owned());
    }

    let mut raw_segments: Vec<_> = current_path
        .as_path()
        .parent()
        .into_iter()
        .flat_map(|parent| parent.iter())
        .map(|segment| segment.to_string_lossy().into_owned())
        .collect();
    for segment in raw_path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                raw_segments.pop();
            }
            segment => raw_segments.push(segment.to_owned()),
        }
    }

    let normalized_raw = raw_segments.join("/");
    let lookup_slash = slash_form(lookup.as_path());
    let output_slash = slash_form(output_path);
    let preserved_prefix_len = common_utf8_prefix_len(&lookup_slash, &output_slash);
    let preserved_raw_end = raw_offset_for_decoded_prefix(&normalized_raw, preserved_prefix_len)
        .ok_or_else(protocol_error)?;

    let relative_segments: Vec<_> = relative.split('/').collect();
    let leading_parents = relative_segments
        .iter()
        .take_while(|component| **component == "..")
        .count();
    let output_start = common_prefix_len(current_path.as_path().parent(), output_path);
    let relative_output_start = decoded_component_prefix_len(output_path, output_start);
    let encoded_suffix = if relative_output_start <= preserved_prefix_len {
        let raw_output_start =
            raw_offset_for_decoded_prefix(&normalized_raw, relative_output_start)
                .ok_or_else(protocol_error)?;
        format!(
            "{}{}",
            &normalized_raw[raw_output_start..preserved_raw_end],
            &output_slash[preserved_prefix_len..],
        )
    } else {
        output_slash[relative_output_start..].to_owned()
    };

    Ok(format!("{}{encoded_suffix}", "../".repeat(leading_parents)))
}

fn slash_form(path: &Path) -> String {
    path.iter()
        .map(|segment| segment.to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn common_utf8_prefix_len(left: &str, right: &str) -> usize {
    left.chars()
        .zip(right.chars())
        .take_while(|(left, right)| left == right)
        .map(|(character, _)| character.len_utf8())
        .sum()
}

fn raw_offset_for_decoded_prefix(raw: &str, decoded_prefix_len: usize) -> Option<usize> {
    let bytes = raw.as_bytes();
    let mut raw_offset = 0;
    let mut decoded_len = 0;

    while decoded_len < decoded_prefix_len {
        if bytes.get(raw_offset) == Some(&b'%') {
            hex_value(*bytes.get(raw_offset + 1)?)?;
            hex_value(*bytes.get(raw_offset + 2)?)?;
            raw_offset += 3;
        } else {
            raw_offset += 1;
        }
        decoded_len += 1;
    }

    (decoded_len == decoded_prefix_len && raw.is_char_boundary(raw_offset)).then_some(raw_offset)
}

fn decoded_component_prefix_len(path: &Path, component_count: usize) -> usize {
    path.iter()
        .take(component_count)
        .map(|component| component.to_string_lossy().len() + 1)
        .sum()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn common_prefix_len(left: Option<&Path>, right: &Path) -> usize {
    left.into_iter()
        .flat_map(|path| path.iter())
        .zip(right.iter())
        .take_while(|(left, right)| left == right)
        .count()
}

fn scan_inline_destination(
    markdown: &str,
    source_range: Range<usize>,
    parser_destination: &str,
) -> Result<ScannedDestination, AppDiagnostic> {
    let source = markdown
        .get(source_range.clone())
        .ok_or_else(protocol_error)?;
    let bytes = source.as_bytes();
    let mut matches = Vec::new();

    for opening in 1..bytes.len() {
        if bytes[opening] != b'(' || bytes[opening - 1] != b']' {
            continue;
        }
        let Some(scanned) = scan_inline_suffix(source, opening) else {
            continue;
        };
        let raw = &source[scanned.range.clone()];
        if decode_scanned_destination(raw, scanned.angle_wrapped).as_deref()
            == Some(parser_destination)
        {
            matches.push(ScannedDestination {
                range: (source_range.start + scanned.range.start)
                    ..(source_range.start + scanned.range.end),
                angle_wrapped: scanned.angle_wrapped,
            });
        }
    }

    match matches.len() {
        1 => Ok(matches.remove(0)),
        _ => Err(protocol_error()),
    }
}

fn scan_inline_suffix(source: &str, opening: usize) -> Option<ScannedDestination> {
    let bytes = source.as_bytes();
    let mut cursor = opening + 1;
    scan_link_separator(bytes, &mut cursor);
    let (range, angle_wrapped) = scan_destination(bytes, &mut cursor)?;

    scan_link_separator(bytes, &mut cursor);
    if matches!(bytes.get(cursor), Some(b'\'' | b'"' | b'(')) {
        cursor = scan_title(bytes, cursor)?;
        scan_link_separator(bytes, &mut cursor);
    }
    (bytes.get(cursor) == Some(&b')') && cursor + 1 == bytes.len()).then_some(ScannedDestination {
        range,
        angle_wrapped,
    })
}

fn scan_reference_destination(
    markdown: &str,
    source_range: Range<usize>,
    parser_destination: &str,
) -> Result<ScannedDestination, AppDiagnostic> {
    let source = markdown
        .get(source_range.clone())
        .ok_or_else(protocol_error)?;
    let bytes = source.as_bytes();
    let mut matches = Vec::new();

    for colon in 0..bytes.len() {
        if bytes[colon] != b':' {
            continue;
        }
        let mut cursor = colon + 1;
        scan_link_separator(bytes, &mut cursor);
        let Some((range, angle_wrapped)) = scan_destination(bytes, &mut cursor) else {
            continue;
        };
        scan_link_separator(bytes, &mut cursor);
        if matches!(bytes.get(cursor), Some(b'\'' | b'"' | b'(')) {
            let Some(after_title) = scan_title(bytes, cursor) else {
                continue;
            };
            cursor = after_title;
            scan_link_separator(bytes, &mut cursor);
        }
        if cursor != bytes.len() {
            continue;
        }

        let raw = &source[range.clone()];
        if decode_scanned_destination(raw, angle_wrapped).as_deref() == Some(parser_destination) {
            matches.push(ScannedDestination {
                range: (source_range.start + range.start)..(source_range.start + range.end),
                angle_wrapped,
            });
        }
    }

    match matches.len() {
        1 => Ok(matches.remove(0)),
        _ => Err(protocol_error()),
    }
}

fn scan_destination(bytes: &[u8], cursor: &mut usize) -> Option<(Range<usize>, bool)> {
    if bytes.get(*cursor) == Some(&b'<') {
        *cursor += 1;
        let start = *cursor;
        while *cursor < bytes.len() {
            match bytes[*cursor] {
                b'\n' | b'\r' | b'<' => return None,
                b'>' => break,
                b'\\'
                    if bytes
                        .get(*cursor + 1)
                        .is_some_and(|byte| is_ascii_punctuation(*byte)) =>
                {
                    *cursor += 2;
                    continue;
                }
                _ => *cursor += 1,
            }
        }
        let end = *cursor;
        (bytes.get(*cursor).is_some_and(|byte| *byte == b'>')).then(|| *cursor += 1)?;
        return Some((start..end, true));
    }

    let start = *cursor;
    let mut nesting = 0usize;
    while *cursor < bytes.len() {
        match bytes[*cursor] {
            0x00..=0x20 => break,
            b'(' => {
                nesting += 1;
                *cursor += 1;
            }
            b')' if nesting == 0 => break,
            b')' => {
                nesting -= 1;
                *cursor += 1;
            }
            b'\\'
                if bytes
                    .get(*cursor + 1)
                    .is_some_and(|byte| is_ascii_punctuation(*byte)) =>
            {
                *cursor += 2;
            }
            _ => *cursor += 1,
        }
    }
    (nesting == 0).then_some((start..*cursor, false))
}

fn scan_title(bytes: &[u8], mut cursor: usize) -> Option<usize> {
    let opening = bytes[cursor];
    let closing = if opening == b'(' { b')' } else { opening };
    cursor += 1;
    while cursor < bytes.len() {
        match bytes[cursor] {
            byte if byte == closing => return Some(cursor + 1),
            byte if byte == opening => return None,
            b'\\'
                if bytes
                    .get(cursor + 1)
                    .is_some_and(|byte| is_ascii_punctuation(*byte)) =>
            {
                cursor += 2;
            }
            _ => cursor += 1,
        }
    }
    None
}

fn scan_link_separator(bytes: &[u8], cursor: &mut usize) {
    while bytes
        .get(*cursor)
        .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
    {
        *cursor += 1;
    }
    let line_break_len = match bytes.get(*cursor..) {
        Some([b'\r', b'\n', ..]) => 2,
        Some([b'\r' | b'\n', ..]) => 1,
        _ => return,
    };
    *cursor += line_break_len;

    loop {
        while bytes
            .get(*cursor)
            .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
        {
            *cursor += 1;
        }
        if bytes.get(*cursor) != Some(&b'>') {
            break;
        }
        *cursor += 1;
        if bytes
            .get(*cursor)
            .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
        {
            *cursor += 1;
        }
    }

    while bytes
        .get(*cursor)
        .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
    {
        *cursor += 1;
    }
}

fn is_ascii_punctuation(byte: u8) -> bool {
    byte.is_ascii_punctuation()
}

fn decode_scanned_destination(raw: &str, angle_wrapped: bool) -> Option<String> {
    let markdown = if angle_wrapped {
        format!("[probe](<{raw}>)")
    } else {
        format!("[probe]({raw})")
    };
    new_cmark_parser(&markdown, &MarkdownOptions::default()).find_map(|event| match event {
        Event::Start(Tag::Link {
            link_type: LinkType::Inline,
            dest_url,
            ..
        }) => Some(dest_url.into_string()),
        _ => None,
    })
}

fn apply_edits(markdown: &str, mut edits: Vec<DestinationEdit>) -> Result<String, AppDiagnostic> {
    edits.sort_by_key(|edit| edit.range.start);
    if edits
        .windows(2)
        .any(|pair| pair[0].range.end > pair[1].range.start)
    {
        return Err(protocol_error());
    }

    let mut rewritten = markdown.to_owned();
    for edit in edits.into_iter().rev() {
        rewritten.replace_range(edit.range, &edit.replacement);
    }
    Ok(rewritten)
}

fn protocol_error() -> AppDiagnostic {
    AppDiagnostic::from_kind(
        AppDiagnosticKind::Protocol,
        "parser-confirmed Markdown destination span mismatch".to_owned(),
    )
}
