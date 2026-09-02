//! An external mdBook preprocessor that renders structured JSON and YAML
//! chapters through render and rewrite-links phases, with an install command
//! and support for the stock HTML renderer.

mod assets;
mod cli;
mod config;
mod destination;
mod diagnostic;
mod install;
mod protocol;
mod render_book;
mod rewrite_links;
mod routes;

pub use cli::run_with_io;
pub use config::{HtmlPreprocessorContext, RenderOptions, RewriteOptions};
pub use diagnostic::{
    AppDiagnostic, AppDiagnosticCategory, AppDiagnosticKind, RegisteredRouteFact,
};
pub use install::{InstallReport, InstallState, install_assets};
pub use render_book::{render_book, rewrite_book_links};
pub(crate) use rewrite_links::rewrite_chapter_links;
pub(crate) use routes::{
    ChapterOrdinal, LogicalChapterPath, OutputRoute, RegisteredRenderRoutes, StructuredTargetIndex,
};
