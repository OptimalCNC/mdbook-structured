mod config;
mod destination;
mod diagnostic;
mod render_book;
mod rewrite_links;
mod routes;

pub use config::{HtmlPreprocessorContext, RenderOptions, RewriteOptions};
pub use diagnostic::{
    AliasCandidateFact, AppDiagnostic, AppDiagnosticCategory, AppDiagnosticKind,
    ChapterDiagnosticFact, MdBookPathHazardFact, RouteCollisionGroup,
};
pub use render_book::{render_book, rewrite_book_links};
pub use rewrite_links::rewrite_chapter_links;
pub use routes::{
    ChapterOrdinal, ChapterTarget, LinkResolution, LinkRouteMap, LogicalChapterPath, OutputRoute,
    ProjectedChapter, RenderRoutePlan, StructuredExtension, StructuredSource,
    preflight_render_routes,
};
