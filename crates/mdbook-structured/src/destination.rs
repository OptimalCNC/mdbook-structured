use std::ops::Range;
use std::path::PathBuf;

use crate::{AppDiagnostic, LogicalChapterPath};

pub(super) enum AuthoredDestination<'a> {
    LeaveUnchanged,
    Local(LocalDestination<'a>),
}

pub(super) struct ParserConfirmedDestination<'a> {
    decoded_path: &'a str,
    raw_authored: &'a str,
    raw_query: Option<&'a str>,
    raw_fragment: Option<&'a str>,
}

pub(super) struct LocalDestination<'a> {
    raw_authored: &'a str,
    normalized_authored_path: NormalizedAuthoredPath,
    raw_query: Option<&'a str>,
    raw_fragment: Option<&'a str>,
    lookup: LogicalChapterPath,
}

struct DecodedToken {
    byte: u8,
    authored_range: Range<usize>,
}

struct InputComponent {
    decoded: String,
    spelling: String,
    decoded_to_spelling_offset: Vec<usize>,
    separator_before: Option<String>,
    source_index: usize,
}

struct NormalizedComponent {
    decoded: String,
    spelling: String,
    decoded_to_spelling_offset: Vec<usize>,
    separator_before: String,
    source_index: Option<usize>,
}

struct NormalizedAuthoredPath {
    spelling: String,
    decoded_to_spelling_offset: Vec<usize>,
}

impl<'a> ParserConfirmedDestination<'a> {
    pub(super) fn new(
        decoded_path: &'a str,
        raw_authored: &'a str,
        raw_query: Option<&'a str>,
        raw_fragment: Option<&'a str>,
    ) -> Self {
        Self {
            decoded_path,
            raw_authored,
            raw_query,
            raw_fragment,
        }
    }

    fn raw_path(&self) -> &'a str {
        let suffix_len = self.raw_query.map_or(0, str::len) + self.raw_fragment.map_or(0, str::len);
        &self.raw_authored[..self.raw_authored.len() - suffix_len]
    }
}

impl<'a> AuthoredDestination<'a> {
    pub(super) fn parse(
        current_path: &LogicalChapterPath,
        authored: ParserConfirmedDestination<'a>,
    ) -> Result<Self, AppDiagnostic> {
        let decoded_path = authored.decoded_path;
        if decoded_path.is_empty()
            || decoded_path.starts_with('/')
            || decoded_path.ends_with(".html")
            || has_uri_scheme(decoded_path)
        {
            return Ok(Self::LeaveUnchanged);
        }

        let raw_path = authored.raw_path();
        let authored_spelling = if raw_path == decoded_path {
            raw_path
        } else {
            decoded_path
        };
        let Some((normalized_authored_path, decoded_lookup)) =
            normalize_relative_path(current_path, authored_spelling)
        else {
            return Ok(Self::LeaveUnchanged);
        };
        let Ok(lookup) = LogicalChapterPath::try_from_path(&decoded_lookup) else {
            return Ok(Self::LeaveUnchanged);
        };

        Ok(Self::Local(LocalDestination {
            raw_authored: authored.raw_authored,
            normalized_authored_path,
            raw_query: authored.raw_query,
            raw_fragment: authored.raw_fragment,
            lookup,
        }))
    }
}

impl LocalDestination<'_> {
    pub(super) fn raw_authored(&self) -> &str {
        self.raw_authored
    }

    pub(super) fn normalized_authored_path(&self) -> &str {
        &self.normalized_authored_path.spelling
    }

    pub(super) fn authored_offset_for_decoded_prefix(
        &self,
        decoded_prefix_len: usize,
    ) -> Option<usize> {
        self.normalized_authored_path
            .decoded_to_spelling_offset
            .get(decoded_prefix_len)
            .copied()
    }

    pub(super) fn raw_query(&self) -> Option<&str> {
        self.raw_query
    }

    pub(super) fn raw_fragment(&self) -> Option<&str> {
        self.raw_fragment
    }

    pub(super) fn lookup(&self) -> &LogicalChapterPath {
        &self.lookup
    }
}

fn has_uri_scheme(path: &str) -> bool {
    let mut bytes = path.bytes();
    if !bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic()) {
        return false;
    }

    for byte in bytes {
        match byte {
            b':' => return true,
            byte if byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.') => {}
            _ => return false,
        }
    }

    false
}

fn normalize_relative_path(
    current_path: &LogicalChapterPath,
    authored_path: &str,
) -> Option<(NormalizedAuthoredPath, PathBuf)> {
    let mut normalized: Vec<_> = current_path
        .as_path()
        .parent()
        .into_iter()
        .flat_map(|parent| parent.iter())
        .map(|component| {
            let component = component.to_string_lossy().into_owned();
            let decoded_to_spelling_offset = (0..=component.len()).collect();
            NormalizedComponent {
                decoded: component.clone(),
                spelling: component,
                decoded_to_spelling_offset,
                separator_before: "/".to_owned(),
                source_index: None,
            }
        })
        .collect();
    if let Some(first) = normalized.first_mut() {
        first.separator_before.clear();
    }

    for component in input_components(authored_path)? {
        match component.decoded.as_str() {
            "" | "." => {}
            ".." if normalized.pop().is_some() => {}
            ".." => return None,
            _ => {
                let preserves_separator = normalized.last().is_some_and(|previous| {
                    previous.source_index == component.source_index.checked_sub(1)
                });
                let separator_before = if normalized.is_empty() {
                    String::new()
                } else if preserves_separator {
                    component
                        .separator_before
                        .clone()
                        .unwrap_or_else(|| "/".to_owned())
                } else {
                    "/".to_owned()
                };
                normalized.push(NormalizedComponent {
                    decoded: component.decoded,
                    spelling: component.spelling,
                    decoded_to_spelling_offset: component.decoded_to_spelling_offset,
                    separator_before,
                    source_index: Some(component.source_index),
                });
            }
        }
    }

    if normalized.is_empty() {
        return None;
    }

    let mut decoded = PathBuf::new();
    let mut authored = String::new();
    let mut decoded_to_authored_offset = vec![0];
    for component in normalized {
        decoded.push(component.decoded);
        authored.push_str(&component.separator_before);
        if !component.separator_before.is_empty() {
            decoded_to_authored_offset.push(authored.len());
        }
        let component_start = authored.len();
        authored.push_str(&component.spelling);
        decoded_to_authored_offset.extend(
            component
                .decoded_to_spelling_offset
                .into_iter()
                .skip(1)
                .map(|offset| component_start + offset),
        );
    }
    Some((
        NormalizedAuthoredPath {
            spelling: authored,
            decoded_to_spelling_offset: decoded_to_authored_offset,
        },
        decoded,
    ))
}

fn input_components(authored_path: &str) -> Option<Vec<InputComponent>> {
    let tokens = percent_decode_with_spans(authored_path)?;

    let mut components = Vec::new();
    let mut component_start = 0;
    let mut separator_before = None;
    let mut source_index = 0;

    for (index, token) in tokens.iter().enumerate() {
        if token.byte != b'/' {
            continue;
        }
        components.push(input_component(
            authored_path,
            &tokens[component_start..index],
            separator_before.take(),
            source_index,
        )?);
        separator_before = Some(authored_path.get(token.authored_range.clone())?.to_owned());
        component_start = index + 1;
        source_index += 1;
    }
    components.push(input_component(
        authored_path,
        &tokens[component_start..],
        separator_before,
        source_index,
    )?);
    Some(components)
}

fn input_component(
    authored_path: &str,
    tokens: &[DecodedToken],
    separator_before: Option<String>,
    source_index: usize,
) -> Option<InputComponent> {
    let decoded = String::from_utf8(tokens.iter().map(|token| token.byte).collect()).ok()?;
    let (spelling, decoded_to_spelling_offset) = match (tokens.first(), tokens.last()) {
        (Some(first), Some(last)) => {
            let spelling_start = first.authored_range.start;
            let spelling = authored_path
                .get(spelling_start..last.authored_range.end)?
                .to_owned();
            let mut offsets = Vec::with_capacity(tokens.len() + 1);
            offsets.push(0);
            offsets.extend(
                tokens
                    .iter()
                    .map(|token| token.authored_range.end - spelling_start),
            );
            (spelling, offsets)
        }
        (None, None) => (String::new(), vec![0]),
        _ => unreachable!("a token slice is either empty or has both endpoints"),
    };
    Some(InputComponent {
        decoded,
        spelling,
        decoded_to_spelling_offset,
        separator_before,
        source_index,
    })
}

fn percent_decode_with_spans(authored_path: &str) -> Option<Vec<DecodedToken>> {
    let bytes = authored_path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = hex_value(*bytes.get(index + 1)?)?;
            let low = hex_value(*bytes.get(index + 2)?)?;
            decoded.push(DecodedToken {
                byte: (high << 4) | low,
                authored_range: index..index + 3,
            });
            index += 3;
            continue;
        }

        let character = authored_path.get(index..)?.chars().next()?;
        let end = index + character.len_utf8();
        decoded.extend(
            authored_path.as_bytes()[index..end]
                .iter()
                .copied()
                .map(|byte| DecodedToken {
                    byte,
                    authored_range: index..end,
                }),
        );
        index = end;
    }

    Some(decoded)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
