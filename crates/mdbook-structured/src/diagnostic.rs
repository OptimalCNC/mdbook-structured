use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};

use mdbook_structured_core::{Diagnostic, DiagnosticCategory};

use crate::routes::{LogicalChapterPath, OutputRoute};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppDiagnosticCategory {
    Core(DiagnosticCategory),
    Configuration,
    Protocol,
    UnsupportedRenderer,
    RegisteredRoute,
    DuplicateRegisteredSource,
    RegisteredRouteCollision,
    MatchedReferencePathHazard,
    InstallConflict,
    Io,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisteredRouteFact {
    source_path: LogicalChapterPath,
    output_route: OutputRoute,
}

impl RegisteredRouteFact {
    pub(crate) fn new(source_path: LogicalChapterPath, output_route: OutputRoute) -> Self {
        Self {
            source_path,
            output_route,
        }
    }

    pub fn source_path(&self) -> &Path {
        self.source_path.as_path()
    }

    pub fn output_route(&self) -> &Path {
        self.output_route.as_path()
    }
}

#[derive(Debug)]
pub enum AppDiagnosticKind {
    Core(Diagnostic),
    Configuration {
        key: Option<String>,
    },
    Protocol,
    UnsupportedRenderer {
        renderer: String,
    },
    RegisteredRoute {
        source_path: PathBuf,
    },
    DuplicateRegisteredSource {
        first: RegisteredRouteFact,
        second: RegisteredRouteFact,
        additional: Vec<RegisteredRouteFact>,
    },
    RegisteredRouteCollision {
        first: RegisteredRouteFact,
        second: RegisteredRouteFact,
        additional: Vec<RegisteredRouteFact>,
    },
    MatchedReferencePathHazard {
        rewritten_destination: String,
    },
    InstallConflict {
        path: PathBuf,
    },
    Io {
        path: Option<PathBuf>,
    },
}

#[derive(Debug)]
pub struct AppDiagnostic {
    kind: Box<AppDiagnosticKind>,
    detail: String,
}

impl AppDiagnostic {
    pub(crate) fn from_kind(kind: AppDiagnosticKind, detail: String) -> Self {
        Self {
            kind: Box::new(kind),
            detail,
        }
    }

    pub fn kind(&self) -> &AppDiagnosticKind {
        self.kind.as_ref()
    }

    pub fn category(&self) -> AppDiagnosticCategory {
        match self.kind.as_ref() {
            AppDiagnosticKind::Core(diagnostic) => {
                AppDiagnosticCategory::Core(diagnostic.category())
            }
            AppDiagnosticKind::Configuration { .. } => AppDiagnosticCategory::Configuration,
            AppDiagnosticKind::Protocol => AppDiagnosticCategory::Protocol,
            AppDiagnosticKind::UnsupportedRenderer { .. } => {
                AppDiagnosticCategory::UnsupportedRenderer
            }
            AppDiagnosticKind::RegisteredRoute { .. } => AppDiagnosticCategory::RegisteredRoute,
            AppDiagnosticKind::DuplicateRegisteredSource { .. } => {
                AppDiagnosticCategory::DuplicateRegisteredSource
            }
            AppDiagnosticKind::RegisteredRouteCollision { .. } => {
                AppDiagnosticCategory::RegisteredRouteCollision
            }
            AppDiagnosticKind::MatchedReferencePathHazard { .. } => {
                AppDiagnosticCategory::MatchedReferencePathHazard
            }
            AppDiagnosticKind::InstallConflict { .. } => AppDiagnosticCategory::InstallConflict,
            AppDiagnosticKind::Io { .. } => AppDiagnosticCategory::Io,
        }
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }

    pub fn core_diagnostic(&self) -> Option<&Diagnostic> {
        match self.kind.as_ref() {
            AppDiagnosticKind::Core(diagnostic) => Some(diagnostic),
            _ => None,
        }
    }
}

impl From<Diagnostic> for AppDiagnostic {
    fn from(diagnostic: Diagnostic) -> Self {
        let detail = diagnostic.detail().to_owned();
        Self::from_kind(AppDiagnosticKind::Core(diagnostic), detail)
    }
}

impl Display for AppDiagnostic {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.kind, self.detail)
    }
}

impl std::error::Error for AppDiagnostic {}
