//! ユースケース層: ポートを合成して spectoru の振る舞いを組み立てる。
//!
//! `core/` `ports/` と同様、このモジュールは外部 crate を一切 `use` しない。
//! I/O はすべて trait 越しに行うため、テストでは Fake を注入して実ファイル
//! システムや実 git に触れずに全分岐を検証できる。
//!
//! 各ユースケースは必要なポートだけを保持する。`Extractor` が
//! [`FileWriter`](crate::ports::file_writer::FileWriter) を持たないのは、
//! `lint` が「書き込む能力を持たない」ことを型として表現するため。
//!
//! `lint` に対応するモジュールが無いのは意図的で、その実体は
//! 「extract して [`fails_quality_gate`](crate::core::lint::fails_quality_gate)
//! に掛ける」以上のものではない。層を足す代わりに CLI 側で合成する。

pub mod extract;
pub mod fragment;
pub mod render;
