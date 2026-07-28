//! 生の引数列からドメイン値型 `CliCommand` への変換ポート。
//!
//! clap のような外部 crate を入れ替えても application 層が壊れないよう、
//! 戻り値は core が知る純値型のみで構成する。

use std::path::PathBuf;

use crate::error::SpectoruError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    Build {
        config: PathBuf,
        out: PathBuf,
        strict: bool,
        revision: Option<String>,
    },
    Extract {
        config: PathBuf,
        out: PathBuf,
        strict: bool,
        revision: Option<String>,
    },
    Render {
        fragments: Vec<PathBuf>,
        out: PathBuf,
    },
    /// `revision` を受け取るのは、抽出を伴う以上 git が無い環境では
    /// `git_revision_unavailable` の警告が出るため。`--strict` を使う CI が
    /// 環境差で落ちないよう、他の抽出系コマンドと同じ逃げ道を用意する。
    Lint {
        config: PathBuf,
        strict: bool,
        revision: Option<String>,
    },
    /// `--help` / `--version` のように、テキストを表示して正常終了する要求。
    ///
    /// 「使い方を知りたい」も利用者の意図の 1 つなので、失敗ではなく
    /// コマンドとして表現する。文面は CLI ライブラリが組み立てたものをそのまま運ぶ。
    Print { text: String },
}

pub trait CliParser: Send + Sync {
    /// `args` は argv 相当（先頭は実行ファイル名）。
    fn parse(&self, args: &[String]) -> Result<CliCommand, SpectoruError>;
}
