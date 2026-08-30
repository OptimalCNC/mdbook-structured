use std::ops::Range;
use std::path::Path;

use mdbook_markdown::pulldown_cmark::{Event, LinkType, Tag};
use mdbook_markdown::{MarkdownOptions, new_cmark_parser};

use crate::destination::{AuthoredDestination, LocalDestination};
use crate::{
    AppDiagnostic, AppDiagnosticKind, ChapterTarget, LinkResolution, LinkRouteMap,
    LogicalChapterPath,
};

struct DestinationEdit {
    range: Range<usize>,
    replacement: String,
}

struct ScannedDestination {
    range: Range<usize>,
    angle_wrapped: bool,
}

pub fn rewrite_chapter_links(
    current_path: &LogicalChapterPath,
    markdown: &str,
    routes: &LinkRouteMap,
) -> Result<String, AppDiagnostic> {
    let parser = new_cmark_parser(markdown, &MarkdownOptions::default());
    let mut edits = Vec::new();

    for (event, source_range) in parser.into_offset_iter() {
        let Event::Start(Tag::Link {
            link_type: LinkType::Inline,
            dest_url,
            ..
        }) = event
        else {
            continue;
        };

        let scanned = scan_inline_destination(markdown, source_range, dest_url.as_ref())?;
        if let Some(edit) = build_edit(current_path, scanned.range, dest_url.as_ref(), routes)? {
            edits.push(edit);
        }
    }

    apply_edits(markdown, edits)
}

fn build_edit(
    current_path: &LogicalChapterPath,
    range: Range<usize>,
    authored: &str,
    routes: &LinkRouteMap,
) -> Result<Option<DestinationEdit>, AppDiagnostic> {
    let AuthoredDestination::Local(destination) =
        AuthoredDestination::parse(current_path, authored)?
    else {
        return Ok(None);
    };

    let target = match routes.resolve(destination.lookup()) {
        LinkResolution::Exact(target) | LinkResolution::UniqueAlias(target) => target,
        LinkResolution::Ambiguous(_) | LinkResolution::Missing => return Ok(None),
    };
    let replacement = rewritten_destination(current_path, &destination, target)?;

    Ok((replacement != authored).then_some(DestinationEdit { range, replacement }))
}

fn rewritten_destination(
    current_path: &LogicalChapterPath,
    destination: &LocalDestination<'_>,
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

    let lookup_segments: Vec<_> = lookup
        .as_path()
        .iter()
        .map(|segment| segment.to_string_lossy())
        .collect();
    let output_segments: Vec<_> = output_path
        .iter()
        .map(|segment| segment.to_string_lossy())
        .collect();
    let relative_segments: Vec<_> = relative.split('/').collect();
    let leading_parents = relative_segments
        .iter()
        .take_while(|component| **component == "..")
        .count();
    let output_start = common_prefix_len(current_path.as_path().parent(), output_path);
    let mut rewritten = Vec::with_capacity(relative_segments.len());

    rewritten.extend((0..leading_parents).map(|_| "..".to_owned()));
    for (offset, output) in output_segments[output_start..].iter().enumerate() {
        let absolute_index = output_start + offset;
        let authored = raw_segments.get(absolute_index);
        let decoded = lookup_segments.get(absolute_index);
        let replacement = match (authored, decoded) {
            (Some(authored), Some(decoded)) if output.as_ref() == decoded.as_ref() => {
                authored.clone()
            }
            (Some(authored), Some(decoded))
                if output
                    .strip_prefix(decoded.as_ref())
                    .is_some_and(|suffix| suffix == ".html") =>
            {
                format!("{authored}.html")
            }
            _ => output.to_string(),
        };
        rewritten.push(replacement);
    }

    Ok(rewritten.join("/"))
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
    scan_whitespace(bytes, &mut cursor);

    let (range, angle_wrapped) = if bytes.get(cursor) == Some(&b'<') {
        cursor += 1;
        let start = cursor;
        while cursor < bytes.len() {
            match bytes[cursor] {
                b'\n' | b'\r' | b'<' => return None,
                b'>' => break,
                b'\\'
                    if bytes
                        .get(cursor + 1)
                        .is_some_and(|byte| is_ascii_punctuation(*byte)) =>
                {
                    cursor += 2;
                    continue;
                }
                _ => cursor += 1,
            }
        }
        let end = cursor;
        (bytes.get(cursor).is_some_and(|byte| *byte == b'>')).then(|| cursor += 1)?;
        (start..end, true)
    } else {
        let start = cursor;
        let mut nesting = 0usize;
        while cursor < bytes.len() {
            match bytes[cursor] {
                0x00..=0x20 => break,
                b'(' => {
                    nesting += 1;
                    cursor += 1;
                }
                b')' if nesting == 0 => break,
                b')' => {
                    nesting -= 1;
                    cursor += 1;
                }
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
        if nesting != 0 {
            return None;
        }
        (start..cursor, false)
    };

    scan_whitespace(bytes, &mut cursor);
    if matches!(bytes.get(cursor), Some(b'\'' | b'"' | b'(')) {
        cursor = scan_title(bytes, cursor)?;
        scan_whitespace(bytes, &mut cursor);
    }
    (bytes.get(cursor) == Some(&b')') && cursor + 1 == bytes.len()).then_some(ScannedDestination {
        range,
        angle_wrapped,
    })
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

fn scan_whitespace(bytes: &[u8], cursor: &mut usize) {
    while bytes.get(*cursor).is_some_and(u8::is_ascii_whitespace) {
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
