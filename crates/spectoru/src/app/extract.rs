//! 設定ファイルを起点にソースを走査し、中間表現を組み立てるユースケース。

use std::path::{Path, PathBuf};

use crate::core::config::{SourceConfig, SourceKind};
use crate::core::ir::{
    Diagnostic, DiagnosticCode, DiagnosticLevel, IntermediateRepresentation, ProjectMeta, Source,
};
use crate::core::lint::validate_sources;
use crate::core::stats::compute_stats;
use crate::core::tree::prune_empty_groups;
use crate::error::SpectoruError;
use crate::ports::clock::Clock;
use crate::ports::file_walker::FileWalker;
use crate::ports::git_provider::GitProvider;
use crate::ports::rust_parser::{ParsedFile, RustParser};
use crate::ports::toml_codec::TomlCodec;
use crate::ports::ts_parser::TsParser;

const RUST_EXTENSIONS: &[&str] = &["rs"];
const VITEST_EXTENSIONS: &[&str] = &["ts", "tsx"];

/// 抽出に必要なポートの束。書き込み系ポートは持たない。
pub struct Extractor<'a> {
    pub config_codec: &'a dyn TomlCodec,
    pub walker: &'a dyn FileWalker,
    pub rust_parser: &'a dyn RustParser,
    pub ts_parser: &'a dyn TsParser,
    pub git: &'a dyn GitProvider,
    pub clock: &'a dyn Clock,
}

impl Extractor<'_> {
    /// `config_path` の設定に従って中間表現を組み立てる。
    ///
    /// 設定に書かれた `paths` は設定ファイルのあるディレクトリからの相対パスとして
    /// 解決し、IR に載るファイルパスも同じ基準に揃える。こうすることで
    /// 生成されたサイトのパス表記が、利用者がリポジトリルートで見る表記と一致する。
    ///
    /// `revision` を明示した場合は git を参照しない。shallow clone やコンテナ内
    /// ビルドなど、git メタデータが手に入らない CI 向けの逃げ道になる。
    ///
    /// 個々のファイルの読み込み・解釈の失敗は診断として記録し、走査は続行する。
    /// 1 ファイルの問題で全体が止まると、他の仕様まで見えなくなってしまうため。
    pub fn extract(
        &self,
        config_path: &Path,
        revision: Option<&str>,
    ) -> Result<IntermediateRepresentation, SpectoruError> {
        let raw_config = self.walker.read_to_string(config_path)?;
        let config = self.config_codec.decode_config(config_path, &raw_config)?;
        let base = config_path.parent().unwrap_or_else(|| Path::new(""));

        let mut diagnostics = Vec::new();
        let revision = self.resolve_revision(base, revision, &mut diagnostics);

        let mut sources = Vec::new();
        for source_config in &config.sources {
            sources.push(self.extract_source(base, source_config, &mut diagnostics)?);
        }

        diagnostics.extend(validate_sources(&sources, config.lint.max_depth));
        let stats = compute_stats(&sources, &diagnostics);

        Ok(IntermediateRepresentation {
            project: ProjectMeta {
                name: config.project.name,
                repository: config.project.repository,
                revision,
                extracted_at: self.clock.now_iso8601(),
            },
            sources,
            diagnostics,
            stats,
        })
    }

    fn resolve_revision(
        &self,
        base: &Path,
        override_revision: Option<&str>,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<String> {
        if let Some(revision) = override_revision {
            return Some(revision.to_string());
        }

        let resolved = self.git.current_revision(base);
        if resolved.is_none() {
            diagnostics.push(Diagnostic {
                level: DiagnosticLevel::Warning,
                code: DiagnosticCode::GitRevisionUnavailable,
                message: "git revision を取得できなかった（--revision で明示できる）".to_string(),
                file: None,
                line: None,
            });
        }
        resolved
    }

    fn extract_source(
        &self,
        base: &Path,
        config: &SourceConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Source, SpectoruError> {
        let roots: Vec<PathBuf> = config.paths.iter().map(|path| base.join(path)).collect();
        let discovered = self.walker.walk(&roots, extensions_for(config.kind))?;

        let mut groups = Vec::new();
        for file in &discovered {
            let absolute = file.root.join(&file.relative);
            let display = display_path(base, &absolute);

            let Some(contents) = self.read_source(&absolute, &display, diagnostics) else {
                continue;
            };
            let Some(parsed) = self.parse(config.kind, &display, &contents, diagnostics) else {
                continue;
            };

            diagnostics.extend(parsed.diagnostics);
            groups.push(parsed.group);
        }

        Ok(Source {
            name: config.name.clone(),
            // テストを含まないファイルはサイトに出さない。
            groups: prune_empty_groups(groups),
        })
    }

    fn read_source(
        &self,
        absolute: &Path,
        display: &Path,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<String> {
        match self.walker.read_to_string(absolute) {
            Ok(contents) => Some(contents),
            Err(error) => {
                diagnostics.push(file_diagnostic(
                    DiagnosticCode::FileUnreadable,
                    display,
                    &error,
                ));
                None
            }
        }
    }

    fn parse(
        &self,
        kind: SourceKind,
        display: &Path,
        contents: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<ParsedFile> {
        let parsed = match kind {
            SourceKind::Rust => self.rust_parser.parse_file(display, contents),
            SourceKind::Vitest => self.ts_parser.parse_file(display, contents),
        };

        match parsed {
            Ok(parsed) => Some(parsed),
            Err(error) => {
                diagnostics.push(file_diagnostic(DiagnosticCode::ParseError, display, &error));
                None
            }
        }
    }
}

fn extensions_for(kind: SourceKind) -> &'static [&'static str] {
    match kind {
        SourceKind::Rust => RUST_EXTENSIONS,
        SourceKind::Vitest => VITEST_EXTENSIONS,
    }
}

/// IR に載せるファイルパス。設定ファイルのあるディレクトリからの相対にする。
fn display_path(base: &Path, absolute: &Path) -> PathBuf {
    absolute
        .strip_prefix(base)
        .unwrap_or(absolute)
        .to_path_buf()
}

/// ファイル単位の失敗を、走査を止めずに伝えるための診断。
///
/// レベルが error なのは、そのファイルの仕様がサイトから丸ごと欠落するため。
/// 書き方の問題（warning）とは質的に異なる。
fn file_diagnostic(code: DiagnosticCode, display: &Path, error: &SpectoruError) -> Diagnostic {
    Diagnostic {
        level: DiagnosticLevel::Error,
        code,
        message: error.to_string(),
        file: Some(display.to_path_buf()),
        line: None,
    }
}
