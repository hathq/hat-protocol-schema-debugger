//! Debug projection for ecosystem protocol and schema identifiers.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use zixcel_graph::{Edge, Graph, GraphSpace, Node, PropertyValue};

pub const PROJECTION_SCHEMA: &str = "hat://hathq/schema/debug/protocol/schema/graph/v1";
const GRAPH_SPACE: &str = "hathq.protocol.schema.debug";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentifierKind {
    Protocol,
    Schema,
    Vocabulary,
    Ledger,
    Transport,
    Resource,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentifierNode {
    pub id: String,
    pub uri: String,
    pub label: String,
    pub kind: IdentifierKind,
    pub scheme: String,
    pub authority: Option<String>,
    pub path_segments: Vec<String>,
    pub version: Option<u64>,
    pub source_paths: Vec<String>,
    pub facts: Vec<IdentifierFact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentifierFact {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentifierEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub kind: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentifierWarning {
    pub code: String,
    pub message: String,
    pub uri: String,
    pub source_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionSummary {
    pub identifiers: usize,
    pub protocols: usize,
    pub schemas: usize,
    pub vocabularies: usize,
    pub ledgers: usize,
    pub transports: usize,
    pub resources: usize,
    pub unknown: usize,
    pub warnings: usize,
    pub graph_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolSchemaProjection {
    pub schema: String,
    pub ecosystem_root: String,
    pub graph_space: String,
    pub summary: ProjectionSummary,
    pub nodes: Vec<IdentifierNode>,
    pub edges: Vec<IdentifierEdge>,
    pub warnings: Vec<IdentifierWarning>,
}

#[derive(Debug, Clone)]
struct Occurrence {
    uri: String,
    source_path: String,
    context: String,
}

#[derive(Debug, Clone)]
struct ParsedUri {
    scheme: String,
    authority: Option<String>,
    path_segments: Vec<String>,
    version: Option<u64>,
}

/// Builds a protocol/schema debug projection from an ecosystem source tree.
///
/// # Errors
///
/// Returns an error when the source tree cannot be scanned or graph projection fails.
pub fn scan_ecosystem(
    root: &Path,
    graph_db: Option<&Path>,
) -> Result<ProtocolSchemaProjection, String> {
    let graph = match graph_db {
        Some(path) => Graph::create(path).map_err(|error| error.to_string())?,
        None => Graph::memory().map_err(|error| error.to_string())?,
    };
    let graph_space = GraphSpace::new(GRAPH_SPACE).map_err(|error| error.to_string())?;
    let mut write = graph
        .write(&graph_space)
        .map_err(|error| error.to_string())?;
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut warnings = Vec::new();
    let mut known_node_ids = BTreeSet::new();
    let root_id = stable_id("root", root.to_string_lossy().as_ref());
    add_graph_node(
        &mut write,
        &mut known_node_ids,
        &root_id,
        &["root"],
        &[
            ("label", "ecosystem providers"),
            ("path", root.to_string_lossy().as_ref()),
        ],
    )?;

    for (uri, items) in grouped_occurrences(root)? {
        let record = identifier_record(uri, &items);
        warnings.extend(record.warnings);
        apply_identifier_to_graph(
            &mut write,
            &mut known_node_ids,
            &mut edges,
            &root_id,
            &record.node,
        )?;
        nodes.push(record.node);
    }

    let revision = write.commit().map_err(|error| error.to_string())?;
    Ok(finalize_projection(root, nodes, edges, warnings, revision))
}

fn grouped_occurrences(root: &Path) -> Result<BTreeMap<String, Vec<Occurrence>>, String> {
    let mut grouped: BTreeMap<String, Vec<Occurrence>> = BTreeMap::new();
    for occurrence in collect_occurrences(root)? {
        grouped
            .entry(occurrence.uri.clone())
            .or_default()
            .push(occurrence);
    }
    Ok(grouped)
}

struct IdentifierRecord {
    node: IdentifierNode,
    warnings: Vec<IdentifierWarning>,
}

fn identifier_record(uri: String, items: &[Occurrence]) -> IdentifierRecord {
    let parsed = parse_uri(&uri);
    let context = items
        .iter()
        .map(|item| item.context.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let kind = classify(&uri, &parsed, &context);
    let source_paths = items
        .iter()
        .map(|item| item.source_path.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut facts = vec![
        fact("scheme", &parsed.scheme),
        fact("source count", &items.len().to_string()),
    ];
    if let Some(authority) = &parsed.authority {
        facts.push(fact("authority", authority));
    }
    if let Some(version) = parsed.version {
        facts.push(fact("version", &version.to_string()));
    }
    let node = IdentifierNode {
        id: stable_id("uri", &uri),
        label: display_label(&uri, &parsed),
        kind,
        scheme: parsed.scheme,
        authority: parsed.authority,
        path_segments: parsed.path_segments,
        version: parsed.version,
        uri,
        source_paths,
        facts,
    };
    let warnings = warnings_for(&node);
    IdentifierRecord { node, warnings }
}

fn warnings_for(node: &IdentifierNode) -> Vec<IdentifierWarning> {
    let mut warnings = Vec::new();
    // Only HATHQ-owned contract paths are governed by the Hatter hierarchy
    // policy.  Provider/resource identifiers (Crowsi, Zixcel, organization,
    // evidence, repository, and similar schemes) are opaque to Hatter and
    // must not be rewritten or reported as naming violations.
    if node.scheme == "hathq" && path_has_compound_segment(&node.path_segments) {
        warnings.push(warning(
            "compound-path-segment",
            "URI path contains a compound segment; split it into semantic hierarchy segments before release.",
            node,
        ));
    }
    // `hathq://hatter/...` is a Hatter-local namespace and must not be
    // mistaken for a HatSpec-owned ecosystem contract.  `hathq://hat/...`
    // remains the 0.10.0 HatSpec namespace until a breaking namespace
    // migration is explicitly released.
    if node.uri.starts_with("hathq://hatter/") {
        warnings.push(warning(
            "hatter-local-namespace",
            "Hatter-local contract namespace; do not publish it as a shared HatSpec contract.",
            node,
        ));
    }
    if matches!(node.kind, IdentifierKind::Unknown) {
        warnings.push(warning(
            "unclassified-identifier",
            "Identifier could not be mechanically classified as protocol, schema, vocabulary, ledger, transport or resource.",
            node,
        ));
    }
    warnings
}

fn warning(code: &str, message: &str, node: &IdentifierNode) -> IdentifierWarning {
    IdentifierWarning {
        code: code.to_owned(),
        message: message.to_owned(),
        uri: node.uri.clone(),
        source_path: node.source_paths.first().cloned().unwrap_or_default(),
    }
}

fn apply_identifier_to_graph(
    write: &mut zixcel_graph::WriteTransaction,
    known_node_ids: &mut BTreeSet<String>,
    edges: &mut Vec<IdentifierEdge>,
    root_id: &str,
    node: &IdentifierNode,
) -> Result<(), String> {
    add_graph_node(
        write,
        known_node_ids,
        &node.id,
        &[label_for_kind(&node.kind), "identifier"],
        &[
            ("uri", &node.uri),
            ("kind", label_for_kind(&node.kind)),
            ("scheme", &node.scheme),
        ],
    )?;
    let scheme_id = stable_id("scheme", &node.scheme);
    add_graph_node(
        write,
        known_node_ids,
        &scheme_id,
        &["scheme"],
        &[("label", &node.scheme)],
    )?;
    edges.push(add_edge(write, &scheme_id, &node.id, "contains")?);
    edges.push(add_edge(write, root_id, &scheme_id, "has_scheme")?);
    if let Some(authority) = &node.authority {
        let authority_id = stable_id("authority", &format!("{}://{authority}", node.scheme));
        add_graph_node(
            write,
            known_node_ids,
            &authority_id,
            &["authority"],
            &[("label", authority), ("scheme", &node.scheme)],
        )?;
        edges.push(add_edge(write, &scheme_id, &authority_id, "has_authority")?);
        edges.push(add_edge(write, &authority_id, &node.id, "declares")?);
    }
    let parsed = ParsedUri {
        scheme: node.scheme.clone(),
        authority: node.authority.clone(),
        path_segments: node.path_segments.clone(),
        version: node.version,
    };
    add_path_hierarchy(write, known_node_ids, edges, &parsed, &node.id)
}

fn finalize_projection(
    root: &Path,
    nodes: Vec<IdentifierNode>,
    edges: Vec<IdentifierEdge>,
    warnings: Vec<IdentifierWarning>,
    revision: u64,
) -> ProtocolSchemaProjection {
    let mut projection = ProtocolSchemaProjection {
        schema: PROJECTION_SCHEMA.to_owned(),
        ecosystem_root: root.to_string_lossy().into_owned(),
        graph_space: GRAPH_SPACE.to_owned(),
        summary: ProjectionSummary {
            identifiers: nodes.len(),
            protocols: nodes
                .iter()
                .filter(|node| node.kind == IdentifierKind::Protocol)
                .count(),
            schemas: nodes
                .iter()
                .filter(|node| node.kind == IdentifierKind::Schema)
                .count(),
            vocabularies: nodes
                .iter()
                .filter(|node| node.kind == IdentifierKind::Vocabulary)
                .count(),
            ledgers: nodes
                .iter()
                .filter(|node| node.kind == IdentifierKind::Ledger)
                .count(),
            transports: nodes
                .iter()
                .filter(|node| node.kind == IdentifierKind::Transport)
                .count(),
            resources: nodes
                .iter()
                .filter(|node| node.kind == IdentifierKind::Resource)
                .count(),
            unknown: nodes
                .iter()
                .filter(|node| node.kind == IdentifierKind::Unknown)
                .count(),
            warnings: warnings.len(),
            graph_revision: revision,
        },
        nodes,
        edges: unique_edges(edges),
        warnings,
    };
    projection
        .nodes
        .sort_by(|left, right| left.uri.cmp(&right.uri));
    projection
        .warnings
        .sort_by(|left, right| left.uri.cmp(&right.uri).then(left.code.cmp(&right.code)));
    projection
}

fn collect_occurrences(root: &Path) -> Result<Vec<Occurrence>, String> {
    let mut output = Vec::new();
    walk(root, root, &mut output)?;
    Ok(output)
}

fn walk(root: &Path, path: &Path, output: &mut Vec<Occurrence>) -> Result<(), String> {
    for entry in fs::read_dir(path).map_err(|error| format!("read {}: {error}", path.display()))? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            if skip_dir(&name) {
                continue;
            }
            walk(root, &path, output)?;
        } else if supported_file(&path) {
            let text = fs::read_to_string(&path).unwrap_or_default();
            let source_path = relative(root, &path);
            for (uri, context) in extract_uris(&text) {
                output.push(Occurrence {
                    uri,
                    source_path: source_path.clone(),
                    context,
                });
            }
        }
    }
    Ok(())
}

fn skip_dir(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | "target"
            | "node_modules"
            | ".nuxt"
            | ".output"
            | "dist"
            | "build"
            | ".cache"
            | "coverage"
            | "vendor"
            | "generated"
    )
}

fn supported_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("rs" | "json" | "md" | "ts" | "vue" | "toml" | "yaml" | "yml" | "mjs" | "js")
    )
}

fn extract_uris(text: &str) -> Vec<(String, String)> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut index = 0;
    while let Some(offset) = text[index..].find("://") {
        let marker = index + offset;
        let mut start = marker;
        while start > 0 && is_scheme_byte(bytes[start - 1]) {
            start -= 1;
        }
        if start == marker || !bytes[start].is_ascii_lowercase() {
            index = marker + 3;
            continue;
        }
        let mut end = marker + 3;
        while end < bytes.len() && !is_uri_delimiter(bytes[end]) {
            end += 1;
        }
        let uri = text[start..end]
            .trim_end_matches(['.', ',', ';'])
            .to_owned();
        if uri.starts_with("http://") || uri.starts_with("https://") {
            index = end;
            continue;
        }
        let line_start = text[..start].rfind('\n').map_or(0, |value| value + 1);
        let line_end = text[end..]
            .find('\n')
            .map_or(text.len(), |value| end + value);
        found.push((uri, text[line_start..line_end].to_owned()));
        index = end;
    }
    found
}

fn is_scheme_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'+' | b'.' | b'-')
}

fn is_uri_delimiter(byte: u8) -> bool {
    byte.is_ascii_whitespace()
        || matches!(
            byte,
            b'"' | b'\'' | b'`' | b')' | b'}' | b']' | b'<' | b'>' | b',' | b';'
        )
}

fn parse_uri(uri: &str) -> ParsedUri {
    let (scheme, rest) = uri.split_once("://").unwrap_or((uri, ""));
    let mut segments = rest
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let authority = segments.first().cloned();
    if authority.is_some() {
        segments.remove(0);
    }
    let version = segments.last().and_then(|segment| {
        segment
            .strip_prefix('v')
            .and_then(|value| value.parse::<u64>().ok())
    });
    ParsedUri {
        scheme: scheme.to_owned(),
        authority,
        path_segments: segments,
        version,
    }
}

fn classify(uri: &str, parsed: &ParsedUri, context: &str) -> IdentifierKind {
    let lower = format!("{uri}\n{context}").to_ascii_lowercase();
    if matches!(
        parsed.scheme.as_str(),
        "ws" | "wss" | "sparse+https" | "file"
    ) {
        return IdentifierKind::Transport;
    }
    // HatSpec contract IDs use an authority to identify the contract owner and
    // a path to identify the semantic family.  Resolve this explicit shape
    // before falling back to source-text heuristics so classification is
    // stable even when a URI appears outside a JSON Schema document.
    if parsed.scheme == "hathq" {
        if let Some(kind) = parsed
            .authority
            .as_deref()
            .and_then(|authority| match authority {
                "vocabulary" => Some(IdentifierKind::Vocabulary),
                "protocol" => Some(IdentifierKind::Protocol),
                "schema" => Some(IdentifierKind::Schema),
                "ledger" => Some(IdentifierKind::Ledger),
                "resource" => Some(IdentifierKind::Resource),
                "transport" => Some(IdentifierKind::Transport),
                _ => None,
            })
        {
            return kind;
        }
        if parsed.authority.as_deref() == Some("hat") {
            return match parsed.path_segments.first().map(String::as_str) {
                Some("vocabulary") => IdentifierKind::Vocabulary,
                Some("protocol") => IdentifierKind::Protocol,
                _ => IdentifierKind::Schema,
            };
        }
        if matches!(
            parsed.authority.as_deref(),
            Some("hat-specifications" | "hatter")
        ) {
            return IdentifierKind::Schema;
        }
    }
    match parsed.path_segments.first().map(String::as_str) {
        Some("protocol") => return IdentifierKind::Protocol,
        Some("schema") => return IdentifierKind::Schema,
        Some("vocabulary") => return IdentifierKind::Vocabulary,
        Some("ledger") => return IdentifierKind::Ledger,
        Some("resource") => return IdentifierKind::Resource,
        Some("transport") => return IdentifierKind::Transport,
        _ => {}
    }
    if parsed
        .path_segments
        .iter()
        .any(|segment| segment == "vocabulary" || segment.starts_with("vocabulary-"))
    {
        return IdentifierKind::Vocabulary;
    }
    if parsed
        .path_segments
        .iter()
        .any(|segment| segment == "ledger" || segment.ends_with("ledger"))
    {
        return IdentifierKind::Ledger;
    }
    if lower.contains("protocol") || lower.contains("\"protocol\"") {
        return IdentifierKind::Protocol;
    }
    if lower.contains("$id")
        || lower.contains("\"schema\"")
        || lower.contains("schema")
        || lower.contains("request")
        || lower.contains("response")
        || lower.contains("receipt")
        || lower.contains("config")
        || lower.contains("catalog")
    {
        return IdentifierKind::Schema;
    }
    if matches!(
        parsed.scheme.as_str(),
        "catalog"
            | "repository"
            | "workspace"
            | "secret"
            | "evidence"
            | "artifact"
            | "command"
            | "connector"
            | "organization"
            | "authority"
            | "app"
    ) {
        return IdentifierKind::Resource;
    }
    IdentifierKind::Unknown
}

fn display_label(uri: &str, parsed: &ParsedUri) -> String {
    parsed
        .path_segments
        .iter()
        .rev()
        .find(|segment| !segment.starts_with('v'))
        .cloned()
        .or_else(|| parsed.authority.clone())
        .unwrap_or_else(|| uri.to_owned())
}

fn path_has_compound_segment(segments: &[String]) -> bool {
    segments.iter().any(|segment| {
        segment.contains('-')
            && !segment.starts_with('v')
            && segment.chars().any(char::is_alphabetic)
    })
}

fn fact(label: &str, value: &str) -> IdentifierFact {
    IdentifierFact {
        label: label.to_owned(),
        value: value.to_owned(),
    }
}

fn label_for_kind(kind: &IdentifierKind) -> &'static str {
    match kind {
        IdentifierKind::Protocol => "protocol",
        IdentifierKind::Schema => "schema",
        IdentifierKind::Vocabulary => "vocabulary",
        IdentifierKind::Ledger => "ledger",
        IdentifierKind::Transport => "transport",
        IdentifierKind::Resource => "resource",
        IdentifierKind::Unknown => "unknown",
    }
}

fn add_graph_node(
    write: &mut zixcel_graph::WriteTransaction,
    known: &mut BTreeSet<String>,
    id: &str,
    labels: &[&str],
    properties: &[(&str, &str)],
) -> Result<(), String> {
    if !known.insert(id.to_owned()) {
        return Ok(());
    }
    let mut node = Node::new(id, labels.iter().copied()).map_err(|error| error.to_string())?;
    for (key, value) in properties {
        node = node
            .with_property(*key, PropertyValue::String((*value).to_owned()))
            .map_err(|error| error.to_string())?;
    }
    write.upsert_node(node);
    Ok(())
}

fn add_path_hierarchy(
    write: &mut zixcel_graph::WriteTransaction,
    known: &mut BTreeSet<String>,
    edges: &mut Vec<IdentifierEdge>,
    parsed: &ParsedUri,
    terminal_id: &str,
) -> Result<(), String> {
    let Some(authority) = &parsed.authority else {
        return Ok(());
    };
    let mut prefix = format!("{}://{authority}", parsed.scheme);
    let mut parent_id = stable_id("authority", &prefix);
    for segment in &parsed.path_segments {
        prefix.push('/');
        prefix.push_str(segment);
        let segment_id = stable_id("path", &prefix);
        add_graph_node(
            write,
            known,
            &segment_id,
            &["path_segment"],
            &[("segment", segment), ("prefix", &prefix)],
        )?;
        edges.push(add_edge(write, &parent_id, &segment_id, "contains")?);
        parent_id = segment_id;
    }
    edges.push(add_edge(write, &parent_id, terminal_id, "identifies")?);
    Ok(())
}

fn add_edge(
    write: &mut zixcel_graph::WriteTransaction,
    from: &str,
    to: &str,
    kind: &str,
) -> Result<IdentifierEdge, String> {
    let id = stable_id("edge", &format!("{from}|{kind}|{to}"));
    write.upsert_edge(Edge::directed(&id, from, to, kind).map_err(|error| error.to_string())?);
    Ok(IdentifierEdge {
        id,
        from: from.to_owned(),
        to: to.to_owned(),
        kind: kind.to_owned(),
        label: kind.replace('_', " "),
    })
}

fn unique_edges(edges: Vec<IdentifierEdge>) -> Vec<IdentifierEdge> {
    let mut values = BTreeMap::new();
    for edge in edges {
        values.insert(edge.id.clone(), edge);
    }
    values.into_values().collect()
}

fn stable_id(prefix: &str, value: &str) -> String {
    format!("{prefix}:{:016x}", fnv1a(value.as_bytes()))
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Writes a projection as pretty JSON.
///
/// # Errors
///
/// Returns an error when the file cannot be written.
pub fn write_projection(path: &Path, projection: &ProtocolSchemaProjection) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let text = serde_json::to_string_pretty(projection).map_err(|error| error.to_string())?;
    fs::write(path, format!("{text}\n")).map_err(|error| error.to_string())
}

/// Parses CLI arguments without adding a command-line framework dependency.
///
/// # Errors
///
/// Returns an error when required values are missing or an unknown flag is supplied.
pub fn run_cli(args: impl IntoIterator<Item = String>) -> Result<(), String> {
    let mut args = args.into_iter();
    let _program = args.next();
    let Some(command) = args.next() else {
        return Err(usage());
    };
    if command != "scan" {
        return Err(usage());
    }
    let mut ecosystem_root: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut graph_db: Option<PathBuf> = None;
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--ecosystem-root" => ecosystem_root = args.next().map(PathBuf::from),
            "--output" => output = args.next().map(PathBuf::from),
            "--graph-db" => graph_db = args.next().map(PathBuf::from),
            _ => return Err(format!("unknown argument {flag}\n{}", usage())),
        }
    }
    let root = ecosystem_root.ok_or_else(usage)?;
    let projection = scan_ecosystem(&root, graph_db.as_deref())?;
    if let Some(path) = output {
        write_projection(&path, &projection)?;
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&projection).map_err(|error| error.to_string())?
        );
    }
    Ok(())
}

fn usage() -> String {
    "usage: hat-protocol-schema-debugger scan --ecosystem-root <path> [--output <path>] [--graph-db <path>]".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_and_classifies_schema_protocol_and_vocabulary() {
        let temp = tempfile::tempdir().expect("tempdir");
        let schema = temp.path().join("sample.schema.json");
        fs::write(
            &schema,
            r#"{
              "$id": "hat://hathq/schema/vocabulary/term/v1",
              "properties": {
                "protocol": { "const": "zixcel://github/protocol/api/v1" },
                "term": { "const": "hat://hathq/vocabulary/core/action/infer/v1" }
              }
            }"#,
        )
        .expect("fixture");
        let projection = scan_ecosystem(temp.path(), None).expect("projection");
        assert!(
            projection
                .nodes
                .iter()
                .any(|node| node.uri == "hat://hathq/schema/vocabulary/term/v1"
                    && node.kind == IdentifierKind::Schema)
        );
        assert!(
            projection
                .nodes
                .iter()
                .any(|node| node.uri == "zixcel://github/protocol/api/v1"
                    && node.kind == IdentifierKind::Protocol)
        );
        assert!(projection.nodes.iter().any(|node| node.uri
            == "hat://hathq/vocabulary/core/action/infer/v1"
            && node.kind == IdentifierKind::Vocabulary));
        assert_eq!(projection.summary.graph_revision, 1);
    }

    #[test]
    fn warns_about_compound_segments_and_hatter_local_namespace() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(
            temp.path().join("legacy.rs"),
            r#"pub const X: &str = "hathq://hatter/local-registry-ledger/v1";"#,
        )
        .expect("fixture");
        let projection = scan_ecosystem(temp.path(), None).expect("projection");
        let codes = projection
            .warnings
            .iter()
            .map(|warning| warning.code.as_str())
            .collect::<BTreeSet<_>>();
        assert!(codes.contains("compound-path-segment"));
        assert!(codes.contains("hatter-local-namespace"));
    }

    #[test]
    fn does_not_rescan_generated_projection() {
        let temp = tempfile::tempdir().expect("tempdir");
        let generated = temp.path().join("generated");
        fs::create_dir_all(&generated).expect("generated directory");
        fs::write(
            generated.join("projection.json"),
            r#"{"$id":"hat://hathq/schema/generated/v1"}"#,
        )
        .expect("generated fixture");
        fs::write(
            temp.path().join("source.json"),
            r#"{"$id":"hat://hathq/schema/source/v1"}"#,
        )
        .expect("source fixture");
        let projection = scan_ecosystem(temp.path(), None).expect("projection");
        assert!(
            projection
                .nodes
                .iter()
                .any(|node| node.uri == "hat://hathq/schema/source/v1")
        );
        assert!(
            !projection
                .nodes
                .iter()
                .any(|node| node.uri == "hat://hathq/schema/generated/v1")
        );
    }

    #[test]
    fn classifies_hat_spec_ids_without_context_heuristics() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(
            temp.path().join("constants.rs"),
            r#"
              const A: &str = "hathq://hat/semantic-protocol/v1";
              const B: &str = "hathq://hat/vocabulary/action/infer/v1";
              const C: &str = "hathq://hat/semantic-clause/v1";
            "#,
        )
        .expect("fixture");
        let projection = scan_ecosystem(temp.path(), None).expect("projection");
        let kind = |uri: &str| {
            projection
                .nodes
                .iter()
                .find(|node| node.uri == uri)
                .map(|node| node.kind.clone())
                .expect("identifier")
        };
        assert_eq!(
            kind("hathq://hat/semantic-protocol/v1"),
            IdentifierKind::Schema
        );
        assert_eq!(
            kind("hathq://hat/vocabulary/action/infer/v1"),
            IdentifierKind::Vocabulary
        );
        assert_eq!(
            kind("hathq://hat/semantic-clause/v1"),
            IdentifierKind::Schema
        );
    }

    #[test]
    fn keeps_external_compound_identifiers_opaque() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(
            temp.path().join("external.json"),
            r#"{"id":"crowsi://credential-authority/device-management/v1"}"#,
        )
        .expect("fixture");
        let projection = scan_ecosystem(temp.path(), None).expect("projection");
        assert!(!projection.warnings.iter().any(|warning| {
            warning.code == "compound-path-segment"
                && warning.uri == "crowsi://credential-authority/device-management/v1"
        }));
    }
}
