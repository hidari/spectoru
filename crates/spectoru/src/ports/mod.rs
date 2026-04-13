//! library-contract の境界を定義する trait 群。
//!
//! 各 trait は「ドメインが必要とする最小 API」のみを宣言し、外部 crate 由来の
//! 型を一切晒さない。実装は `adapters/` 配下にのみ存在する。
//!
//! このモジュール内のファイルは外部 crate を `use` してはならない。
//! `extern crate` を取り込んでよいのは `adapters/` 配下だけ、という規律を
//! コードレビューと grep ベースのアーキテクチャテストで担保する。

pub mod cli_parser;
pub mod file_walker;
pub mod git_provider;
pub mod json_codec;
pub mod rust_parser;
pub mod template_engine;
pub mod toml_codec;
pub mod ts_parser;
