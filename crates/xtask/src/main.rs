use std::process::ExitCode;

use xshell::{Shell, cmd};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let subcommand = args.first().map_or("help", String::as_str);

    let sh = match Shell::new() {
        Ok(sh) => sh,
        Err(err) => {
            eprintln!("xtask: シェル初期化失敗: {err}");
            return ExitCode::FAILURE;
        }
    };

    let result = match subcommand {
        "fmt" => fmt(&sh, false),
        "fmt-check" => fmt(&sh, true),
        "lint" => lint(&sh),
        "test" => test(&sh),
        "build" => build(&sh),
        "deny" => deny(&sh),
        "ci" => ci(&sh),
        "help" | "--help" | "-h" => {
            print_help();
            return ExitCode::SUCCESS;
        }
        other => {
            eprintln!("xtask: 未知のサブコマンド `{other}`");
            print_help();
            return ExitCode::FAILURE;
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("xtask: {err}");
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    eprintln!("xtask — Spectoru 開発タスクランナー");
    eprintln!();
    eprintln!("Usage: cargo xtask <subcommand>");
    eprintln!();
    eprintln!("Subcommands:");
    eprintln!("  fmt        rustfmt をワークスペース全体に適用");
    eprintln!("  fmt-check  rustfmt の差分検査（変更しない、CI向け）");
    eprintln!("  lint       cargo clippy --workspace --all-targets -- -D warnings");
    eprintln!("  test       cargo test --workspace");
    eprintln!("  build      cargo build --release -p spectoru");
    eprintln!("  deny       cargo deny check（advisories / licenses / bans / sources）");
    eprintln!("  ci         fmt-check + lint + test を順に実行");
}

fn fmt(sh: &Shell, check: bool) -> xshell::Result<()> {
    if check {
        cmd!(sh, "cargo fmt --all -- --check").run()
    } else {
        cmd!(sh, "cargo fmt --all").run()
    }
}

fn lint(sh: &Shell) -> xshell::Result<()> {
    cmd!(
        sh,
        "cargo clippy --workspace --all-targets --all-features -- -D warnings"
    )
    .run()
}

fn test(sh: &Shell) -> xshell::Result<()> {
    cmd!(sh, "cargo test --workspace --all-features").run()
}

fn build(sh: &Shell) -> xshell::Result<()> {
    cmd!(sh, "cargo build --release -p spectoru").run()
}

fn deny(sh: &Shell) -> xshell::Result<()> {
    cmd!(sh, "cargo deny check").run()
}

fn ci(sh: &Shell) -> xshell::Result<()> {
    fmt(sh, true)?;
    lint(sh)?;
    test(sh)?;
    Ok(())
}
