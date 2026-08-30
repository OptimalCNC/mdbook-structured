use mdbook_core::book::{Book, BookItem, Chapter};
use mdbook_structured_core::{parse_document, render_structured_page};

use crate::{
    AppDiagnostic, AppDiagnosticKind, ChapterOrdinal, HtmlPreprocessorContext, LinkRouteMap,
    LogicalChapterPath, RenderOptions, RewriteOptions, preflight_render_routes,
    rewrite_chapter_links,
};

pub fn render_book(
    context: &HtmlPreprocessorContext,
    mut book: Book,
) -> Result<Book, AppDiagnostic> {
    let options = RenderOptions::from_context(context)?;
    let plan = preflight_render_routes(&book, context.source_dir())?;
    let mut traversal_index = 0;

    try_for_each_chapter_preorder_mut(&mut book.items, &mut |chapter| {
        let ordinal = ChapterOrdinal::from_traversal_index(traversal_index);
        traversal_index += 1;
        let projected = plan.chapter(ordinal).ok_or_else(|| {
            AppDiagnostic::from_kind(
                AppDiagnosticKind::Protocol,
                "render route plan omitted a non-draft chapter".to_owned(),
            )
        })?;
        let Some(structured_source) = projected.structured_source() else {
            return Ok(());
        };

        let document = parse_document(
            structured_source.format(),
            &chapter.content,
            structured_source.source_path(),
            options.limits(),
        )?;
        let rendered = render_structured_page(&chapter.name, &document, options.html());
        chapter.content = rendered.as_str().to_owned();
        chapter.path = Some(projected.transformed_logical_path().as_path().to_owned());
        Ok(())
    })?;

    Ok(book)
}

pub fn rewrite_book_links(_options: RewriteOptions, mut book: Book) -> Result<Book, AppDiagnostic> {
    let routes = LinkRouteMap::from_book(&book)?;

    try_for_each_chapter_preorder_mut(&mut book.items, &mut |chapter| {
        let Some(path) = chapter.path.as_deref() else {
            return Ok(());
        };
        let path = LogicalChapterPath::try_from_path(path)?;
        chapter.content = rewrite_chapter_links(&path, &chapter.content, &routes)?;
        Ok(())
    })?;

    Ok(book)
}

fn try_for_each_chapter_preorder_mut(
    items: &mut [BookItem],
    visit: &mut impl FnMut(&mut Chapter) -> Result<(), AppDiagnostic>,
) -> Result<(), AppDiagnostic> {
    for item in items {
        let BookItem::Chapter(chapter) = item else {
            continue;
        };
        if !chapter.is_draft_chapter() {
            visit(chapter)?;
        }
        try_for_each_chapter_preorder_mut(&mut chapter.sub_items, visit)?;
    }
    Ok(())
}
