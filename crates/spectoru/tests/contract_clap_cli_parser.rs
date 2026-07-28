//! `ClapCliParser` の契約テスト。
//!
//! 引数の綴りと既定値は利用者との約束そのもの。引数パーサを差し替えても
//! ここに書かれた解釈は変わってはならない。

#![allow(non_snake_case)]

use std::path::PathBuf;

use spectoru::adapters::clap_cli_parser::ClapCliParser;
use spectoru::error::SpectoruError;
use spectoru::ports::cli_parser::{CliCommand, CliParser};

/// argv 相当（先頭は実行ファイル名）を組み立てて解釈する。
fn parse(args: &[&str]) -> Result<CliCommand, SpectoruError> {
    let with_program_name: Vec<String> = std::iter::once("spectoru")
        .chain(args.iter().copied())
        .map(str::to_string)
        .collect();
    ClapCliParser.parse(&with_program_name)
}

fn parsed(args: &[&str]) -> CliCommand {
    parse(args).unwrap_or_else(|e| panic!("parse {args:?}: {e}"))
}

#[test]
fn buildは設定と出力先とstrictとrevisionを受け取る() {
    assert_eq!(
        parsed(&[
            "build",
            "--config",
            "custom.toml",
            "--out",
            "site",
            "--strict",
            "--revision",
            "abc1234",
        ]),
        CliCommand::Build {
            config: PathBuf::from("custom.toml"),
            out: PathBuf::from("site"),
            strict: true,
            revision: Some("abc1234".to_string()),
        }
    );
}

#[test]
fn buildは引数無しでも既定値で動く() {
    // 最も普通の使い方が `spectoru build` だけで済むこと。
    assert_eq!(
        parsed(&["build"]),
        CliCommand::Build {
            config: PathBuf::from("spec-site.toml"),
            out: PathBuf::from("dist"),
            strict: false,
            revision: None,
        }
    );
}

#[test]
fn extractの出力先の既定値はフラグメントのファイル名になる() {
    assert_eq!(
        parsed(&["extract"]),
        CliCommand::Extract {
            config: PathBuf::from("spec-site.toml"),
            out: PathBuf::from("spec-fragment.json"),
            strict: false,
            revision: None,
        }
    );
}

#[test]
fn renderは複数のフラグメントを順序を保って受け取る() {
    assert_eq!(
        parsed(&[
            "render",
            "--fragments",
            "backend.json",
            "frontend.json",
            "monitoring.json",
        ]),
        CliCommand::Render {
            fragments: vec![
                PathBuf::from("backend.json"),
                PathBuf::from("frontend.json"),
                PathBuf::from("monitoring.json"),
            ],
            out: PathBuf::from("dist"),
        }
    );
}

#[test]
fn renderはフラグメントの指定が必須() {
    assert!(matches!(
        parse(&["render"]),
        Err(SpectoruError::CliArgs { .. })
    ));
}

#[test]
fn lintは設定とstrictとrevisionを受け取る() {
    // 抽出を伴う以上 git が無い環境では警告が出るため、他の抽出系コマンドと
    // 同じ逃げ道を持たせている。
    assert_eq!(
        parsed(&["lint", "--strict", "--revision", "abc1234"]),
        CliCommand::Lint {
            config: PathBuf::from("spec-site.toml"),
            strict: true,
            revision: Some("abc1234".to_string()),
        }
    );
}

#[test]
fn helpは失敗ではなく表示要求として返る() {
    let CliCommand::Print { text } = parsed(&["--help"]) else {
        panic!("expected Print");
    };
    assert!(text.contains("spectoru"), "got {text}");
    assert!(text.contains("build"), "got {text}");
}

#[test]
fn versionも表示要求として返る() {
    let CliCommand::Print { text } = parsed(&["--version"]) else {
        panic!("expected Print");
    };
    assert!(text.contains(env!("CARGO_PKG_VERSION")), "got {text}");
}

#[test]
fn サブコマンドが無ければ使い方を表示する() {
    assert!(matches!(parsed(&[]), CliCommand::Print { .. }));
}

#[test]
fn 未知のサブコマンドはCliArgsエラーになる() {
    assert!(matches!(
        parse(&["publish"]),
        Err(SpectoruError::CliArgs { .. })
    ));
}

#[test]
fn 未知のフラグはCliArgsエラーになる() {
    assert!(matches!(
        parse(&["build", "--unknown"]),
        Err(SpectoruError::CliArgs { .. })
    ));
}

#[test]
fn 値が必要なフラグに値が無ければCliArgsエラーになる() {
    assert!(matches!(
        parse(&["build", "--config"]),
        Err(SpectoruError::CliArgs { .. })
    ));
}
