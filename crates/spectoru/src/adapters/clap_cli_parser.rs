//! `clap` による [`CliParser`](crate::ports::cli_parser::CliParser) 実装。
//!
//! clap の型はこのファイルの外に出ない。引数の宣言もここに閉じているため、
//! 別の引数パーサへ差し替えても `tests/contract_clap_cli_parser.rs` を通れば
//! 合成側（`cli`）は影響を受けない。

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::error::SpectoruError;
use crate::ports::cli_parser::{CliCommand, CliParser};

/// 引数を省略したときの既定値。`spectoru build` だけで動くのが最も普通の使い方。
const DEFAULT_CONFIG: &str = "spec-site.toml";
const DEFAULT_SITE_OUT: &str = "dist";
const DEFAULT_FRAGMENT_OUT: &str = "spec-fragment.json";

#[derive(Debug, Default, Clone, Copy)]
pub struct ClapCliParser;

impl CliParser for ClapCliParser {
    fn parse(&self, args: &[String]) -> Result<CliCommand, SpectoruError> {
        match Cli::try_parse_from(args) {
            Ok(cli) => Ok(cli.command.into()),
            // `--help` / `--version` は失敗ではない。clap が組み立てた文面を
            // そのまま運び、表示と終了コードの決定は合成側に委ねる。
            Err(error) if is_display_request(&error) => Ok(CliCommand::Print {
                text: error.render().to_string(),
            }),
            Err(error) => Err(SpectoruError::CliArgs {
                message: error.to_string(),
            }),
        }
    }
}

fn is_display_request(error: &clap::Error) -> bool {
    matches!(
        error.kind(),
        clap::error::ErrorKind::DisplayHelp
            | clap::error::ErrorKind::DisplayVersion
            | clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    )
}

#[derive(Debug, Parser)]
#[command(
    name = "spectoru",
    version,
    about = "テストを仕様として可視化する静的サイトジェネレータ"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// ソースをパースして静的サイトを生成する（extract + render）
    Build {
        #[arg(long, default_value = DEFAULT_CONFIG)]
        config: PathBuf,
        #[arg(long, default_value = DEFAULT_SITE_OUT)]
        out: PathBuf,
        /// 警告が 1 件でもあれば失敗させる
        #[arg(long)]
        strict: bool,
        /// git から取得せず revision を明示する
        #[arg(long)]
        revision: Option<String>,
    },
    /// JSON フラグメントだけを出力する（複数リポジトリ集約用）
    Extract {
        #[arg(long, default_value = DEFAULT_CONFIG)]
        config: PathBuf,
        #[arg(long, default_value = DEFAULT_FRAGMENT_OUT)]
        out: PathBuf,
        #[arg(long)]
        strict: bool,
        #[arg(long)]
        revision: Option<String>,
    },
    /// 複数の JSON フラグメントから静的サイトを生成する
    Render {
        #[arg(long, num_args = 1.., required = true)]
        fragments: Vec<PathBuf>,
        #[arg(long, default_value = DEFAULT_SITE_OUT)]
        out: PathBuf,
    },
    /// 規約チェックのみ行う（CI の品質ゲート向け）
    Lint {
        #[arg(long, default_value = DEFAULT_CONFIG)]
        config: PathBuf,
        #[arg(long)]
        strict: bool,
        #[arg(long)]
        revision: Option<String>,
    },
}

impl From<Command> for CliCommand {
    fn from(command: Command) -> Self {
        match command {
            Command::Build {
                config,
                out,
                strict,
                revision,
            } => Self::Build {
                config,
                out,
                strict,
                revision,
            },
            Command::Extract {
                config,
                out,
                strict,
                revision,
            } => Self::Extract {
                config,
                out,
                strict,
                revision,
            },
            Command::Render { fragments, out } => Self::Render { fragments, out },
            Command::Lint {
                config,
                strict,
                revision,
            } => Self::Lint {
                config,
                strict,
                revision,
            },
        }
    }
}
