use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::path::Path;

use rlsp_yaml_parser::{
    Event, LoadError, LoaderBuilder, Node as YamlNode, ResolvedTag, Span as YamlSpan, parse_events,
};

use crate::limits::Budget;
use crate::model::ProvenanceError;
use crate::{
    Diagnostic, DiagnosticCategory, Limits, MappingEntry, Node, NodeValue, NumberLexeme,
    SourceLocation, SourceSpan, StructuredDocument, StructuredFormat, StructuredPath,
};

use super::FormatAdapter;
use super::coordinates::location_from_byte_offset;

pub(super) struct YamlAdapter;

enum ExpectedSlot {
    Root,
    SequenceItem {
        parent_depth: NonZeroUsize,
        index: usize,
    },
    MappingKey {
        parent_depth: NonZeroUsize,
    },
    MappingValue {
        parent_depth: NonZeroUsize,
        decoded_key: String,
    },
}

enum ContainerFrame {
    Sequence {
        depth: NonZeroUsize,
        next_index: usize,
    },
    Mapping {
        depth: NonZeroUsize,
        pending_decoded_key: Option<String>,
    },
}

struct YamlPreflight {
    budget: Budget,
    stack: Vec<ContainerFrame>,
    container_paths: Vec<StructuredPath>,
    next_slot: ExpectedSlot,
    document_count: usize,
}

impl FormatAdapter for YamlAdapter {
    fn parse(
        &self,
        loaded_source: &str,
        source_name: &Path,
        limits: Limits,
    ) -> Result<StructuredDocument, Diagnostic> {
        let preflight = YamlPreflight::run(loaded_source, source_name, limits)?;
        let documents = LoaderBuilder::new()
            .lossless()
            .build()
            .load(loaded_source)
            .map_err(|error| loader_error(error, loaded_source, source_name))?;

        if documents.len() != 1 {
            return Err(lossless_projection(
                source_name,
                None,
                None,
                format!(
                    "expected exactly one YAML document, loaded {}",
                    documents.len()
                ),
            ));
        }

        let document = documents
            .into_iter()
            .next()
            .expect("the document count was checked");
        if document.version.is_some() || !document.tags.is_empty() {
            return Err(lossless_projection(
                source_name,
                None,
                None,
                "YAML directive metadata has no lossless public projection".to_owned(),
            ));
        }
        let root = project_node(
            &document.root,
            loaded_source,
            source_name,
            &StructuredPath::root(),
        )?;

        Ok(StructuredDocument::from_parser_verified(
            StructuredFormat::Yaml,
            loaded_source.to_owned(),
            root,
            preflight.finish(),
        ))
    }
}

impl YamlPreflight {
    fn run(loaded_source: &str, source_name: &Path, limits: Limits) -> Result<Budget, Diagnostic> {
        let mut preflight = Self {
            budget: Budget::new(limits),
            stack: Vec::new(),
            container_paths: Vec::new(),
            next_slot: ExpectedSlot::Root,
            document_count: 0,
        };

        for result in parse_events(loaded_source) {
            let (event, span) = result.map_err(|error| {
                parser_error(
                    loaded_source,
                    source_name,
                    error.pos.byte_offset,
                    error.to_string(),
                )
            })?;
            preflight.accept(event, span, loaded_source, source_name)?;
        }

        Ok(preflight.budget)
    }

    fn accept(
        &mut self,
        event: Event<'_>,
        span: YamlSpan,
        loaded_source: &str,
        source_name: &Path,
    ) -> Result<(), Diagnostic> {
        match event {
            Event::StreamStart | Event::StreamEnd | Event::Comment { .. } => Ok(()),
            Event::DocumentStart {
                version,
                tag_directives,
                ..
            } => {
                self.document_count = self.document_count.saturating_add(1);
                if self.document_count > 1 {
                    return Err(lossless_at_span(
                        loaded_source,
                        source_name,
                        span,
                        None,
                        "multiple YAML documents have no public projection".to_owned(),
                    ));
                }
                if version.is_some() || !tag_directives.is_empty() {
                    return Err(lossless_at_span(
                        loaded_source,
                        source_name,
                        span,
                        None,
                        "YAML directive metadata has no lossless public projection".to_owned(),
                    ));
                }
                Ok(())
            }
            Event::DocumentEnd { .. } => Ok(()),
            Event::Alias { .. } => {
                let path = self.path_for_next_slot();
                Err(lossless_at_span(
                    loaded_source,
                    source_name,
                    span,
                    Some(path),
                    "YAML aliases have no lossless public projection".to_owned(),
                ))
            }
            Event::Scalar { value, meta, .. } => {
                let metadata_span = meta
                    .as_ref()
                    .and_then(|metadata| metadata.anchor_loc.or(metadata.tag_loc));
                if let Some(metadata_span) = metadata_span {
                    let path = match self.next_slot {
                        ExpectedSlot::MappingKey { .. } => self
                            .current_container_path()
                            .clone()
                            .with_key(value.as_ref()),
                        _ => self.path_for_next_slot(),
                    };
                    return Err(lossless_at_span(
                        loaded_source,
                        source_name,
                        metadata_span,
                        Some(path),
                        "YAML anchors or authored tags have no lossless public projection"
                            .to_owned(),
                    ));
                }

                if matches!(self.next_slot, ExpectedSlot::MappingKey { .. }) {
                    self.accept_mapping_key(value.into_owned());
                    Ok(())
                } else {
                    let _ = self.accept_node(span, loaded_source, source_name)?;
                    Ok(())
                }
            }
            Event::SequenceStart { meta, .. } => {
                self.reject_metadata(meta.as_deref(), span, loaded_source, source_name)?;
                let (depth, path) = self.accept_node(span, loaded_source, source_name)?;
                self.stack.push(ContainerFrame::Sequence {
                    depth,
                    next_index: 0,
                });
                self.container_paths.push(path);
                self.next_slot = ExpectedSlot::SequenceItem {
                    parent_depth: depth,
                    index: 0,
                };
                Ok(())
            }
            Event::MappingStart { meta, .. } => {
                self.reject_metadata(meta.as_deref(), span, loaded_source, source_name)?;
                if matches!(self.next_slot, ExpectedSlot::MappingKey { .. }) {
                    return Err(lossless_at_span(
                        loaded_source,
                        source_name,
                        span,
                        Some(self.current_container_path().clone()),
                        "collection mapping keys have no lossless public projection".to_owned(),
                    ));
                }
                let (depth, path) = self.accept_node(span, loaded_source, source_name)?;
                self.stack.push(ContainerFrame::Mapping {
                    depth,
                    pending_decoded_key: None,
                });
                self.container_paths.push(path);
                self.next_slot = ExpectedSlot::MappingKey {
                    parent_depth: depth,
                };
                Ok(())
            }
            Event::SequenceEnd => self.close_container(false, loaded_source, source_name, span),
            Event::MappingEnd => self.close_container(true, loaded_source, source_name, span),
        }
    }

    fn reject_metadata(
        &self,
        meta: Option<&rlsp_yaml_parser::EventMeta<'_>>,
        event_span: YamlSpan,
        loaded_source: &str,
        source_name: &Path,
    ) -> Result<(), Diagnostic> {
        let Some(meta) = meta else {
            return Ok(());
        };
        let Some(metadata_span) = meta.anchor_loc.or(meta.tag_loc) else {
            return Ok(());
        };
        Err(lossless_at_span(
            loaded_source,
            source_name,
            metadata_span,
            Some(self.path_for_next_slot()),
            format!(
                "YAML anchors or authored tags on the node at bytes {}..{} have no lossless public projection",
                event_span.start, event_span.end
            ),
        ))
    }

    fn accept_mapping_key(&mut self, decoded_key: String) {
        let parent_depth = match self.next_slot {
            ExpectedSlot::MappingKey { parent_depth } => parent_depth,
            _ => unreachable!("mapping-key handling is called only in key position"),
        };
        let Some(ContainerFrame::Mapping {
            pending_decoded_key,
            ..
        }) = self.stack.last_mut()
        else {
            unreachable!("a mapping key has a mapping frame");
        };
        *pending_decoded_key = Some(decoded_key.clone());
        self.next_slot = ExpectedSlot::MappingValue {
            parent_depth,
            decoded_key,
        };
    }

    fn accept_node(
        &mut self,
        span: YamlSpan,
        loaded_source: &str,
        source_name: &Path,
    ) -> Result<(NonZeroUsize, StructuredPath), Diagnostic> {
        let slot = std::mem::replace(&mut self.next_slot, ExpectedSlot::Root);
        let (depth, path) = match slot {
            ExpectedSlot::Root => (NonZeroUsize::MIN, StructuredPath::root()),
            ExpectedSlot::SequenceItem {
                parent_depth,
                index,
            } => {
                let depth = next_depth(parent_depth, loaded_source, source_name, span)?;
                let path = self.current_container_path().clone().with_index(index);
                let Some(ContainerFrame::Sequence { next_index, .. }) = self.stack.last_mut()
                else {
                    unreachable!("a sequence item has a sequence frame");
                };
                *next_index = next_index.saturating_add(1);
                (depth, path)
            }
            ExpectedSlot::MappingValue {
                parent_depth,
                decoded_key,
            } => {
                let depth = next_depth(parent_depth, loaded_source, source_name, span)?;
                let path = self.current_container_path().clone().with_key(decoded_key);
                let Some(ContainerFrame::Mapping {
                    pending_decoded_key,
                    ..
                }) = self.stack.last_mut()
                else {
                    unreachable!("a mapping value has a mapping frame");
                };
                *pending_decoded_key = None;
                (depth, path)
            }
            ExpectedSlot::MappingKey { .. } => {
                return Err(lossless_at_span(
                    loaded_source,
                    source_name,
                    span,
                    Some(self.current_container_path().clone()),
                    "collection mapping keys have no lossless public projection".to_owned(),
                ));
            }
        };

        let location = self
            .budget
            .requires_diagnostic_location(depth)
            .then(|| location_from_byte_offset(loaded_source, span.start as usize))
            .transpose()
            .map_err(|error| invalid_provenance(source_name, Some(path.clone()), error))?;
        self.budget
            .enter_node(depth, source_name, location, &path)?;
        self.next_slot = self.slot_for_open_container();
        Ok((depth, path))
    }

    fn close_container(
        &mut self,
        mapping: bool,
        loaded_source: &str,
        source_name: &Path,
        span: YamlSpan,
    ) -> Result<(), Diagnostic> {
        let matches = matches!(
            (mapping, self.stack.last()),
            (true, Some(ContainerFrame::Mapping { .. }))
                | (false, Some(ContainerFrame::Sequence { .. }))
        );
        if !matches {
            return Err(lossless_at_span(
                loaded_source,
                source_name,
                span,
                self.container_paths.last().cloned(),
                "YAML event stream closed a mismatched collection".to_owned(),
            ));
        }
        self.stack.pop();
        self.container_paths.pop();
        self.next_slot = self.slot_for_open_container();
        Ok(())
    }

    fn slot_for_open_container(&self) -> ExpectedSlot {
        match self.stack.last() {
            Some(ContainerFrame::Sequence { depth, next_index }) => ExpectedSlot::SequenceItem {
                parent_depth: *depth,
                index: *next_index,
            },
            Some(ContainerFrame::Mapping {
                depth,
                pending_decoded_key: Some(decoded_key),
            }) => ExpectedSlot::MappingValue {
                parent_depth: *depth,
                decoded_key: decoded_key.clone(),
            },
            Some(ContainerFrame::Mapping {
                depth,
                pending_decoded_key: None,
            }) => ExpectedSlot::MappingKey {
                parent_depth: *depth,
            },
            None => ExpectedSlot::Root,
        }
    }

    fn path_for_next_slot(&self) -> StructuredPath {
        match &self.next_slot {
            ExpectedSlot::Root => StructuredPath::root(),
            ExpectedSlot::SequenceItem { index, .. } => {
                self.current_container_path().clone().with_index(*index)
            }
            ExpectedSlot::MappingKey { .. } => self.current_container_path().clone(),
            ExpectedSlot::MappingValue { decoded_key, .. } => self
                .current_container_path()
                .clone()
                .with_key(decoded_key.clone()),
        }
    }

    fn current_container_path(&self) -> &StructuredPath {
        self.container_paths
            .last()
            .expect("a collection slot has a container path")
    }
}

fn project_node(
    node: &YamlNode<YamlSpan>,
    loaded_source: &str,
    source_name: &Path,
    path: &StructuredPath,
) -> Result<Node, Diagnostic> {
    match node {
        YamlNode::Scalar {
            value,
            tag,
            loc,
            meta,
            ..
        } => {
            reject_node_metadata(node, loaded_source, source_name, path)?;
            let span = source_span(*loc, loaded_source, source_name, path)?;
            let value = match tag.as_deref() {
                Some(tag) if tag == ResolvedTag::Str.as_str() => NodeValue::String(value.clone()),
                Some(tag) if tag == ResolvedTag::Int.as_str() => {
                    NodeValue::Number(NumberLexeme::from_parser_verified(value.clone()))
                }
                Some(tag) if tag == ResolvedTag::Float.as_str() => {
                    NodeValue::Number(NumberLexeme::from_parser_verified(value.clone()))
                }
                Some(tag) if tag == ResolvedTag::Bool.as_str() => {
                    NodeValue::Boolean(value.eq_ignore_ascii_case("true"))
                }
                Some(tag) if tag == ResolvedTag::Null.as_str() => NodeValue::Null,
                _ => {
                    return Err(lossless_at_span(
                        loaded_source,
                        source_name,
                        *loc,
                        Some(path.clone()),
                        "YAML scalar tag has no lossless public projection".to_owned(),
                    ));
                }
            };
            let _ = meta;
            Ok(Node::from_parser_verified(span, value))
        }
        YamlNode::Sequence {
            items, tag, loc, ..
        } => {
            reject_node_metadata(node, loaded_source, source_name, path)?;
            if tag.as_deref() != Some(ResolvedTag::Seq.as_str()) {
                return Err(lossless_at_span(
                    loaded_source,
                    source_name,
                    *loc,
                    Some(path.clone()),
                    "YAML sequence tag has no lossless public projection".to_owned(),
                ));
            }
            let span = source_span(*loc, loaded_source, source_name, path)?;
            let values = items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    project_node(
                        item,
                        loaded_source,
                        source_name,
                        &path.clone().with_index(index),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Node::from_parser_verified(
                span,
                NodeValue::Sequence(values),
            ))
        }
        YamlNode::Mapping {
            entries, tag, loc, ..
        } => {
            reject_node_metadata(node, loaded_source, source_name, path)?;
            if tag.as_deref() != Some(ResolvedTag::Map.as_str()) {
                return Err(lossless_at_span(
                    loaded_source,
                    source_name,
                    *loc,
                    Some(path.clone()),
                    "YAML mapping tag has no lossless public projection".to_owned(),
                ));
            }
            let span = source_span(*loc, loaded_source, source_name, path)?;
            let mut decoded_keys = HashSet::with_capacity(entries.len());
            let mut projected = Vec::with_capacity(entries.len());
            for (key, value) in entries {
                reject_node_metadata(key, loaded_source, source_name, path)?;
                let YamlNode::Scalar {
                    value: decoded_key,
                    tag,
                    loc: key_loc,
                    ..
                } = key
                else {
                    return Err(lossless_at_node(
                        key,
                        loaded_source,
                        source_name,
                        Some(path.clone()),
                        "non-scalar YAML mapping keys have no lossless public projection"
                            .to_owned(),
                    ));
                };
                let key_path = path.clone().with_key(decoded_key.clone());
                if tag.as_deref() != Some(ResolvedTag::Str.as_str()) {
                    return Err(lossless_at_span(
                        loaded_source,
                        source_name,
                        *key_loc,
                        Some(key_path),
                        "non-string YAML mapping keys have no lossless public projection"
                            .to_owned(),
                    ));
                }
                let key_span = source_span(*key_loc, loaded_source, source_name, &key_path)?;
                if !decoded_keys.insert(decoded_key.clone()) {
                    let location = location_from_byte_offset(loaded_source, key_span.start_byte())
                        .map_err(|error| {
                            invalid_provenance(source_name, Some(key_path.clone()), error)
                        })?;
                    return Err(Diagnostic::from_parts(
                        DiagnosticCategory::DuplicateDecodedKey,
                        source_name.to_owned(),
                        Some(location),
                        Some(key_path),
                        format!("duplicate decoded mapping key {decoded_key:?}"),
                    ));
                }
                projected.push(MappingEntry::from_parser_verified(
                    decoded_key.clone(),
                    key_span,
                    project_node(value, loaded_source, source_name, &key_path)?,
                ));
            }
            Ok(Node::from_parser_verified(
                span,
                NodeValue::Mapping(projected),
            ))
        }
        YamlNode::Alias { loc, .. } => Err(lossless_at_span(
            loaded_source,
            source_name,
            *loc,
            Some(path.clone()),
            "YAML aliases have no lossless public projection".to_owned(),
        )),
    }
}

fn reject_node_metadata(
    node: &YamlNode<YamlSpan>,
    loaded_source: &str,
    source_name: &Path,
    path: &StructuredPath,
) -> Result<(), Diagnostic> {
    let span = node.anchor_loc().or(node.tag_loc());
    if let Some(span) = span {
        return Err(lossless_at_span(
            loaded_source,
            source_name,
            span,
            Some(path.clone()),
            "YAML anchors or authored tags have no lossless public projection".to_owned(),
        ));
    }
    Ok(())
}

fn source_span(
    span: YamlSpan,
    loaded_source: &str,
    source_name: &Path,
    path: &StructuredPath,
) -> Result<SourceSpan, Diagnostic> {
    SourceSpan::try_from_parser(span.start as usize, span.end as usize, loaded_source)
        .map_err(|error| invalid_provenance(source_name, Some(path.clone()), error))
}

fn next_depth(
    parent_depth: NonZeroUsize,
    loaded_source: &str,
    source_name: &Path,
    span: YamlSpan,
) -> Result<NonZeroUsize, Diagnostic> {
    parent_depth
        .get()
        .checked_add(1)
        .and_then(NonZeroUsize::new)
        .ok_or_else(|| {
            let location = location_from_byte_offset(loaded_source, span.start as usize).ok();
            Diagnostic::from_parts(
                DiagnosticCategory::DepthLimit,
                source_name.to_owned(),
                location,
                None,
                "model depth exceeds the representation limit".to_owned(),
            )
        })
}

fn parser_error(
    loaded_source: &str,
    source_name: &Path,
    byte_offset: usize,
    detail: String,
) -> Diagnostic {
    match location_from_byte_offset(loaded_source, byte_offset) {
        Ok(location) => Diagnostic::from_parts(
            DiagnosticCategory::Parse,
            source_name.to_owned(),
            Some(location),
            None,
            detail,
        ),
        Err(error) => invalid_provenance(source_name, None, error),
    }
}

fn loader_error(error: LoadError, loaded_source: &str, source_name: &Path) -> Diagnostic {
    match error {
        LoadError::Parse { pos, message, .. } => {
            parser_error(loaded_source, source_name, pos.byte_offset, message)
        }
        LoadError::NestingDepthLimitExceeded { pos, .. }
        | LoadError::AnchorCountLimitExceeded { pos, .. }
        | LoadError::AliasExpansionLimitExceeded { pos, .. }
        | LoadError::CircularAlias { pos, .. }
        | LoadError::UndefinedAlias { pos, .. }
        | LoadError::UnresolvedScalar { pos, .. } => {
            match location_from_byte_offset(loaded_source, pos.byte_offset) {
                Ok(location) => {
                    lossless_projection(source_name, Some(location), None, error.to_string())
                }
                Err(provenance) => invalid_provenance(source_name, None, provenance),
            }
        }
        LoadError::UnexpectedEndOfStream => {
            lossless_projection(source_name, None, None, error.to_string())
        }
        _ => lossless_projection(source_name, None, None, error.to_string()),
    }
}

fn lossless_at_node(
    node: &YamlNode<YamlSpan>,
    loaded_source: &str,
    source_name: &Path,
    path: Option<StructuredPath>,
    detail: String,
) -> Diagnostic {
    let span = match node {
        YamlNode::Scalar { loc, .. }
        | YamlNode::Mapping { loc, .. }
        | YamlNode::Sequence { loc, .. }
        | YamlNode::Alias { loc, .. } => *loc,
    };
    lossless_at_span(loaded_source, source_name, span, path, detail)
}

fn lossless_at_span(
    loaded_source: &str,
    source_name: &Path,
    span: YamlSpan,
    path: Option<StructuredPath>,
    detail: String,
) -> Diagnostic {
    match location_from_byte_offset(loaded_source, span.start as usize) {
        Ok(location) => lossless_projection(source_name, Some(location), path, detail),
        Err(error) => invalid_provenance(source_name, path, error),
    }
}

fn lossless_projection(
    source_name: &Path,
    location: Option<SourceLocation>,
    path: Option<StructuredPath>,
    detail: String,
) -> Diagnostic {
    Diagnostic::from_parts(
        DiagnosticCategory::LosslessProjection,
        source_name.to_owned(),
        location,
        path,
        detail,
    )
}

fn invalid_provenance(
    source_name: &Path,
    path: Option<StructuredPath>,
    error: ProvenanceError,
) -> Diagnostic {
    Diagnostic::from_parts(
        DiagnosticCategory::InvalidProvenance,
        source_name.to_owned(),
        None,
        path,
        format!("parser provenance is invalid: {error:?}"),
    )
}
