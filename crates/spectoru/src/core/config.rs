//! `spec-site.toml` を表現するドメイン値型。
//!
//! TOML 由来の生表現を `adapters::toml_codec_impl` がここで定義された純値型に
//! 変換する。これ以降の application 層は TOML 形式に依存しない。

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpectruConfig {
    pub project: ProjectConfig,
    pub sources: Vec<SourceConfig>,
    pub lint: LintConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectConfig {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceConfig {
    pub name: String,
    pub kind: SourceKind,
    pub paths: Vec<PathBuf>,
    pub repository: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceKind {
    Rust,
    Vitest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LintConfig {
    pub max_depth: usize,
}

impl Default for LintConfig {
    fn default() -> Self {
        Self { max_depth: 4 }
    }
}
