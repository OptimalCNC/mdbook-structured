use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use mdbook_core::book::{Book, BookItem};
use mdbook_structured_core::StructuredFormat;

use crate::diagnostic::{AppDiagnostic, AppDiagnosticKind, RegisteredRouteFact};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ChapterOrdinal(usize);

impl ChapterOrdinal {
    pub(crate) fn from_traversal_index(index: usize) -> Self {
        Self(index)
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct LogicalChapterPath(PathBuf);

impl LogicalChapterPath {
    pub(crate) fn try_from_path(path: &Path) -> Result<Self, ()> {
        let mut normalized = PathBuf::new();

        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::Normal(component) => normalized.push(component),
                Component::ParentDir if normalized.pop() => {}
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => return Err(()),
            }
        }

        if normalized.as_os_str().is_empty() {
            return Err(());
        }

        Ok(Self(normalized))
    }

    pub(crate) fn as_path(&self) -> &Path {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct OutputRoute(PathBuf);

impl OutputRoute {
    fn from_projected_path(path: PathBuf) -> Self {
        Self(path)
    }

    pub(crate) fn as_path(&self) -> &Path {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StructuredExtension {
    Json,
    Yaml,
    Yml,
}

impl StructuredExtension {
    fn from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "json" => Some(Self::Json),
            "yaml" => Some(Self::Yaml),
            "yml" => Some(Self::Yml),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Yml => "yml",
        }
    }

    fn format(self) -> StructuredFormat {
        match self {
            Self::Json => StructuredFormat::Json,
            Self::Yaml | Self::Yml => StructuredFormat::Yaml,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StructuredSource {
    source_path: LogicalChapterPath,
    extension: StructuredExtension,
}

impl StructuredSource {
    fn admit(source_path: &Path) -> Result<Option<Self>, AppDiagnostic> {
        let Some(extension) = StructuredExtension::from_path(source_path) else {
            return Ok(None);
        };

        let source_path = LogicalChapterPath::try_from_path(source_path).map_err(|()| {
            invalid_registered_route(source_path, "invalid registered source identity")
        })?;
        Ok(Some(Self {
            source_path,
            extension,
        }))
    }

    fn source_path(&self) -> &Path {
        self.source_path.as_path()
    }

    fn source_identity(&self) -> &LogicalChapterPath {
        &self.source_path
    }

    fn extension(&self) -> StructuredExtension {
        self.extension
    }

    fn format(&self) -> StructuredFormat {
        self.extension.format()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RegisteredRenderRoute {
    source: StructuredSource,
    transformed_logical_path: LogicalChapterPath,
    output_route: OutputRoute,
}

impl RegisteredRenderRoute {
    pub(crate) fn format(&self) -> StructuredFormat {
        self.source.format()
    }

    pub(crate) fn source_path(&self) -> &Path {
        self.source.source_path()
    }

    pub(crate) fn transformed_logical_path(&self) -> &LogicalChapterPath {
        &self.transformed_logical_path
    }
}

pub(crate) struct RegisteredRenderRoutes {
    by_ordinal: BTreeMap<ChapterOrdinal, RegisteredRenderRoute>,
}

impl RegisteredRenderRoutes {
    pub(crate) fn from_book(book: &Book) -> Result<Self, AppDiagnostic> {
        let mut by_ordinal = BTreeMap::new();
        let mut traversal_index = 0;
        collect_registered_render_routes(&book.items, &mut traversal_index, &mut by_ordinal)?;
        validate_registered_pairs(
            by_ordinal
                .values()
                .map(|route| (route.source.source_identity(), &route.output_route)),
        )?;
        Ok(Self { by_ordinal })
    }

    pub(crate) fn get(&self, ordinal: ChapterOrdinal) -> Option<&RegisteredRenderRoute> {
        self.by_ordinal.get(&ordinal)
    }
}

fn collect_registered_render_routes(
    items: &[BookItem],
    traversal_index: &mut usize,
    by_ordinal: &mut BTreeMap<ChapterOrdinal, RegisteredRenderRoute>,
) -> Result<(), AppDiagnostic> {
    for item in items {
        let BookItem::Chapter(chapter) = item else {
            continue;
        };

        let ordinal = ChapterOrdinal::from_traversal_index(*traversal_index);
        *traversal_index += 1;

        if let Some(source_path) = chapter.source_path.as_deref()
            && let Some(source) = StructuredSource::admit(source_path)?
        {
            let logical_path = chapter.path.as_deref().ok_or_else(|| {
                invalid_registered_route(source_path, "missing registered logical path")
            })?;
            let logical_path = LogicalChapterPath::try_from_path(logical_path).map_err(|()| {
                invalid_registered_route(source_path, "invalid registered logical path")
            })?;
            let mut transformed_path = logical_path.as_path().to_owned();
            transformed_path.set_extension(format!("{}.md", source.extension().as_str()));
            let transformed_logical_path = LogicalChapterPath::try_from_path(&transformed_path)
                .map_err(|()| {
                    invalid_registered_route(source_path, "invalid registered route shim")
                })?;
            let output_route = OutputRoute::from_projected_path(
                transformed_logical_path.as_path().with_extension("html"),
            );
            by_ordinal.insert(
                ordinal,
                RegisteredRenderRoute {
                    source,
                    transformed_logical_path,
                    output_route,
                },
            );
        }

        collect_registered_render_routes(&chapter.sub_items, traversal_index, by_ordinal)?;
    }

    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RegisteredTarget {
    source_path: LogicalChapterPath,
    output_route: OutputRoute,
}

pub(crate) struct StructuredTargetIndex {
    by_source: BTreeMap<LogicalChapterPath, OutputRoute>,
}

impl StructuredTargetIndex {
    pub(crate) fn from_book(book: &Book) -> Result<Self, AppDiagnostic> {
        let mut targets = Vec::new();
        collect_structured_targets(&book.items, &mut targets)?;
        validate_registered_pairs(
            targets
                .iter()
                .map(|target| (&target.source_path, &target.output_route)),
        )?;
        let by_source = targets
            .into_iter()
            .map(|target| (target.source_path, target.output_route))
            .collect();
        Ok(Self { by_source })
    }

    pub(crate) fn output_for_source(
        &self,
        source_path: &LogicalChapterPath,
    ) -> Option<&OutputRoute> {
        self.by_source.get(source_path)
    }
}

fn collect_structured_targets(
    items: &[BookItem],
    targets: &mut Vec<RegisteredTarget>,
) -> Result<(), AppDiagnostic> {
    for item in items {
        let BookItem::Chapter(chapter) = item else {
            continue;
        };

        if let Some(source_path) = chapter.source_path.as_deref()
            && let Some(source) = StructuredSource::admit(source_path)?
        {
            let rendered_path = chapter.path.as_deref().ok_or_else(|| {
                invalid_registered_route(source_path, "missing registered route shim")
            })?;
            let rendered_path = LogicalChapterPath::try_from_path(rendered_path).map_err(|()| {
                invalid_registered_route(source_path, "invalid registered route shim")
            })?;
            if !is_source_extension_shim(&rendered_path, source.extension()) {
                return Err(invalid_registered_route(
                    source_path,
                    "invalid registered route shim",
                ));
            }
            targets.push(RegisteredTarget {
                source_path: source.source_identity().clone(),
                output_route: OutputRoute::from_projected_path(
                    rendered_path.as_path().with_extension("html"),
                ),
            });
        }

        collect_structured_targets(&chapter.sub_items, targets)?;
    }

    Ok(())
}

fn is_source_extension_shim(
    rendered_path: &LogicalChapterPath,
    source_extension: StructuredExtension,
) -> bool {
    rendered_path
        .as_path()
        .extension()
        .and_then(|extension| extension.to_str())
        == Some("md")
        && rendered_path
            .as_path()
            .file_stem()
            .and_then(|stem| Path::new(stem).extension())
            .and_then(|extension| extension.to_str())
            == Some(source_extension.as_str())
}

fn validate_registered_pairs<'a>(
    pairs: impl Iterator<Item = (&'a LogicalChapterPath, &'a OutputRoute)>,
) -> Result<(), AppDiagnostic> {
    let mut facts_by_source = BTreeMap::<LogicalChapterPath, Vec<RegisteredRouteFact>>::new();
    let mut facts_by_route = BTreeMap::<OutputRoute, Vec<RegisteredRouteFact>>::new();

    for (source_path, output_route) in pairs {
        let fact = RegisteredRouteFact::new(source_path.clone(), output_route.clone());
        facts_by_source
            .entry(source_path.clone())
            .or_default()
            .push(fact.clone());
        facts_by_route
            .entry(output_route.clone())
            .or_default()
            .push(fact);
    }

    if let Some(conflict) = facts_by_source
        .into_values()
        .find_map(RegisteredPairConflict::from_facts)
    {
        return Err(pair_conflict(
            conflict,
            |first, second, additional| AppDiagnosticKind::DuplicateRegisteredSource {
                first,
                second,
                additional,
            },
            "multiple registered routes use the same structured source identity",
        ));
    }

    if let Some(conflict) = facts_by_route
        .into_values()
        .find_map(RegisteredPairConflict::from_facts)
    {
        return Err(pair_conflict(
            conflict,
            |first, second, additional| AppDiagnosticKind::RegisteredRouteCollision {
                first,
                second,
                additional,
            },
            "multiple registered structured sources project to the same output route",
        ));
    }

    Ok(())
}

struct RegisteredPairConflict {
    first: RegisteredRouteFact,
    second: RegisteredRouteFact,
    additional: Vec<RegisteredRouteFact>,
}

impl RegisteredPairConflict {
    fn from_facts(facts: Vec<RegisteredRouteFact>) -> Option<Self> {
        let mut facts = facts.into_iter();
        let first = facts.next()?;
        let second = facts.next()?;
        Some(Self {
            first,
            second,
            additional: facts.collect(),
        })
    }
}

fn pair_conflict(
    conflict: RegisteredPairConflict,
    kind: impl FnOnce(
        RegisteredRouteFact,
        RegisteredRouteFact,
        Vec<RegisteredRouteFact>,
    ) -> AppDiagnosticKind,
    detail: &str,
) -> AppDiagnostic {
    let RegisteredPairConflict {
        first,
        second,
        additional,
    } = conflict;
    AppDiagnostic::from_kind(kind(first, second, additional), detail.to_owned())
}

fn invalid_registered_route(source_path: &Path, detail: &str) -> AppDiagnostic {
    AppDiagnostic::from_kind(
        AppDiagnosticKind::RegisteredRoute {
            source_path: source_path.to_owned(),
        },
        format!("{detail}: {}", source_path.display()),
    )
}
