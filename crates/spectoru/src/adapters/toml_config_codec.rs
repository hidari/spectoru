//! `toml` crate による [`TomlCodec`](crate::ports::toml_codec::TomlCodec) 実装。
//!
//! serde 派生はこのファイル内の DTO 群に閉じ込め、`core::config` のドメイン型は
//! TOML 形式に依存しない。
//!
//! DTO はすべて `deny_unknown_fields` を付ける。設定ファイルのキー名を打ち間違えた
//! ときに黙って無視されると、利用者は「設定したのに効かない」という最も分かり
//! にくい形の失敗に出会うことになる。

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::core::config::{LintConfig, ProjectConfig, SourceConfig, SourceKind, SpectoruConfig};
use crate::error::SpectoruError;
use crate::ports::toml_codec::TomlCodec;

#[derive(Debug, Default, Clone, Copy)]
pub struct TomlConfigCodec;

impl TomlCodec for TomlConfigCodec {
    fn decode_config(&self, path: &Path, source: &str) -> Result<SpectoruConfig, SpectoruError> {
        let dto: ConfigDto = toml::from_str(source).map_err(|error| SpectoruError::TomlParse {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

        validate(&dto).map_err(|message| SpectoruError::TomlParse {
            path: path.to_path_buf(),
            message,
        })?;

        Ok(dto.into())
    }
}

/// 型としては通るが設定として意味をなさない値を弾く。
///
/// いずれも「書いたのに何も起きない」形の失敗につながるため、
/// 黙って受け入れるより明示的に落とす方が親切になる。
fn validate(dto: &ConfigDto) -> Result<(), String> {
    if dto.sources.is_empty() {
        return Err("[[sources]] が1つも定義されていない".to_string());
    }
    for source in &dto.sources {
        if source.paths.is_empty() {
            return Err(format!("source `{}` の paths が空", source.name));
        }
    }
    if dto.lint.max_depth == 0 {
        return Err("lint.max_depth は 1 以上である必要がある".to_string());
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigDto {
    project: ProjectDto,
    #[serde(default)]
    sources: Vec<SourceDto>,
    #[serde(default)]
    lint: LintDto,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectDto {
    name: String,
    #[serde(default)]
    repository: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceDto {
    name: String,
    kind: SourceKindDto,
    paths: Vec<PathBuf>,
    #[serde(default)]
    exclude: Vec<PathBuf>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SourceKindDto {
    Rust,
    Vitest,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, default)]
struct LintDto {
    max_depth: usize,
}

impl Default for LintDto {
    fn default() -> Self {
        // デフォルト値の出どころはドメイン側の 1 箇所に保つ。
        Self {
            max_depth: LintConfig::default().max_depth,
        }
    }
}

impl From<ConfigDto> for SpectoruConfig {
    fn from(dto: ConfigDto) -> Self {
        Self {
            project: ProjectConfig {
                name: dto.project.name,
                repository: dto.project.repository,
            },
            sources: dto.sources.into_iter().map(Into::into).collect(),
            lint: LintConfig {
                max_depth: dto.lint.max_depth,
            },
        }
    }
}

impl From<SourceDto> for SourceConfig {
    fn from(dto: SourceDto) -> Self {
        Self {
            name: dto.name,
            kind: dto.kind.into(),
            paths: dto.paths,
            exclude: dto.exclude,
        }
    }
}

impl From<SourceKindDto> for SourceKind {
    fn from(dto: SourceKindDto) -> Self {
        match dto {
            SourceKindDto::Rust => Self::Rust,
            SourceKindDto::Vitest => Self::Vitest,
        }
    }
}
