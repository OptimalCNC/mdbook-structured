use mdbook_core::book::{Book, BookItem, Chapter};
use mdbook_structured_core::{parse_document, render_structured_page};

use crate::{
    AppDiagnostic, ChapterOrdinal, HtmlPreprocessorContext, LogicalChapterPath,
    RegisteredRenderRoutes, RenderOptions, RewriteOptions, StructuredTargetIndex,
    rewrite_chapter_links,
};

pub fn render_book(
    context: &HtmlPreprocessorContext,
    mut book: Book,
) -> Result<Book, AppDiagnostic> {
    let options = RenderOptions::from_context(context)?;
    let routes = RegisteredRenderRoutes::from_book(&book)?;
    let mut traversal_index = 0;

    try_for_each_chapter_preorder_mut(&mut book.items, &mut |chapter| {
        let ordinal = ChapterOrdinal::from_traversal_index(traversal_index);
        traversal_index += 1;
        let Some(route) = routes.get(ordinal) else {
            return Ok(());
        };

        let document = parse_document(
            route.format(),
            &chapter.content,
            route.source_path(),
            options.limits(),
        )?;
        let rendered = render_structured_page(&chapter.name, &document, options.html());
        chapter.content = rendered.as_str().to_owned();
        chapter.path = Some(route.transformed_logical_path().as_path().to_owned());
        Ok(())
    })?;

    Ok(book)
}

pub fn rewrite_book_links(_options: RewriteOptions, mut book: Book) -> Result<Book, AppDiagnostic> {
    let targets = StructuredTargetIndex::from_book(&book)?;

    try_for_each_chapter_preorder_mut(&mut book.items, &mut |chapter| {
        let Some(path) = chapter.path.as_deref() else {
            return Ok(());
        };
        let Ok(path) = LogicalChapterPath::try_from_path(path) else {
            return Ok(());
        };
        chapter.content = rewrite_chapter_links(&path, &chapter.content, &targets)?;
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
        visit(chapter)?;
        try_for_each_chapter_preorder_mut(&mut chapter.sub_items, visit)?;
    }
    Ok(())
}
