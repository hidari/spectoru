# [Phase 3.5] 中間表現の project 層導入と port 拡充

親: [0001](./0001-v0.1リリースまでのロードマップ.md)

## 背景

Phase 0–3 の実装をレビューしたところ、実装に着手すると手戻りする設計の穴が
3 件見つかった。アダプタを書き始める前に潰す。

## 課題と対応

### 1. IR に project 層がなく設定ファイルと構造が噛み合わない

`spec-site.toml` は `[project]` 1 つに対して `[[sources]]` を複数持てるのに、
`IntermediateRepresentation` は `source` を単数でしか持てなかった。モノレポで
Rust バックエンドと Vitest フロントエンドを 1 つの設定から抽出できず、方向性
ドキュメントが謳う「サイドバー: プロジェクト → ソース → ファイル → グループ」の
4 階層とも一致しない。

対応: IR を `project` + `sources: Vec<Source>` の 2 階層に変更した。

- `SourceMeta` を `ProjectMeta` と `Source` に分割
- `repository` / `revision` / `extracted_at` は project 側に集約。1 つの設定
  ファイルは単一の作業ディレクトリを指すため、全 source が同じ値を共有する
- `Source` に言語種別は持たせない。言語は spec ごとの `language` が事実として
  保持しており、設定上の `kind` を複製すると食い違いうる
- `stats` も project に 1 つだけ。source 別の内訳はレンダラが都度算出する
- `SourceConfig.repository` を削除し `ProjectConfig.repository` に移動

### 2. Clock ポートと `FileWriter` ポートが存在しない

`extracted_at` を誰が生成するのか未定義だった。`SystemTime::now()` を直接
呼ぶと extract の出力が実行時刻に依存してテストが決定的でなくなる。また
`FileWalker` は読み取り専用で、`--out dist/` に書き出すポートが無かった。

対応: `ports/clock.rs` と `ports/file_writer.rs` を追加。

- `Clock::now_iso8601()` は秒精度の UTC 文字列を返す契約。ローカルタイムゾーンを
  含む表現は生成環境で内容が変わるため契約違反とする
- `FileWriter::write()` は親ディレクトリを再帰的に作成する契約。読み取りと
  書き込みを別ポートに分けることで、`lint` が「書き込む能力を持たない」ことを
  型で表現できる

adapter 実装は 0003 で行う。

### 3. 検索ライブラリの CDN 読み込みがセキュリティ方針と衝突する

方向性ドキュメントは minisearch を CDN から読み込む想定だったが、SRI なしの
外部スクリプト読み込みは、CDN または配信元パッケージが侵害された時点で閲覧者の
ブラウザ上での任意コード実行を許す。「サプライチェーン攻撃の影響範囲をアダプター層に
限定する」という方針とも矛盾する。

対応: 外部依存ゼロの自己完結 HTML とし、検索は自前実装する方針を確定。
対象はテスト名の文字列のみ、規模も数千 spec のオーダーであり、正規化トークンの
インデックスをビルド時に生成して HTML にインライン化すれば足りる。
実装は 0006 で行う。

## 併せて対応（ボーイスカウトルール）

- `SpectruConfig` → `SpectoruConfig`（`o` 欠落のタイポ）
- `ports/mod.rs` が「grep ベースのアーキテクチャテストで担保する」と書きながら
  未実装だった件。`tests/architecture.rs` を追加し、`core/` と `ports/` の
  プロダクションコードが外部 crate を `use` せず、組み込み derive しか
  使わないことを検証する。`error.rs` が `thiserror` のみに依存することも
  明示的に固定した
- 旧形式のフラグメントが黙って部分的に読めてしまわないことを契約テストで固定

## 完了条件

- [x] IR / config / stats / lint / JSON adapter が新構造で一貫している
- [x] `Clock` / `FileWriter` ポートを追加
- [x] `tests/architecture.rs` が境界違反を検出できる（検出器自体の動作も検証）
- [x] 方向性ドキュメントを 3 件すべて反映して改訂
- [x] `cargo xtask ci` が緑
