use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::assets::{STRUCTURED_CSS, STRUCTURED_JS};
use crate::{AppDiagnostic, AppDiagnosticKind};

const CSS_FILE: &str = "mdbook-structured.css";
const JAVASCRIPT_FILE: &str = "mdbook-structured.js";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstallState {
    Created,
    Unchanged,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstallReport {
    css: InstallState,
    javascript: InstallState,
}

impl InstallReport {
    pub fn css(&self) -> InstallState {
        self.css
    }

    pub fn javascript(&self) -> InstallState {
        self.javascript
    }
}

pub fn install_assets(destination: &Path) -> Result<InstallReport, AppDiagnostic> {
    let css = install_asset(destination.join(CSS_FILE), STRUCTURED_CSS)?;
    let javascript = install_asset(destination.join(JAVASCRIPT_FILE), STRUCTURED_JS)?;
    Ok(InstallReport { css, javascript })
}

fn install_asset(path: PathBuf, canonical_bytes: &[u8]) -> Result<InstallState, AppDiagnostic> {
    match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(mut file) => {
            file.write_all(canonical_bytes)
                .map_err(|error| io_error(path.clone(), "write", error))?;
            Ok(InstallState::Created)
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            let existing =
                fs::read(&path).map_err(|error| io_error(path.clone(), "read", error))?;
            if existing == canonical_bytes {
                Ok(InstallState::Unchanged)
            } else {
                Err(AppDiagnostic::from_kind(
                    AppDiagnosticKind::InstallConflict { path: path.clone() },
                    format!(
                        "refusing to replace an existing asset with different bytes: {}",
                        path.display()
                    ),
                ))
            }
        }
        Err(error) => Err(io_error(path, "create", error)),
    }
}

fn io_error(path: PathBuf, operation: &str, error: io::Error) -> AppDiagnostic {
    AppDiagnostic::from_kind(
        AppDiagnosticKind::Io {
            path: Some(path.clone()),
        },
        format!(
            "failed to {operation} structured asset {}: {error}",
            path.display()
        ),
    )
}
