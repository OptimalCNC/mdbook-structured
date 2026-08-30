use crate::SourceLocation;
use crate::model::ProvenanceError;

#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(test)]
static SCALAR_VISITS: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
pub(super) fn reset_scalar_visits() {
    SCALAR_VISITS.store(0, Ordering::Relaxed);
}

#[cfg(test)]
pub(super) fn scalar_visits() -> usize {
    SCALAR_VISITS.load(Ordering::Relaxed)
}

pub(super) fn location_from_byte_offset(
    loaded_source: &str,
    byte_offset: usize,
) -> Result<SourceLocation, ProvenanceError> {
    if byte_offset > loaded_source.len() {
        return Err(ProvenanceError::OutOfBounds);
    }
    if !loaded_source.is_char_boundary(byte_offset) {
        return Err(ProvenanceError::NotCharBoundary);
    }

    let mut line = 1;
    let mut column = 1;
    for character in loaded_source[..byte_offset].chars() {
        #[cfg(test)]
        SCALAR_VISITS.fetch_add(1, Ordering::Relaxed);
        if character == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    SourceLocation::try_new(line, column).ok_or(ProvenanceError::OutOfBounds)
}
