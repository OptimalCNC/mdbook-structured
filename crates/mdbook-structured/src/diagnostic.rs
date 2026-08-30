use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;

use mdbook_structured_core::{Diagnostic, DiagnosticCategory};

use crate::routes::{LogicalChapterPath, OutputRoute};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppDiagnosticCategory {
    Core(DiagnosticCategory),
    Configuration,
    Protocol,
    UnsupportedRenderer,
    RouteCollision,
    StaticSourceCollision,
    MdBookPathHazard,
    AmbiguousAlias,
    MixedReferenceUse,
    InstallConflict,
    Io,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChapterDiagnosticFact {
    chapter_name: String,
    source_path: Option<LogicalChapterPath>,
    logical_path: LogicalChapterPath,
}

impl ChapterDiagnosticFact {
    pub(crate) fn new(
        chapter_name: String,
        source_path: Option<LogicalChapterPath>,
        logical_path: LogicalChapterPath,
    ) -> Self {
        Self {
            chapter_name,
            source_path,
            logical_path,
        }
    }

    pub fn chapter_name(&self) -> &str {
        &self.chapter_name
    }

    pub fn source_path(&self) -> Option<&LogicalChapterPath> {
        self.source_path.as_ref()
    }

    pub fn logical_path(&self) -> &LogicalChapterPath {
        &self.logical_path
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteCollisionGroup {
    output_route: OutputRoute,
    first_chapter: ChapterDiagnosticFact,
    second_chapter: ChapterDiagnosticFact,
    additional_chapters: Vec<ChapterDiagnosticFact>,
}

impl RouteCollisionGroup {
    pub(crate) fn new(
        output_route: OutputRoute,
        first_chapter: ChapterDiagnosticFact,
        second_chapter: ChapterDiagnosticFact,
        additional_chapters: Vec<ChapterDiagnosticFact>,
    ) -> Self {
        Self {
            output_route,
            first_chapter,
            second_chapter,
            additional_chapters,
        }
    }

    pub fn output_route(&self) -> &OutputRoute {
        &self.output_route
    }

    pub fn first_chapter(&self) -> &ChapterDiagnosticFact {
        &self.first_chapter
    }

    pub fn second_chapter(&self) -> &ChapterDiagnosticFact {
        &self.second_chapter
    }

    pub fn additional_chapters(&self) -> &[ChapterDiagnosticFact] {
        &self.additional_chapters
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AliasCandidateFact {
    source_path: LogicalChapterPath,
    output_route: OutputRoute,
}

impl AliasCandidateFact {
    #[allow(dead_code)]
    pub(crate) fn new(source_path: LogicalChapterPath, output_route: OutputRoute) -> Self {
        Self {
            source_path,
            output_route,
        }
    }

    pub fn source_path(&self) -> &LogicalChapterPath {
        &self.source_path
    }

    pub fn output_route(&self) -> &OutputRoute {
        &self.output_route
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MdBookPathHazardFact {
    ProjectedRoute(OutputRoute),
    RewrittenDestination(String),
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
    RouteCollision {
        first: RouteCollisionGroup,
        additional: Vec<RouteCollisionGroup>,
    },
    StaticSourceCollision {
        output_route: OutputRoute,
        static_source: PathBuf,
    },
    MdBookPathHazard {
        fact: MdBookPathHazardFact,
    },
    AmbiguousAlias {
        authored_path: LogicalChapterPath,
        first: AliasCandidateFact,
        second: AliasCandidateFact,
        additional: Vec<AliasCandidateFact>,
    },
    MixedReferenceUse {
        reference_label: String,
        destination: String,
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
            AppDiagnosticKind::RouteCollision { .. } => AppDiagnosticCategory::RouteCollision,
            AppDiagnosticKind::StaticSourceCollision { .. } => {
                AppDiagnosticCategory::StaticSourceCollision
            }
            AppDiagnosticKind::MdBookPathHazard { .. } => AppDiagnosticCategory::MdBookPathHazard,
            AppDiagnosticKind::AmbiguousAlias { .. } => AppDiagnosticCategory::AmbiguousAlias,
            AppDiagnosticKind::MixedReferenceUse { .. } => AppDiagnosticCategory::MixedReferenceUse,
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
