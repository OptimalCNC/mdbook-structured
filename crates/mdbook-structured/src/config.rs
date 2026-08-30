use std::path::{Path, PathBuf};

use mdbook_preprocessor::PreprocessorContext;
use mdbook_structured_core::{HtmlRenderOptions, Limits};
use serde::{Deserialize, de::DeserializeOwned};

use crate::{AppDiagnostic, AppDiagnosticKind};

pub struct HtmlPreprocessorContext {
    context: PreprocessorContext,
    source_dir: PathBuf,
}

impl HtmlPreprocessorContext {
    pub fn try_from_context(context: PreprocessorContext) -> Result<Self, AppDiagnostic> {
        if context.renderer != "html" {
            return Err(AppDiagnostic::from_kind(
                AppDiagnosticKind::UnsupportedRenderer {
                    renderer: context.renderer,
                },
                "mdbook-structured supports only the html renderer".to_owned(),
            ));
        }

        let source_dir = context.root.join(&context.config.book.src);
        Ok(Self {
            context,
            source_dir,
        })
    }

    pub(crate) fn source_dir(&self) -> &Path {
        &self.source_dir
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderOptions {
    limits: Limits,
    html: HtmlRenderOptions,
}

impl RenderOptions {
    pub fn from_context(context: &HtmlPreprocessorContext) -> Result<Self, AppDiagnostic> {
        let raw: RawRenderOptions =
            plugin_options_table(context, "structured")?.deserialize("structured")?;
        let max_input_bytes =
            usize_from_config(raw.max_input_bytes.unwrap_or(1_048_576), "max-input-bytes")?;
        let max_nodes = usize_from_config(raw.max_nodes.unwrap_or(10_000), "max-nodes")?;
        let max_depth = usize_from_config(raw.max_depth.unwrap_or(64), "max-depth")?;
        let large_container_threshold = usize_from_config(
            raw.large_container_threshold.unwrap_or(100),
            "large-container-threshold",
        )?;
        let limits = Limits::try_new(max_input_bytes, max_nodes, max_depth).map_err(|error| {
            configuration_error(
                "structured",
                format!("invalid configured limits: {error:?}"),
            )
        })?;

        Ok(Self {
            limits,
            html: HtmlRenderOptions::new(large_container_threshold),
        })
    }

    pub fn limits(&self) -> Limits {
        self.limits
    }

    pub fn html(&self) -> HtmlRenderOptions {
        self.html
    }
}

struct PluginOptionsTable(toml::Table);

fn plugin_options_table(
    context: &HtmlPreprocessorContext,
    registration: &'static str,
) -> Result<PluginOptionsTable, AppDiagnostic> {
    let key = format!("preprocessor.{registration}");
    let mut table = context
        .context
        .config
        .get::<toml::Table>(&key)
        .map_err(|error| configuration_error(&key, error.to_string()))?
        .unwrap_or_default();
    for key in ["command", "after", "before", "renderers", "optional"] {
        table.remove(key);
    }
    Ok(PluginOptionsTable(table))
}

impl PluginOptionsTable {
    fn deserialize<T: DeserializeOwned>(
        self,
        registration: &'static str,
    ) -> Result<T, AppDiagnostic> {
        T::deserialize(toml::Value::Table(self.0))
            .map_err(|error| configuration_error(registration, error.to_string()))
    }
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct RawRenderOptions {
    max_input_bytes: Option<u64>,
    max_nodes: Option<u64>,
    max_depth: Option<u64>,
    large_container_threshold: Option<u64>,
}

fn usize_from_config(value: u64, name: &'static str) -> Result<usize, AppDiagnostic> {
    usize::try_from(value)
        .map_err(|error| configuration_error("structured", format!("{name}: {error}")))
}

fn configuration_error(key: &str, detail: String) -> AppDiagnostic {
    AppDiagnostic::from_kind(
        AppDiagnosticKind::Configuration {
            key: Some(key.to_owned()),
        },
        format!("invalid configuration for {key}: {detail}"),
    )
}
