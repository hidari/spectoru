//! `serde_json` による [`JsonCodec`](crate::ports::json_codec::JsonCodec) 実装。
//!
//! serde 派生はこのファイル内の DTO 群に閉じ込められており、`core::ir` の
//! ドメイン型は serde に依存しない。フィールド名・enum 表現は方向性ドキュメント
//! の JSON 例に揃えてある。

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::ir::{
    Diagnostic, DiagnosticCode, DiagnosticLevel, Group, IntermediateRepresentation, Language,
    Languages, ProjectMeta, Source, Spec, SpecStatus, Stats,
};
use crate::error::SpectoruError;
use crate::ports::json_codec::JsonCodec;

#[derive(Debug, Default, Clone, Copy)]
pub struct SerdeJsonCodec;

impl JsonCodec for SerdeJsonCodec {
    fn encode(&self, ir: &IntermediateRepresentation) -> Result<String, SpectoruError> {
        let dto = IrDto::from(ir);
        serde_json::to_string_pretty(&dto).map_err(|e| SpectoruError::JsonEncode {
            message: e.to_string(),
        })
    }

    fn decode(&self, json: &str) -> Result<IntermediateRepresentation, SpectoruError> {
        let dto: IrDto = serde_json::from_str(json).map_err(|e| SpectoruError::JsonDecode {
            message: e.to_string(),
        })?;
        Ok(dto.into())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct IrDto {
    project: ProjectDto,
    sources: Vec<SourceDto>,
    diagnostics: Vec<DiagnosticDto>,
    stats: StatsDto,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProjectDto {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    revision: Option<String>,
    extracted_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SourceDto {
    name: String,
    groups: Vec<GroupDto>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GroupDto {
    name: String,
    file: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<u32>,
    children: Vec<GroupDto>,
    specs: Vec<SpecDto>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SpecDto {
    name: String,
    file: PathBuf,
    line: u32,
    language: LanguageDto,
    status: SpecStatusDto,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum LanguageDto {
    Rust,
    TypeScript,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SpecStatusDto {
    Active,
    Skipped,
}

#[derive(Debug, Serialize, Deserialize)]
struct DiagnosticDto {
    level: DiagnosticLevelDto,
    code: DiagnosticCodeDto,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum DiagnosticLevelDto {
    Warning,
    Error,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DiagnosticCodeDto {
    NestingTooDeep,
    EmptyName,
    DynamicTestName,
    GitRevisionUnavailable,
    ParseError,
}

#[derive(Debug, Serialize, Deserialize)]
struct StatsDto {
    total_specs: usize,
    warnings: usize,
    languages: LanguagesDto,
}

#[derive(Debug, Serialize, Deserialize)]
struct LanguagesDto {
    rust: usize,
    typescript: usize,
}

impl From<&IntermediateRepresentation> for IrDto {
    fn from(ir: &IntermediateRepresentation) -> Self {
        Self {
            project: ProjectDto::from(&ir.project),
            sources: ir.sources.iter().map(SourceDto::from).collect(),
            diagnostics: ir.diagnostics.iter().map(DiagnosticDto::from).collect(),
            stats: StatsDto::from(&ir.stats),
        }
    }
}

impl From<IrDto> for IntermediateRepresentation {
    fn from(dto: IrDto) -> Self {
        Self {
            project: dto.project.into(),
            sources: dto.sources.into_iter().map(Into::into).collect(),
            diagnostics: dto.diagnostics.into_iter().map(Into::into).collect(),
            stats: dto.stats.into(),
        }
    }
}

impl From<&ProjectMeta> for ProjectDto {
    fn from(p: &ProjectMeta) -> Self {
        Self {
            name: p.name.clone(),
            repository: p.repository.clone(),
            revision: p.revision.clone(),
            extracted_at: p.extracted_at.clone(),
        }
    }
}

impl From<ProjectDto> for ProjectMeta {
    fn from(d: ProjectDto) -> Self {
        Self {
            name: d.name,
            repository: d.repository,
            revision: d.revision,
            extracted_at: d.extracted_at,
        }
    }
}

impl From<&Source> for SourceDto {
    fn from(s: &Source) -> Self {
        Self {
            name: s.name.clone(),
            groups: s.groups.iter().map(GroupDto::from).collect(),
        }
    }
}

impl From<SourceDto> for Source {
    fn from(d: SourceDto) -> Self {
        Self {
            name: d.name,
            groups: d.groups.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<&Group> for GroupDto {
    fn from(g: &Group) -> Self {
        Self {
            name: g.name.clone(),
            file: g.file.clone(),
            line: g.line,
            children: g.children.iter().map(GroupDto::from).collect(),
            specs: g.specs.iter().map(SpecDto::from).collect(),
        }
    }
}

impl From<GroupDto> for Group {
    fn from(d: GroupDto) -> Self {
        Self {
            name: d.name,
            file: d.file,
            line: d.line,
            children: d.children.into_iter().map(Into::into).collect(),
            specs: d.specs.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<&Spec> for SpecDto {
    fn from(s: &Spec) -> Self {
        Self {
            name: s.name.clone(),
            file: s.file.clone(),
            line: s.line,
            language: s.language.into(),
            status: s.status.into(),
        }
    }
}

impl From<SpecDto> for Spec {
    fn from(d: SpecDto) -> Self {
        Self {
            name: d.name,
            file: d.file,
            line: d.line,
            language: d.language.into(),
            status: d.status.into(),
        }
    }
}

impl From<Language> for LanguageDto {
    fn from(value: Language) -> Self {
        match value {
            Language::Rust => Self::Rust,
            Language::TypeScript => Self::TypeScript,
        }
    }
}

impl From<LanguageDto> for Language {
    fn from(value: LanguageDto) -> Self {
        match value {
            LanguageDto::Rust => Self::Rust,
            LanguageDto::TypeScript => Self::TypeScript,
        }
    }
}

impl From<SpecStatus> for SpecStatusDto {
    fn from(value: SpecStatus) -> Self {
        match value {
            SpecStatus::Active => Self::Active,
            SpecStatus::Skipped => Self::Skipped,
        }
    }
}

impl From<SpecStatusDto> for SpecStatus {
    fn from(value: SpecStatusDto) -> Self {
        match value {
            SpecStatusDto::Active => Self::Active,
            SpecStatusDto::Skipped => Self::Skipped,
        }
    }
}

impl From<&Diagnostic> for DiagnosticDto {
    fn from(d: &Diagnostic) -> Self {
        Self {
            level: d.level.into(),
            code: d.code.into(),
            message: d.message.clone(),
            file: d.file.clone(),
            line: d.line,
        }
    }
}

impl From<DiagnosticDto> for Diagnostic {
    fn from(d: DiagnosticDto) -> Self {
        Self {
            level: d.level.into(),
            code: d.code.into(),
            message: d.message,
            file: d.file,
            line: d.line,
        }
    }
}

impl From<DiagnosticLevel> for DiagnosticLevelDto {
    fn from(value: DiagnosticLevel) -> Self {
        match value {
            DiagnosticLevel::Warning => Self::Warning,
            DiagnosticLevel::Error => Self::Error,
        }
    }
}

impl From<DiagnosticLevelDto> for DiagnosticLevel {
    fn from(value: DiagnosticLevelDto) -> Self {
        match value {
            DiagnosticLevelDto::Warning => Self::Warning,
            DiagnosticLevelDto::Error => Self::Error,
        }
    }
}

impl From<DiagnosticCode> for DiagnosticCodeDto {
    fn from(value: DiagnosticCode) -> Self {
        match value {
            DiagnosticCode::NestingTooDeep => Self::NestingTooDeep,
            DiagnosticCode::EmptyName => Self::EmptyName,
            DiagnosticCode::DynamicTestName => Self::DynamicTestName,
            DiagnosticCode::GitRevisionUnavailable => Self::GitRevisionUnavailable,
            DiagnosticCode::ParseError => Self::ParseError,
        }
    }
}

impl From<DiagnosticCodeDto> for DiagnosticCode {
    fn from(value: DiagnosticCodeDto) -> Self {
        match value {
            DiagnosticCodeDto::NestingTooDeep => Self::NestingTooDeep,
            DiagnosticCodeDto::EmptyName => Self::EmptyName,
            DiagnosticCodeDto::DynamicTestName => Self::DynamicTestName,
            DiagnosticCodeDto::GitRevisionUnavailable => Self::GitRevisionUnavailable,
            DiagnosticCodeDto::ParseError => Self::ParseError,
        }
    }
}

impl From<&Stats> for StatsDto {
    fn from(s: &Stats) -> Self {
        Self {
            total_specs: s.total_specs,
            warnings: s.warnings,
            languages: s.languages.into(),
        }
    }
}

impl From<StatsDto> for Stats {
    fn from(d: StatsDto) -> Self {
        Self {
            total_specs: d.total_specs,
            warnings: d.warnings,
            languages: d.languages.into(),
        }
    }
}

impl From<Languages> for LanguagesDto {
    fn from(value: Languages) -> Self {
        Self {
            rust: value.rust,
            typescript: value.typescript,
        }
    }
}

impl From<LanguagesDto> for Languages {
    fn from(value: LanguagesDto) -> Self {
        Self {
            rust: value.rust,
            typescript: value.typescript,
        }
    }
}
