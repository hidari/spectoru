//! 合成ルート: adapter を組み立てて application 層に注入し、結果を終了コードに変換する。
//!
//! 具象 adapter を名指しするのはこの層だけ。`app/` は trait しか知らないため、
//! ここを差し替えれば別の実装構成でも同じユースケースが動く。

use std::path::Path;
use std::process::ExitCode;

use crate::adapters::clap_cli_parser::ClapCliParser;
use crate::adapters::command_git_provider::CommandGitProvider;
use crate::adapters::fs_file_writer::FsFileWriter;
use crate::adapters::ignore_file_walker::IgnoreFileWalker;
use crate::adapters::inline_html_template::InlineHtmlTemplate;
use crate::adapters::serde_json_codec::SerdeJsonCodec;
use crate::adapters::syn_rust_parser::SynRustParser;
use crate::adapters::system_clock::SystemClock;
use crate::adapters::toml_config_codec::TomlConfigCodec;
use crate::adapters::tree_sitter_ts_parser::TreeSitterTsParser;
use crate::app::extract::Extractor;
use crate::app::fragment::FragmentStore;
use crate::app::render::Renderer;
use crate::core::ir::{Diagnostic, IntermediateRepresentation, Stats};
use crate::core::lint::fails_quality_gate;
use crate::error::SpectoruError;
use crate::ports::cli_parser::{CliCommand, CliParser};

/// 正常終了。
const EXIT_SUCCESS: u8 = 0;
/// 品質ゲートに掛かった。仕様の書き方の問題か、解釈できないソースがある。
const EXIT_QUALITY_GATE: u8 = 1;
/// spectoru が動けなかった。設定が読めない、出力できない等。
///
/// 品質ゲートと別のコードにすることで、CI 側で「仕様の問題」と
/// 「ツールが動かなかった」を区別できる。
const EXIT_EXECUTION_ERROR: u8 = 2;

#[must_use]
pub fn run(args: &[String]) -> ExitCode {
    match execute(args) {
        Ok(report) => report.finish(),
        Err(error) => {
            eprintln!("spectoru: {error}");
            ExitCode::from(EXIT_EXECUTION_ERROR)
        }
    }
}

/// コマンド実行の結果。診断の報告と終了コードの決定をここに集約する。
struct Report {
    diagnostics: Vec<Diagnostic>,
    stats: Option<Stats>,
    /// 診断を品質ゲートに掛けるか。`render` は受け取った成果物を描くだけなので
    /// 掛けない（診断は生成元のリポジトリで既に評価されている）。
    gated: bool,
    strict: bool,
}

impl Report {
    fn silent() -> Self {
        Self {
            diagnostics: Vec::new(),
            stats: None,
            gated: false,
            strict: false,
        }
    }

    fn gated(ir: &IntermediateRepresentation, strict: bool) -> Self {
        Self {
            diagnostics: ir.diagnostics.clone(),
            stats: Some(ir.stats),
            gated: true,
            strict,
        }
    }

    fn reported(projects: &[IntermediateRepresentation]) -> Self {
        Self {
            diagnostics: projects
                .iter()
                .flat_map(|project| project.diagnostics.clone())
                .collect(),
            stats: None,
            gated: false,
            strict: false,
        }
    }

    fn finish(self) -> ExitCode {
        for diagnostic in &self.diagnostics {
            eprintln!("{}", format_diagnostic(diagnostic));
        }
        if let Some(stats) = self.stats {
            eprintln!(
                "仕様 {}件 / rust {} / typescript {} / 警告 {}件",
                stats.total_specs, stats.languages.rust, stats.languages.typescript, stats.warnings
            );
        }

        if self.gated && fails_quality_gate(&self.diagnostics, self.strict) {
            return ExitCode::from(EXIT_QUALITY_GATE);
        }
        ExitCode::from(EXIT_SUCCESS)
    }
}

/// エディタや CI のアノテーションが拾える `file:line: ` 形式で出す。
fn format_diagnostic(diagnostic: &Diagnostic) -> String {
    let body = format!(
        "{}[{}]: {}",
        diagnostic.level.as_str(),
        diagnostic.code.as_str(),
        diagnostic.message
    );

    match (&diagnostic.file, diagnostic.line) {
        (Some(file), Some(line)) => format!("{}:{line}: {body}", file.display()),
        (Some(file), None) => format!("{}: {body}", file.display()),
        (None, _) => body,
    }
}

fn execute(args: &[String]) -> Result<Report, SpectoruError> {
    match ClapCliParser.parse(args)? {
        CliCommand::Print { text } => {
            print!("{text}");
            Ok(Report::silent())
        }

        CliCommand::Build {
            config,
            out,
            strict,
            revision,
        } => {
            let ir = extract(&config, revision.as_deref())?;
            let report = Report::gated(&ir, strict);
            renderer().render(&[ir], &out)?;
            Ok(report)
        }

        CliCommand::Extract {
            config,
            out,
            strict,
            revision,
        } => {
            let ir = extract(&config, revision.as_deref())?;
            let report = Report::gated(&ir, strict);
            fragment_store().save(&ir, &out)?;
            Ok(report)
        }

        CliCommand::Render { fragments, out } => {
            let projects = fragment_store().load_all(&fragments)?;
            renderer().render(&projects, &out)?;
            Ok(Report::reported(&projects))
        }

        CliCommand::Lint {
            config,
            strict,
            revision,
        } => {
            let ir = extract(&config, revision.as_deref())?;
            Ok(Report::gated(&ir, strict))
        }
    }
}

// 以下が本番構成の組み立て。adapter はいずれも状態を持たないユニット型なので、
// 束ねる構造体を置かず、必要な場所でその場で組み立てる。

fn extract(
    config: &Path,
    revision: Option<&str>,
) -> Result<IntermediateRepresentation, SpectoruError> {
    Extractor {
        config_codec: &TomlConfigCodec,
        walker: &IgnoreFileWalker,
        rust_parser: &SynRustParser,
        ts_parser: &TreeSitterTsParser,
        git: &CommandGitProvider,
        clock: &SystemClock,
    }
    .extract(config, revision)
}

fn fragment_store() -> FragmentStore<'static> {
    FragmentStore {
        json_codec: &SerdeJsonCodec,
        walker: &IgnoreFileWalker,
        writer: &FsFileWriter,
    }
}

fn renderer() -> Renderer<'static> {
    Renderer {
        template: &InlineHtmlTemplate,
        writer: &FsFileWriter,
    }
}
