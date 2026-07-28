# [Phase 6] application 層の実装

親: [0001](./0001-v0.1リリースまでのロードマップ.md)
依存: [0003](./0003-IOアダプタ群の実装.md), [0004](./0004-パーサアダプタの実装.md)

## 目的

ポートを合成して `extract` / `render` / `lint` のユースケースを組み立てる。
このプロジェクトで最もテスト価値が高い層。

## 構成

`crates/spectoru/src/app/` に配置し、外部 crate は一切使わず trait 経由でのみ
I/O を行う（`tests/architecture.rs` の走査対象に追加する）。

```
app/
  extract.rs   設定 → ファイル探索 → パース → lint → 集計 → IR
  render.rs    IR 群 → HTML
  lint.rs      設定 → extract → 診断の要約と終了判定
```

依存はコンストラクタ引数として受け取る。グローバル状態やシングルトンは作らない。

## `extract` の処理順

1. `TomlCodec` で設定を読む
2. `GitProvider` で revision を解決（失敗時は `GitRevisionUnavailable` の警告を積む）
3. `Clock` で `extracted_at` を得る
4. `[[sources]]` ごとに `FileWalker` でファイルを列挙し、`kind` に応じたパーサに渡す
5. `core::lint::validate_sources` で診断を収集し、パーサ由来の診断と結合する
6. `core::stats::compute_stats` で集計する
7. `JsonCodec` でエンコードし `FileWriter` で書き出す

診断の順序は決定的にする（source 宣言順 → ファイルパス順 → 行番号順）。

## 検討事項

- **`build` = extract + render** をどう表現するか。`extract` を IR を返す関数に
  し、ファイル書き出しは呼び出し側の責務にすると `build` が自然に書ける
- **パースエラーの扱い**: 1 ファイルの構文エラーで全体を失敗させるか、診断に
  記録して続行するか。CLI の品質ゲートとしては続行して最後にまとめて報告する方が
  使いやすい
- **`--strict` の判定位置**: 診断の集計は app 層、exit code への変換は CLI 層
- **フラグメントのエラー帰属**: `JsonCodec::decode` は汎用の文字列 codec であり
  `JsonDecode { message }` にパスを持たない。`render --fragments a.json b.json` で
  どのファイルが壊れているか伝えるのは app 層の責務になる。読み込み中のパスを
  エラーに添える仕組みを用意する（0003 から持ち越し）

## テスト

すべてのポートに Fake 実装を用意し、実ファイルシステム・実 git に触れずに
統合テストを書く。Fake は「テスト内で内容を宣言できる単純な構造体」に留め、
呼び出し回数の検証などモックフレームワーク的な使い方はしない。

- 単一 source / 複数 source
- Rust と Vitest が混在するプロジェクト
- git revision が取得できない場合に警告が積まれる
- パースエラーを含むファイルがあっても他のファイルの抽出は続行する
- `max_depth` 違反が診断と `stats.warnings` の両方に反映される
- 同じ入力から常に同じ IR が出る（決定性）
- 空のプロジェクト（テストが 1 件も無い）

## 完了条件

- [ ] `extract` / `render` / `lint` が実装され、Fake ベースの統合テストを持つ
- [ ] `tests/architecture.rs` が `app/` も走査対象にしている
- [ ] `cargo xtask ci` が緑
