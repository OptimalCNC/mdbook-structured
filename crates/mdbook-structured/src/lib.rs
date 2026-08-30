mod diagnostic;
mod routes;

pub use diagnostic::{
    AliasCandidateFact, AppDiagnostic, AppDiagnosticCategory, AppDiagnosticKind,
    ChapterDiagnosticFact, MdBookPathHazardFact, RouteCollisionGroup,
};
pub use routes::{
    ChapterOrdinal, LogicalChapterPath, OutputRoute, ProjectedChapter, RenderRoutePlan,
    StructuredExtension, StructuredSource, preflight_render_routes,
};
