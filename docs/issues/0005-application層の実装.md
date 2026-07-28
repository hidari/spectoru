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

## 決定事項

### モジュールの切り方

`extract` / `fragment` / `render` の 3 つにした。`build` = extract + render、
`extract` サブコマンド = extract + fragment.save、`render` サブコマンド =
fragment.load_all + render という合成になり、経路ごとに専用のコードを持たない。

`lint` に対応するモジュールは作らない。その実体は「extract して
`core::lint::fails_quality_gate` に掛ける」以上のものではなく、層を足すより
CLI 側で合成する方が単純だから。

### ファイル単位の失敗は診断にして続行する

構文エラーも読み込み失敗も、そのファイルを診断に記録して走査を続ける。
1 ファイルの問題で全体が止まると無関係な他の仕様まで見えなくなり、CI としても
全部まとめて報告される方が修正の回数が減る。

ただし設定ファイル自体が読めない・壊れている場合と、探索パスが存在しない場合は
即座に失敗させる。抽出対象が確定しないという意味で質的に異なり、黙って空を返すと
打ち間違いが「なぜか何も出ない」形でしか現れないため。

### warning と error の判断基準

「サイトが正しく作れるか」で分ける。書き方の問題は warning、ソースを解釈できず
仕様が丸ごと欠落するものは error。`fails_quality_gate` は error を常に落とし、
warning は `--strict` のときだけ落とす（`core::lint` の純関数）。

### パスの基準

設定の `paths` は設定ファイルのあるディレクトリからの相対として解決し、IR に
載るファイルパスも同じ基準に揃える。生成サイトのパス表記が、利用者が
リポジトリルートで見る表記と一致する。

### フラグメントのエラー帰属（0003 からの持ち越し）

`JsonCodec::decode` は汎用の文字列 codec でパスを持たないため、
`SpectoruError::Fragment { path, message }` を追加し、`FragmentStore::load` が
読み込み中のパスを添えて包み直す。`render --fragments a.json b.json` で
どのファイルが壊れているか型として伝わる。

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

## 併せて対応

`DiagnosticCode::FileUnreadable` を追加した。探索では見つかったのに読めない
ファイルを `ParseError` に混ぜると、原因が構文なのか I/O なのか区別できない。

JSON 契約テストに、全 `DiagnosticCode` / `DiagnosticLevel` のラウンドトリップ
検証を追加した。網羅性は `match` で担保しており、変異体を増やすとテストが
コンパイルエラーになる。

## 完了条件

- [x] `extract` / `render` / `lint` が実装され、Fake ベースの統合テストを持つ
- [x] `tests/architecture.rs` が `app/` も走査対象にしている
- [x] `cargo xtask ci` が緑
