//! spectoru の実行ファイル。
//!
//! 引数を集めて合成ルートに渡すだけにとどめ、判断は一切持たない。
//! こうすることで CLI の振る舞いはすべて `cli::run` 側でテストできる。

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    spectoru::cli::run(&args)
}
