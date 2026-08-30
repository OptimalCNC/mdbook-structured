use std::path::PathBuf;

use crate::{AppDiagnostic, LogicalChapterPath};

pub(super) enum AuthoredDestination<'a> {
    LeaveUnchanged,
    Local(LocalDestination<'a>),
}

pub(super) struct LocalDestination<'a> {
    raw_path: &'a str,
    raw_query: Option<&'a str>,
    raw_fragment: Option<&'a str>,
    lookup: LogicalChapterPath,
}

impl<'a> AuthoredDestination<'a> {
    pub(super) fn parse(
        current_path: &LogicalChapterPath,
        authored: &'a str,
    ) -> Result<Self, AppDiagnostic> {
        let fragment_start = authored.find('#').unwrap_or(authored.len());
        let before_fragment = &authored[..fragment_start];
        let query_start = before_fragment.find('?').unwrap_or(before_fragment.len());
        let raw_path = &before_fragment[..query_start];
        let raw_query =
            (query_start < before_fragment.len()).then_some(&authored[query_start..fragment_start]);
        let raw_fragment = (fragment_start < authored.len()).then_some(&authored[fragment_start..]);

        if raw_path.is_empty()
            || raw_path.starts_with('/')
            || raw_path.ends_with(".html")
            || has_uri_scheme(raw_path)
        {
            return Ok(Self::LeaveUnchanged);
        }

        let Some(normalized) = normalize_relative_path(current_path, raw_path) else {
            return Ok(Self::LeaveUnchanged);
        };
        let Some(decoded) = percent_decode_utf8(&normalized) else {
            return Ok(Self::LeaveUnchanged);
        };
        let Ok(lookup) = LogicalChapterPath::try_from_path(decoded.as_ref()) else {
            return Ok(Self::LeaveUnchanged);
        };

        Ok(Self::Local(LocalDestination {
            raw_path,
            raw_query,
            raw_fragment,
            lookup,
        }))
    }
}

impl LocalDestination<'_> {
    pub(super) fn raw_path(&self) -> &str {
        self.raw_path
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
) -> Option<PathBuf> {
    let mut segments: Vec<_> = current_path
        .as_path()
        .parent()
        .into_iter()
        .flat_map(|parent| parent.iter())
        .map(|segment| segment.to_string_lossy().into_owned())
        .collect();

    for segment in authored_path.split('/') {
        match segment {
            "" | "." => {}
            ".." if segments.pop().is_some() => {}
            ".." => return None,
            segment => segments.push(segment.to_owned()),
        }
    }

    if segments.is_empty() {
        return None;
    }

    Some(segments.into_iter().collect())
}

fn percent_decode_utf8(path: &std::path::Path) -> Option<PathBuf> {
    let authored = path.to_str()?;
    let bytes = authored.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] != b'%' {
            decoded.push(bytes[index]);
            index += 1;
            continue;
        }

        let high = hex_value(*bytes.get(index + 1)?)?;
        let low = hex_value(*bytes.get(index + 2)?)?;
        decoded.push((high << 4) | low);
        index += 3;
    }

    Some(PathBuf::from(String::from_utf8(decoded).ok()?))
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
