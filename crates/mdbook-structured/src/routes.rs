use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

use mdbook_core::book::Book;
use mdbook_structured_core::StructuredFormat;

use crate::diagnostic::{
    AppDiagnostic, AppDiagnosticKind, ChapterDiagnosticFact, MdBookPathHazardFact,
    RouteCollisionGroup,
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ChapterOrdinal(usize);

impl ChapterOrdinal {
    pub(crate) fn from_traversal_index(index: usize) -> Self {
        Self(index)
    }

    pub fn get(self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LogicalChapterPath(PathBuf);

impl LogicalChapterPath {
    pub fn try_from_path(path: &Path) -> Result<Self, AppDiagnostic> {
        let mut normalized = PathBuf::new();

        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::Normal(component) => normalized.push(component),
                Component::ParentDir if normalized.pop() => {}
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(invalid_logical_path(path));
                }
            }
        }

        if normalized.as_os_str().is_empty() {
            return Err(invalid_logical_path(path));
        }

        Ok(Self(normalized))
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct OutputRoute(PathBuf);

impl OutputRoute {
    fn from_projected_path(path: PathBuf) -> Self {
        Self(path)
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }

    fn slash_form(&self) -> String {
        self.0.to_string_lossy().replace('\\', "/")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuredExtension {
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuredSource {
    source_path: PathBuf,
    extension: StructuredExtension,
}

impl StructuredSource {
    fn from_source_path(source_path: &Path) -> Option<Self> {
        Some(Self {
            source_path: source_path.to_owned(),
            extension: StructuredExtension::from_path(source_path)?,
        })
    }

    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    pub fn extension(&self) -> StructuredExtension {
        self.extension
    }

    pub fn format(&self) -> StructuredFormat {
        match self.extension {
            StructuredExtension::Json => StructuredFormat::Json,
            StructuredExtension::Yaml | StructuredExtension::Yml => StructuredFormat::Yaml,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedChapter {
    ordinal: ChapterOrdinal,
    source_path: Option<PathBuf>,
    structured_source: Option<StructuredSource>,
    original_logical_path: LogicalChapterPath,
    transformed_logical_path: LogicalChapterPath,
    output_route: OutputRoute,
}

impl ProjectedChapter {
    pub fn ordinal(&self) -> ChapterOrdinal {
        self.ordinal
    }

    pub fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }

    pub fn structured_source(&self) -> Option<&StructuredSource> {
        self.structured_source.as_ref()
    }

    pub fn original_logical_path(&self) -> &LogicalChapterPath {
        &self.original_logical_path
    }

    pub fn transformed_logical_path(&self) -> &LogicalChapterPath {
        &self.transformed_logical_path
    }

    pub fn output_route(&self) -> &OutputRoute {
        &self.output_route
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderRoutePlan {
    chapters: Vec<ProjectedChapter>,
}

impl RenderRoutePlan {
    pub fn chapters(&self) -> &[ProjectedChapter] {
        &self.chapters
    }

    pub fn chapter(&self, ordinal: ChapterOrdinal) -> Option<&ProjectedChapter> {
        self.chapters.get(ordinal.get())
    }
}

pub fn preflight_render_routes(
    book: &Book,
    source_dir: &Path,
) -> Result<RenderRoutePlan, AppDiagnostic> {
    let mut projected_chapters = Vec::new();
    let mut chapter_facts = Vec::new();

    for (index, chapter) in book.chapters().enumerate() {
        let logical_path = chapter
            .path
            .as_deref()
            .ok_or_else(|| invalid_logical_path(Path::new("")))?;
        let original_logical_path = LogicalChapterPath::try_from_path(logical_path)?;
        let structured_source = chapter
            .source_path
            .as_deref()
            .and_then(StructuredSource::from_source_path);

        let transformed_logical_path = if let Some(source) = &structured_source {
            let mut transformed = original_logical_path.as_path().to_owned();
            transformed.set_extension(format!("{}.md", source.extension.as_str()));
            LogicalChapterPath::try_from_path(&transformed)?
        } else {
            original_logical_path.clone()
        };
        let output_route = OutputRoute::from_projected_path(
            transformed_logical_path.as_path().with_extension("html"),
        );
        let ordinal = ChapterOrdinal::from_traversal_index(index);

        chapter_facts.push(ChapterDiagnosticFact::new(
            chapter.name.clone(),
            chapter
                .source_path
                .as_deref()
                .map(LogicalChapterPath::try_from_path)
                .transpose()?,
            original_logical_path.clone(),
        ));
        projected_chapters.push(ProjectedChapter {
            ordinal,
            source_path: chapter.source_path.clone(),
            structured_source,
            original_logical_path,
            transformed_logical_path,
            output_route,
        });
    }

    reject_route_collisions(&projected_chapters, &chapter_facts)?;
    reject_mdbook_path_hazards(&projected_chapters)?;
    reject_static_source_collisions(&projected_chapters, source_dir)?;

    Ok(RenderRoutePlan {
        chapters: projected_chapters,
    })
}

fn reject_route_collisions(
    projected_chapters: &[ProjectedChapter],
    chapter_facts: &[ChapterDiagnosticFact],
) -> Result<(), AppDiagnostic> {
    let mut participants_by_route = BTreeMap::<OutputRoute, Vec<ChapterOrdinal>>::new();
    for chapter in projected_chapters {
        participants_by_route
            .entry(chapter.output_route.clone())
            .or_default()
            .push(chapter.ordinal);
    }

    let mut groups = Vec::new();
    for (route, participants) in participants_by_route {
        if participants.len() < 2 {
            continue;
        }

        let facts: Vec<_> = participants
            .into_iter()
            .map(|ordinal| chapter_facts[ordinal.get()].clone())
            .collect();
        groups.push(RouteCollisionGroup::new(
            route,
            facts[0].clone(),
            facts[1].clone(),
            facts[2..].to_vec(),
        ));
    }

    if groups.is_empty() {
        return Ok(());
    }

    let first = groups.remove(0);
    Err(AppDiagnostic::from_kind(
        AppDiagnosticKind::RouteCollision {
            first,
            additional: groups,
        },
        "multiple chapters project to the same output route".to_owned(),
    ))
}

fn reject_mdbook_path_hazards(
    projected_chapters: &[ProjectedChapter],
) -> Result<(), AppDiagnostic> {
    for chapter in projected_chapters {
        if chapter.structured_source.is_some() && chapter.output_route.slash_form().contains(".md")
        {
            return Err(AppDiagnostic::from_kind(
                AppDiagnosticKind::MdBookPathHazard {
                    fact: MdBookPathHazardFact::ProjectedRoute(chapter.output_route.clone()),
                },
                "a structured output route contains literal .md".to_owned(),
            ));
        }
    }

    Ok(())
}

fn reject_static_source_collisions(
    projected_chapters: &[ProjectedChapter],
    source_dir: &Path,
) -> Result<(), AppDiagnostic> {
    for chapter in projected_chapters {
        let static_source = source_dir.join(chapter.output_route.as_path());
        match fs::symlink_metadata(&static_source) {
            Ok(metadata)
                if metadata.file_type().is_file()
                    && static_source.extension().and_then(|value| value.to_str()) != Some("md") =>
            {
                return Err(AppDiagnostic::from_kind(
                    AppDiagnosticKind::StaticSourceCollision {
                        output_route: chapter.output_route.clone(),
                        static_source,
                    },
                    "a source file would be copied to a projected chapter route".to_owned(),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                return Err(AppDiagnostic::from_kind(
                    AppDiagnosticKind::Io {
                        path: Some(static_source),
                    },
                    format!("failed to inspect an exact static-source candidate: {error}"),
                ));
            }
        }
    }

    Ok(())
}

fn invalid_logical_path(path: &Path) -> AppDiagnostic {
    AppDiagnostic::from_kind(
        AppDiagnosticKind::Configuration { key: None },
        format!("invalid mdBook-relative chapter path: {}", path.display()),
    )
}
