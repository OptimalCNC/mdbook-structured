use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::path::Path;

use mdbook_markdown::pulldown_cmark::{Event, LinkType, Tag};
use mdbook_markdown::{MarkdownOptions, new_cmark_parser};

use crate::destination::{AuthoredDestination, LocalDestination, ParserConfirmedDestination};
use crate::{
    AppDiagnostic, AppDiagnosticKind, LogicalChapterPath, OutputRoute, StructuredTargetIndex,
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

struct ReferenceDefinition {
    destination: String,
    span: Range<usize>,
}

pub(crate) fn rewrite_chapter_links(
    current_path: &LogicalChapterPath,
    markdown: &str,
    targets: &StructuredTargetIndex,
) -> Result<String, AppDiagnostic> {
    let definition_parser = new_cmark_parser(markdown, &MarkdownOptions::default());
    let reference_definitions = definition_parser.reference_definitions().clone();
    let definitions_by_span: BTreeMap<_, _> = reference_definitions
        .iter()
        .map(|(_, definition)| {
            (
                definition_key(&definition.span),
                ReferenceDefinition {
                    destination: definition.dest.to_string(),
                    span: definition.span.clone(),
                },
            )
        })
        .collect();
    let mut linked_definitions = BTreeSet::<DefinitionKey>::new();
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
                let authored = parser_confirmed_destination(
                    raw_destination,
                    dest_url.as_ref(),
                    scanned.angle_wrapped,
                )?;
                if let Some(edit) = build_edit(current_path, scanned.range, authored, targets)? {
                    edits.push(edit);
                }
            }
            Event::Start(Tag::Link { link_type, id, .. }) if is_reference_link_type(link_type) => {
                record_linked_definition(
                    &reference_definitions,
                    id.as_ref(),
                    &mut linked_definitions,
                )?;
            }
            _ => {}
        }
    }

    for key in linked_definitions {
        let definition = definitions_by_span.get(&key).ok_or_else(protocol_error)?;
        let scanned =
            scan_reference_destination(markdown, definition.span.clone(), &definition.destination)?;
        let raw_destination = markdown
            .get(scanned.range.clone())
            .ok_or_else(protocol_error)?;
        let authored = parser_confirmed_destination(
            raw_destination,
            &definition.destination,
            scanned.angle_wrapped,
        )?;
        if let Some(edit) = build_edit(current_path, scanned.range, authored, targets)? {
            edits.push(edit);
        }
    }

    apply_edits(markdown, edits)
}

fn is_reference_link_type(link_type: LinkType) -> bool {
    matches!(
        link_type,
        LinkType::Reference | LinkType::Collapsed | LinkType::Shortcut
    )
}

fn record_linked_definition(
    definitions: &mdbook_markdown::pulldown_cmark::RefDefs<'_>,
    id: &str,
    linked_definitions: &mut BTreeSet<DefinitionKey>,
) -> Result<(), AppDiagnostic> {
    let definition = definitions.get(id).ok_or_else(protocol_error)?;
    linked_definitions.insert(definition_key(&definition.span));
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
    authored: ParserConfirmedDestination<'_>,
    targets: &StructuredTargetIndex,
) -> Result<Option<DestinationEdit>, AppDiagnostic> {
    let AuthoredDestination::Local(destination) =
        AuthoredDestination::parse(current_path, authored)?
    else {
        return Ok(None);
    };

    let Some(output_route) = targets.output_for_source(destination.lookup()) else {
        return Ok(None);
    };
    let replacement = rewritten_destination(current_path, &destination, output_route)?;
    if replacement == destination.raw_authored() {
        return Ok(None);
    }
    if replacement.contains(".md") {
        return Err(AppDiagnostic::from_kind(
            AppDiagnosticKind::MatchedReferencePathHazard {
                rewritten_destination: replacement,
            },
            "a rewritten Markdown destination contains literal .md".to_owned(),
        ));
    }

    Ok(Some(DestinationEdit { range, replacement }))
}

fn rewritten_destination(
    current_path: &LogicalChapterPath,
    destination: &LocalDestination<'_>,
    output_route: &OutputRoute,
) -> Result<String, AppDiagnostic> {
    let current_parent = current_path
        .as_path()
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let output_path = output_route.as_path();
    let relative = relative_path(current_parent, output_path);
    let mut rewritten =
        preserve_authored_path_spelling(current_path, destination, output_path, &relative)?;
    if let Some(query) = destination.raw_query() {
        rewritten.push_str(query);
    }
    if let Some(fragment) = destination.raw_fragment() {
        rewritten.push_str(fragment);
    }
    Ok(rewritten)
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

fn preserve_authored_path_spelling(
    current_path: &LogicalChapterPath,
    destination: &LocalDestination<'_>,
    output_path: &Path,
    relative: &str,
) -> Result<String, AppDiagnostic> {
    let lookup_slash = slash_form(destination.lookup().as_path());
    let output_slash = slash_form(output_path);
    let preserved_prefix_len = common_utf8_prefix_len(&lookup_slash, &output_slash);
    let preserved_authored_end = destination
        .authored_offset_for_decoded_prefix(preserved_prefix_len)
        .ok_or_else(protocol_error)?;

    let relative_segments: Vec<_> = relative.split('/').collect();
    let leading_parents = relative_segments
        .iter()
        .take_while(|component| **component == "..")
        .count();
    let output_start = common_prefix_len(current_path.as_path().parent(), output_path);
    let relative_output_start = decoded_component_prefix_len(output_path, output_start);
    let encoded_suffix = if relative_output_start <= preserved_prefix_len {
        let authored_output_start = destination
            .authored_offset_for_decoded_prefix(relative_output_start)
            .ok_or_else(protocol_error)?;
        format!(
            "{}{}",
            &destination.normalized_authored_path()[authored_output_start..preserved_authored_end],
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

fn decoded_component_prefix_len(path: &Path, component_count: usize) -> usize {
    path.iter()
        .take(component_count)
        .map(|component| component.to_string_lossy().len() + 1)
        .sum()
}

fn common_prefix_len(left: Option<&Path>, right: &Path) -> usize {
    left.into_iter()
        .flat_map(|path| path.iter())
        .zip(right.iter())
        .take_while(|(left, right)| left == right)
        .count()
}

fn parser_confirmed_destination<'a>(
    raw_authored: &'a str,
    decoded_authored: &'a str,
    angle_wrapped: bool,
) -> Result<ParserConfirmedDestination<'a>, AppDiagnostic> {
    let fragment_start = decoded_authored.find('#').unwrap_or(decoded_authored.len());
    let query_start = decoded_authored[..fragment_start]
        .find('?')
        .unwrap_or(fragment_start);
    let has_query = query_start < fragment_start;
    let has_fragment = fragment_start < decoded_authored.len();

    let raw_path_end =
        raw_offset_for_decoded_boundary(raw_authored, decoded_authored, query_start, angle_wrapped)
            .ok_or_else(protocol_error)?;
    let raw_fragment_start = if has_fragment {
        raw_offset_for_decoded_boundary(
            raw_authored,
            decoded_authored,
            fragment_start,
            angle_wrapped,
        )
        .ok_or_else(protocol_error)?
    } else {
        raw_authored.len()
    };
    if raw_path_end > raw_fragment_start {
        return Err(protocol_error());
    }

    let raw_query = if has_query {
        Some(
            raw_authored
                .get(raw_path_end..raw_fragment_start)
                .ok_or_else(protocol_error)?,
        )
    } else {
        None
    };
    let raw_fragment = if has_fragment {
        Some(
            raw_authored
                .get(raw_fragment_start..)
                .ok_or_else(protocol_error)?,
        )
    } else {
        None
    };

    Ok(ParserConfirmedDestination::new(
        &decoded_authored[..query_start],
        raw_authored,
        raw_query,
        raw_fragment,
    ))
}

fn raw_offset_for_decoded_boundary(
    raw_authored: &str,
    decoded_authored: &str,
    decoded_boundary: usize,
    angle_wrapped: bool,
) -> Option<usize> {
    if decoded_boundary == 0 {
        return Some(0);
    }
    if decoded_boundary == decoded_authored.len() {
        return Some(raw_authored.len());
    }
    let decoded_prefix = decoded_authored.get(..decoded_boundary)?;
    let decoded_suffix = decoded_authored.get(decoded_boundary..)?;
    let mut matches = raw_authored
        .char_indices()
        .map(|(offset, _)| offset)
        .chain(std::iter::once(raw_authored.len()))
        .filter(|&offset| {
            offset >= decoded_prefix.len() && raw_authored.len() - offset >= decoded_suffix.len()
        })
        .filter(|&offset| {
            decode_scanned_destination(&raw_authored[..offset], angle_wrapped).as_deref()
                == Some(decoded_prefix)
                && decode_scanned_destination(&raw_authored[offset..], angle_wrapped).as_deref()
                    == Some(decoded_suffix)
        });
    let matched = matches.next()?;
    matches.next().is_none().then_some(matched)
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
