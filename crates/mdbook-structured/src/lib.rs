mod destination;
mod diagnostic;
mod rewrite_links;
mod routes;

pub use diagnostic::{
    AliasCandidateFact, AppDiagnostic, AppDiagnosticCategory, AppDiagnosticKind,
    ChapterDiagnosticFact, MdBookPathHazardFact, RouteCollisionGroup,
};
pub use rewrite_links::rewrite_chapter_links;
pub use routes::{
    ChapterOrdinal, ChapterTarget, LinkResolution, LinkRouteMap, LogicalChapterPath, OutputRoute,
    ProjectedChapter, RenderRoutePlan, StructuredExtension, StructuredSource,
    preflight_render_routes,
};
