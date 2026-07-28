//! 外部 crate を実際に `use` する唯一のレイヤ。
//!
//! ここで定義される型は `ports/` の trait を実装する具象 adapter であり、
//! application 層（`extract/` `render/` `cli/`）からは trait オブジェクト経由で
//! 注入される。各 adapter は、対応する `tests/contract_*.rs` で「ライブラリ
//! 差し替え時に守るべき契約」を検証する。

pub mod command_git_provider;
pub mod fs_file_writer;
pub mod ignore_file_walker;
pub mod serde_json_codec;
pub mod syn_rust_parser;
pub mod system_clock;
pub mod toml_config_codec;
pub mod tree_sitter_ts_parser;
