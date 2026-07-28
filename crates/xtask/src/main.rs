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
        "e2e" => e2e(&sh),
        "build" => build(&sh),
        "spec-site" => spec_site(&sh),
        "dist" => {
            let Some(target) = args.get(1) else {
                eprintln!("xtask: dist にはターゲット三つ組が必要（例: x86_64-unknown-linux-gnu）");
                return ExitCode::FAILURE;
            };
            dist(&sh, target)
        }
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
    eprintln!("  e2e        実バイナリに対する E2E テストのみ実行");
    eprintln!("  build      cargo build --release -p spectoru");
    eprintln!("  spec-site  spectoru 自身の仕様サイトを dist/ に生成（--strict）");
    eprintln!("  dist <t>   配布用アーカイブを dist-artifacts/ に作成（t: ターゲット三つ組）");
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

/// E2E だけを回す。CLI を触っているときの反復を短くするための入り口で、
/// `test` にも含まれるため CI で別途走らせる必要はない。
fn e2e(sh: &Shell) -> xshell::Result<()> {
    cmd!(sh, "cargo test -p spectoru --test e2e_cli").run()
}

fn build(sh: &Shell) -> xshell::Result<()> {
    cmd!(sh, "cargo build --release -p spectoru").run()
}

/// spectoru 自身の仕様サイトを生成する（ドッグフーディング）。
///
/// `--strict` を付けるのは、自分の仕様が自分の品質基準を満たしていることを
/// 常に保つため。想定ユースケースにそのまま当てはまる構成を自前で持っているので、
/// これが最も実効性のある回帰テストになる。
fn spec_site(sh: &Shell) -> xshell::Result<()> {
    cmd!(
        sh,
        "cargo run --quiet --release -p spectoru -- build --strict --out dist"
    )
    .run()
}

/// 配布用アーカイブを作る。
///
/// `--locked` を付けるのは、リリース成果物が Cargo.lock どおりの依存で
/// ビルドされたことを保証するため。ここで解決が変わると、テストした構成と
/// 配布した構成が別物になる。
///
/// リリースそのものは GitHub Actions が行う。このサブコマンドは「配布物を
/// 手元で組み立てて確かめる」ためのものであって、公開はしない。
fn dist(sh: &Shell, target: &str) -> xshell::Result<()> {
    cmd!(
        sh,
        "cargo build --release --locked --target {target} -p spectoru"
    )
    .run()?;

    sh.create_dir("dist-artifacts")?;
    let archive = format!("dist-artifacts/spectoru-{target}.tar.gz");
    let binary_dir = format!("target/{target}/release");

    cmd!(
        sh,
        "tar -czf {archive} -C {binary_dir} spectoru -C ../../.. README.md"
    )
    .run()?;

    eprintln!("xtask: {archive} を作成した");
    Ok(())
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
