//! Deterministic, side-effect-free parser for declarative `SKILL.md` packages.
//!
//! The parser accepts a closed JSON frontmatter object followed by Markdown.
//! It returns typed metadata, untrusted instruction sections, and untrusted
//! artifact data separately. It never reads files, resolves links, mutates
//! runtime state, imports a skill, or executes scripts.

use crate::ids::{ProjectId, SkillId, TraceId};
use crate::skill::{Skill, SkillError, SkillFileRole, SkillManifest, SkillScope, SkillSource};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const SKILL_PARSER_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_MAX_DOCUMENT_BYTES: usize = 256 * 1024;
pub const DEFAULT_MAX_FRONTMATTER_BYTES: usize = 64 * 1024;
pub const DEFAULT_MAX_SECTION_BYTES: usize = 64 * 1024;
pub const DEFAULT_MAX_ARTIFACT_BYTES: usize = 512 * 1024;
pub const DEFAULT_MAX_SCRIPT_BYTES: usize = 128 * 1024;
pub const DEFAULT_MAX_TEMPLATE_BYTES: usize = 128 * 1024;
pub const DEFAULT_MAX_REFERENCE_BYTES: usize = 256 * 1024;
pub const DEFAULT_MAX_TEST_BYTES: usize = 256 * 1024;
pub const DEFAULT_MAX_SECTIONS: usize = 64;
pub const DEFAULT_MAX_LINKS: usize = 128;
pub const DEFAULT_MAX_JSON_DEPTH: usize = 16;
pub const DEFAULT_MAX_HEADING_DEPTH: u8 = 6;
pub const DEFAULT_MAX_DIAGNOSTICS: usize = 64;

/// Limits applied before any untrusted content is returned to a caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillParserLimits {
    pub max_document_bytes: usize,
    pub max_frontmatter_bytes: usize,
    pub max_section_bytes: usize,
    pub max_artifact_bytes: usize,
    pub max_script_bytes: usize,
    pub max_template_bytes: usize,
    pub max_reference_bytes: usize,
    pub max_test_bytes: usize,
    pub max_sections: usize,
    pub max_links: usize,
    pub max_json_depth: usize,
    pub max_heading_depth: u8,
    pub max_diagnostics: usize,
}

impl Default for SkillParserLimits {
    fn default() -> Self {
        Self {
            max_document_bytes: DEFAULT_MAX_DOCUMENT_BYTES,
            max_frontmatter_bytes: DEFAULT_MAX_FRONTMATTER_BYTES,
            max_section_bytes: DEFAULT_MAX_SECTION_BYTES,
            max_artifact_bytes: DEFAULT_MAX_ARTIFACT_BYTES,
            max_script_bytes: DEFAULT_MAX_SCRIPT_BYTES,
            max_template_bytes: DEFAULT_MAX_TEMPLATE_BYTES,
            max_reference_bytes: DEFAULT_MAX_REFERENCE_BYTES,
            max_test_bytes: DEFAULT_MAX_TEST_BYTES,
            max_sections: DEFAULT_MAX_SECTIONS,
            max_links: DEFAULT_MAX_LINKS,
            max_json_depth: DEFAULT_MAX_JSON_DEPTH,
            max_heading_depth: DEFAULT_MAX_HEADING_DEPTH,
            max_diagnostics: DEFAULT_MAX_DIAGNOSTICS,
        }
    }
}

/// Input supplied by a loader or repository adapter. The parser does not
/// access the filesystem; every artifact is explicit data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillParseRequest {
    pub document: String,
    pub files: Vec<SkillFileInput>,
    pub project_id: Option<ProjectId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillFileInput {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SkillParseError {
    #[error("skill document exceeds parser limit")]
    DocumentTooLarge { actual: usize, max: usize },
    #[error("skill document is missing the opening frontmatter delimiter")]
    MissingFrontmatter,
    #[error("skill document is missing the closing frontmatter delimiter")]
    MissingFrontmatterTerminator,
    #[error("skill frontmatter exceeds parser limit")]
    FrontmatterTooLarge { actual: usize, max: usize },
    #[error("skill frontmatter is malformed near line {line}")]
    MalformedFrontmatter { line: usize },
    #[error("skill frontmatter nesting exceeds parser limit")]
    FrontmatterTooDeep { max: usize },
    #[error("skill document contains an empty instruction body")]
    EmptyDocument,
    #[error("skill document contains an invalid control character")]
    InvalidControlCharacter { field: &'static str },
    #[error("skill document contains too many sections")]
    TooManySections { max: usize },
    #[error("skill document contains a heading deeper than the parser limit")]
    HeadingTooDeep { line: usize, max: u8 },
    #[error("skill document contains an invalid empty heading")]
    InvalidHeading { line: usize },
    #[error("skill document contains a duplicate section")]
    DuplicateSection { line: usize },
    #[error("skill section exceeds parser limit")]
    SectionTooLarge { line: usize, max: usize },
    #[error("skill document contains too many links")]
    TooManyLinks { max: usize },
    #[error("skill document contains an unsafe link")]
    InvalidLink { line: usize, reason: &'static str },
    #[error("skill document links to an undeclared file")]
    UndeclaredLink { line: usize },
    #[error("skill artifact is missing from the parse request")]
    MissingArtifact { path: String },
    #[error("skill artifact is not declared by the manifest")]
    UndeclaredArtifact { path: String },
    #[error("skill artifact is declared more than once")]
    DuplicateArtifact { path: String },
    #[error("skill artifact uses SKILL.md, which is supplied as the document")]
    DocumentArtifactConflict,
    #[error("skill artifact exceeds its role limit")]
    ArtifactTooLarge { path: String, max: usize },
    #[error("skill diagnostic limit exceeded")]
    TooManyDiagnostics { max: usize },
    #[error(transparent)]
    Manifest(#[from] SkillError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillDiagnosticSeverity {
    Warning,
    Quarantine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillDiagnosticCode {
    ExternalLink,
    InstructionOverride,
}

/// Diagnostics deliberately contain no raw skill content or URLs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillDiagnostic {
    pub code: SkillDiagnosticCode,
    pub severity: SkillDiagnosticSeverity,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillInstructionSection {
    pub heading: String,
    pub level: u8,
    pub line: usize,
    pub content: String,
    pub quarantined: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillLinkKind {
    Internal,
    External,
    Anchor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillLink {
    pub source_path: String,
    pub target: String,
    pub kind: SkillLinkKind,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillArtifact {
    pub path: String,
    pub role: SkillFileRole,
    pub content: String,
}

/// Provenance is bounded metadata only; no instruction or artifact body is
/// copied into the trace record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillParseProvenance {
    pub schema_version: u32,
    pub skill_id: SkillId,
    pub version: String,
    pub source: SkillSource,
    pub scope: SkillScope,
    pub project_id: Option<ProjectId>,
    pub trace_id: TraceId,
    pub document_bytes: usize,
    pub artifact_count: usize,
    pub diagnostics_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSkill {
    /// Declarative metadata parsed from the closed frontmatter object.
    pub manifest: SkillManifest,
    /// Markdown instruction content, always untrusted and separate from data.
    pub instructions: Vec<SkillInstructionSection>,
    /// Scripts, templates, references, and tests are returned as data only.
    pub artifacts: Vec<SkillArtifact>,
    pub links: Vec<SkillLink>,
    pub diagnostics: Vec<SkillDiagnostic>,
    pub quarantined: bool,
    pub provenance: SkillParseProvenance,
}

#[derive(Debug, Clone, Default)]
pub struct SkillParser {
    limits: SkillParserLimits,
}

impl SkillParser {
    pub fn new(limits: SkillParserLimits) -> Self {
        Self { limits }
    }

    pub fn limits(&self) -> &SkillParserLimits {
        &self.limits
    }

    pub fn parse(&self, request: SkillParseRequest) -> Result<ParsedSkill, SkillParseError> {
        let document_bytes = request.document.len();
        if document_bytes > self.limits.max_document_bytes {
            return Err(SkillParseError::DocumentTooLarge {
                actual: document_bytes,
                max: self.limits.max_document_bytes,
            });
        }
        reject_control_characters(&request.document, "document")?;

        let (frontmatter, body, body_start_line) = split_frontmatter(&request.document)?;
        if frontmatter.len() > self.limits.max_frontmatter_bytes {
            return Err(SkillParseError::FrontmatterTooLarge {
                actual: frontmatter.len(),
                max: self.limits.max_frontmatter_bytes,
            });
        }

        let value: Value = serde_json::from_str(&frontmatter)
            .map_err(|error| SkillParseError::MalformedFrontmatter { line: error.line() })?;
        if !json_depth_within(&value, 1, self.limits.max_json_depth) {
            return Err(SkillParseError::FrontmatterTooDeep {
                max: self.limits.max_json_depth,
            });
        }
        let manifest: SkillManifest = serde_json::from_value(value)
            .map_err(|error| SkillParseError::MalformedFrontmatter { line: error.line() })?;
        Skill::new(manifest.clone(), request.project_id)
            .validate()
            .map_err(SkillParseError::Manifest)?;

        let declared_files = declared_files(&manifest);
        let markdown = parse_markdown(body, body_start_line, &declared_files, &self.limits)?;
        let artifacts = parse_artifacts(&request.files, &declared_files, &self.limits)?;

        let provenance = SkillParseProvenance {
            schema_version: SKILL_PARSER_SCHEMA_VERSION,
            skill_id: manifest.id,
            version: manifest.version.clone(),
            source: manifest.source.clone(),
            scope: manifest.scope,
            project_id: request.project_id,
            trace_id: manifest.trace.trace_id,
            document_bytes,
            artifact_count: artifacts.len(),
            diagnostics_count: markdown.diagnostics.len(),
        };

        Ok(ParsedSkill {
            manifest,
            instructions: markdown.instructions,
            artifacts,
            links: markdown.links,
            diagnostics: markdown.diagnostics,
            quarantined: markdown.quarantined,
            provenance,
        })
    }
}

fn split_frontmatter(document: &str) -> Result<(String, &str, usize), SkillParseError> {
    let mut lines = document.split_inclusive('\n');
    let Some(first) = lines.next() else {
        return Err(SkillParseError::MissingFrontmatter);
    };
    if trim_line(first) != "---" {
        return Err(SkillParseError::MissingFrontmatter);
    }

    let mut frontmatter = String::new();
    let mut offset = first.len();
    let mut frontmatter_lines = 0;
    while offset < document.len() {
        let remaining = &document[offset..];
        let end = remaining
            .find('\n')
            .map(|index| offset + index + 1)
            .unwrap_or(document.len());
        let line = &document[offset..end];
        if trim_line(line) == "---" {
            return Ok((frontmatter, &document[end..], frontmatter_lines + 3));
        }
        frontmatter.push_str(line);
        frontmatter_lines += 1;
        offset = end;
    }
    Err(SkillParseError::MissingFrontmatterTerminator)
}

fn trim_line(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
}

fn declared_files(manifest: &SkillManifest) -> BTreeMap<String, SkillFileRole> {
    manifest
        .files
        .iter()
        .map(|file| (file.path.clone(), file.role))
        .collect()
}

struct ParsedMarkdown {
    instructions: Vec<SkillInstructionSection>,
    links: Vec<SkillLink>,
    diagnostics: Vec<SkillDiagnostic>,
    quarantined: bool,
}

struct SectionBuilder {
    heading: String,
    level: u8,
    line: usize,
    content: String,
}

fn parse_markdown(
    body: &str,
    body_start_line: usize,
    declared_files: &BTreeMap<String, SkillFileRole>,
    limits: &SkillParserLimits,
) -> Result<ParsedMarkdown, SkillParseError> {
    if body.trim().is_empty() {
        return Err(SkillParseError::EmptyDocument);
    }

    let declared_paths = declared_files.keys().cloned().collect::<BTreeSet<_>>();
    let mut sections = Vec::new();
    let mut links = Vec::new();
    let mut diagnostics = Vec::new();
    let mut quarantined = false;
    let mut seen_headings = BTreeSet::new();
    let mut current_heading = String::new();
    let mut current_level = 0;
    let mut current_line = body_start_line;
    let mut current_content = String::new();
    let mut saw_heading = false;

    for (index, raw_line) in body.lines().enumerate() {
        let line_number = body_start_line + index;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if let Some((level, title)) = markdown_heading(line) {
            if level > limits.max_heading_depth {
                return Err(SkillParseError::HeadingTooDeep {
                    line: line_number,
                    max: limits.max_heading_depth,
                });
            }
            if title.is_empty() {
                return Err(SkillParseError::InvalidHeading { line: line_number });
            }
            let heading_key = title.to_ascii_lowercase();
            if !seen_headings.insert(heading_key) {
                return Err(SkillParseError::DuplicateSection { line: line_number });
            }
            if saw_heading {
                finish_section(
                    SectionBuilder {
                        heading: current_heading,
                        level: current_level,
                        line: current_line,
                        content: current_content,
                    },
                    &mut sections,
                    &mut diagnostics,
                    &mut quarantined,
                    limits,
                )?;
            }
            if sections.len() >= limits.max_sections {
                return Err(SkillParseError::TooManySections {
                    max: limits.max_sections,
                });
            }
            current_heading = title.to_owned();
            current_level = level;
            current_line = line_number;
            current_content = String::new();
            saw_heading = true;
        } else {
            parse_links(
                line,
                line_number,
                &declared_paths,
                &mut links,
                &mut diagnostics,
                limits,
            )?;
            append_section_line(&mut current_content, line, current_line, limits)?;
        }
    }

    if saw_heading {
        finish_section(
            SectionBuilder {
                heading: current_heading,
                level: current_level,
                line: current_line,
                content: current_content,
            },
            &mut sections,
            &mut diagnostics,
            &mut quarantined,
            limits,
        )?;
    } else {
        finish_section(
            SectionBuilder {
                heading: String::new(),
                level: 0,
                line: current_line,
                content: current_content,
            },
            &mut sections,
            &mut diagnostics,
            &mut quarantined,
            limits,
        )?;
    }

    Ok(ParsedMarkdown {
        instructions: sections,
        links,
        diagnostics,
        quarantined,
    })
}

fn markdown_heading(line: &str) -> Option<(u8, &str)> {
    let level = line.bytes().take_while(|byte| *byte == b'#').count();
    if level == 0 || line.as_bytes().get(level) != Some(&b' ') {
        return None;
    }
    Some((level as u8, line[level + 1..].trim()))
}

fn finish_section(
    builder: SectionBuilder,
    sections: &mut Vec<SkillInstructionSection>,
    diagnostics: &mut Vec<SkillDiagnostic>,
    quarantined: &mut bool,
    limits: &SkillParserLimits,
) -> Result<(), SkillParseError> {
    if sections.len() >= limits.max_sections {
        return Err(SkillParseError::TooManySections {
            max: limits.max_sections,
        });
    }
    if builder.content.len() > limits.max_section_bytes {
        return Err(SkillParseError::SectionTooLarge {
            line: builder.line,
            max: limits.max_section_bytes,
        });
    }
    let injection = contains_instruction_override(&builder.heading)
        || contains_instruction_override(&builder.content);
    if injection {
        push_diagnostic(
            diagnostics,
            SkillDiagnostic {
                code: SkillDiagnosticCode::InstructionOverride,
                severity: SkillDiagnosticSeverity::Quarantine,
                line: builder.line,
            },
            limits,
        )?;
        *quarantined = true;
    }
    sections.push(SkillInstructionSection {
        heading: builder.heading,
        level: builder.level,
        line: builder.line,
        content: builder.content,
        quarantined: injection,
    });
    Ok(())
}

fn append_section_line(
    content: &mut String,
    line: &str,
    section_line: usize,
    limits: &SkillParserLimits,
) -> Result<(), SkillParseError> {
    content.push_str(line);
    content.push('\n');
    if content.len() > limits.max_section_bytes {
        return Err(SkillParseError::SectionTooLarge {
            line: section_line,
            max: limits.max_section_bytes,
        });
    }
    Ok(())
}

fn parse_links(
    line: &str,
    line_number: usize,
    declared_paths: &BTreeSet<String>,
    links: &mut Vec<SkillLink>,
    diagnostics: &mut Vec<SkillDiagnostic>,
    limits: &SkillParserLimits,
) -> Result<(), SkillParseError> {
    let mut cursor = 0;
    while let Some(relative) = line[cursor..].find("](") {
        let open = cursor + relative;
        let target_start = open + 2;
        let Some(close_relative) = line[target_start..].find(')') else {
            return Err(SkillParseError::InvalidLink {
                line: line_number,
                reason: "unterminated target",
            });
        };
        let close = target_start + close_relative;
        let raw_target = line[target_start..close].trim();
        let target = raw_target
            .strip_prefix('<')
            .and_then(|value| value.strip_suffix('>'))
            .unwrap_or(raw_target)
            .split_whitespace()
            .next()
            .unwrap_or_default();
        if target.is_empty() || target.len() > 512 {
            return Err(SkillParseError::InvalidLink {
                line: line_number,
                reason: "empty or oversized target",
            });
        }

        let kind = classify_link(target, declared_paths, line_number)?;
        if links.len() >= limits.max_links {
            return Err(SkillParseError::TooManyLinks {
                max: limits.max_links,
            });
        }
        if kind == SkillLinkKind::External {
            push_diagnostic(
                diagnostics,
                SkillDiagnostic {
                    code: SkillDiagnosticCode::ExternalLink,
                    severity: SkillDiagnosticSeverity::Warning,
                    line: line_number,
                },
                limits,
            )?;
        }
        links.push(SkillLink {
            source_path: "SKILL.md".into(),
            target: target.to_owned(),
            kind,
            line: line_number,
        });
        cursor = close + 1;
    }
    Ok(())
}

fn classify_link(
    target: &str,
    declared_paths: &BTreeSet<String>,
    line: usize,
) -> Result<SkillLinkKind, SkillParseError> {
    if target.chars().any(char::is_control)
        || target.contains('\\')
        || target.starts_with('/')
        || target.starts_with("//")
        || target.to_ascii_lowercase().contains("%2e")
    {
        return Err(SkillParseError::InvalidLink {
            line,
            reason: "absolute, encoded, or control path",
        });
    }
    let lower = target.to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") {
        if target.chars().any(char::is_whitespace) {
            return Err(SkillParseError::InvalidLink {
                line,
                reason: "external target contains whitespace",
            });
        }
        return Ok(SkillLinkKind::External);
    }
    if target.starts_with('#') {
        return Ok(SkillLinkKind::Anchor);
    }
    if lower.starts_with("javascript:")
        || lower.starts_with("data:")
        || lower.starts_with("file:")
        || lower.starts_with("mailto:")
        || target.contains(':')
    {
        return Err(SkillParseError::InvalidLink {
            line,
            reason: "unsupported or executable scheme",
        });
    }

    let path = target.split('#').next().unwrap_or_default();
    if path.is_empty()
        || path
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
        || !declared_paths.contains(path)
    {
        if path
            .split('/')
            .any(|segment| segment == ".." || segment == ".")
        {
            return Err(SkillParseError::InvalidLink {
                line,
                reason: "path traversal",
            });
        }
        return Err(SkillParseError::UndeclaredLink { line });
    }
    Ok(SkillLinkKind::Internal)
}

fn parse_artifacts(
    files: &[SkillFileInput],
    declared_files: &BTreeMap<String, SkillFileRole>,
    limits: &SkillParserLimits,
) -> Result<Vec<SkillArtifact>, SkillParseError> {
    let mut seen = BTreeSet::new();
    let mut artifacts = Vec::new();
    for file in files {
        if file.path == "SKILL.md" {
            return Err(SkillParseError::DocumentArtifactConflict);
        }
        if !seen.insert(file.path.clone()) {
            return Err(SkillParseError::DuplicateArtifact {
                path: file.path.clone(),
            });
        }
        let Some(role) = declared_files.get(&file.path).copied() else {
            return Err(SkillParseError::UndeclaredArtifact {
                path: file.path.clone(),
            });
        };
        reject_control_characters(&file.content, "artifact")?;
        let max = artifact_limit(role, limits);
        if file.content.len() > max {
            return Err(SkillParseError::ArtifactTooLarge {
                path: file.path.clone(),
                max,
            });
        }
        artifacts.push(SkillArtifact {
            path: file.path.clone(),
            role,
            content: file.content.clone(),
        });
    }

    for path in declared_files
        .keys()
        .filter(|path| path.as_str() != "SKILL.md")
    {
        if !seen.contains(path) {
            return Err(SkillParseError::MissingArtifact { path: path.clone() });
        }
    }
    Ok(artifacts)
}

fn artifact_limit(role: SkillFileRole, limits: &SkillParserLimits) -> usize {
    match role {
        SkillFileRole::Script => limits.max_script_bytes,
        SkillFileRole::Template => limits.max_template_bytes,
        SkillFileRole::Reference => limits.max_reference_bytes,
        SkillFileRole::Test => limits.max_test_bytes,
        SkillFileRole::Instruction | SkillFileRole::Manifest => limits.max_artifact_bytes,
    }
}

fn push_diagnostic(
    diagnostics: &mut Vec<SkillDiagnostic>,
    diagnostic: SkillDiagnostic,
    limits: &SkillParserLimits,
) -> Result<(), SkillParseError> {
    if diagnostics.len() >= limits.max_diagnostics {
        return Err(SkillParseError::TooManyDiagnostics {
            max: limits.max_diagnostics,
        });
    }
    diagnostics.push(diagnostic);
    Ok(())
}

fn reject_control_characters(value: &str, field: &'static str) -> Result<(), SkillParseError> {
    if value
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(SkillParseError::InvalidControlCharacter { field });
    }
    Ok(())
}

fn json_depth_within(value: &Value, depth: usize, max: usize) -> bool {
    if depth > max {
        return false;
    }
    match value {
        Value::Array(values) => values
            .iter()
            .all(|value| json_depth_within(value, depth + 1, max)),
        Value::Object(values) => values
            .values()
            .all(|value| json_depth_within(value, depth + 1, max)),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => true,
    }
}

fn contains_instruction_override(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "ignore previous instructions",
        "ignore all previous instructions",
        "disregard previous instructions",
        "override system instructions",
        "system message",
        "developer message",
        "<|system|>",
        "<|developer|>",
        "jailbreak",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}
