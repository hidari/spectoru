//! IR に対する純粋な検証ロジック。
//!
//! `validate_sources` / `validate_groups` は IR のツリーを走査し、ネスト深さ超過と
//! 空文字テスト名を `Diagnostic` として収集する純関数。副作用ゼロ・順序決定的。

use crate::core::ir::{Diagnostic, DiagnosticCode, DiagnosticLevel, Group, Source, Spec};

/// 診断の集合が品質ゲートを通らないかを判定する純関数。
///
/// error は常に失敗させ、warning は `strict` のときだけ失敗させる。
/// 「error」という語が意味するとおりの挙動にするのが最も驚きが少ない。
///
/// error になるのはソースを解釈できなかった場合、つまり仕様サイトが不完全に
/// なる場合に限る。ネストの深さや空のテスト名は「書き方の問題」であって
/// サイトは正しく作れるため warning にとどめ、ゲートにするかは利用者が
/// `--strict` で選ぶ。
#[must_use]
pub fn fails_quality_gate(diagnostics: &[Diagnostic], strict: bool) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| strict || diagnostic.level == DiagnosticLevel::Error)
}

/// IR 全体（全 source）を走査して lint diagnostics を返す純関数。
///
/// diagnostics は `sources` の宣言順、その中では [`validate_groups`] の順序で並ぶ。
#[must_use]
pub fn validate_sources(sources: &[Source], max_depth: usize) -> Vec<Diagnostic> {
    sources
        .iter()
        .flat_map(|source| validate_groups(&source.groups, max_depth))
        .collect()
}

/// 1 source 分の `groups` を走査して lint diagnostics を返す純関数。
///
/// `max_depth` はファイル直下のグループ自身を深さ 1 として数えたときの上限。
/// ネスト深さ違反は最初に上限を超えたグループ単位で 1 件出力される。
#[must_use]
pub fn validate_groups(groups: &[Group], max_depth: usize) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for group in groups {
        walk(group, 1, max_depth, &mut diagnostics);
    }
    diagnostics
}

fn walk(group: &Group, depth: usize, max_depth: usize, out: &mut Vec<Diagnostic>) {
    if group.name.is_empty() {
        out.push(empty_name_diagnostic(group));
    }
    if depth == max_depth + 1 {
        out.push(nesting_too_deep_diagnostic(group, depth, max_depth));
    }

    for spec in &group.specs {
        check_spec_name(spec, out);
    }

    for child in &group.children {
        walk(child, depth + 1, max_depth, out);
    }
}

fn check_spec_name(spec: &Spec, out: &mut Vec<Diagnostic>) {
    if spec.name.is_empty() {
        out.push(Diagnostic {
            level: DiagnosticLevel::Warning,
            code: DiagnosticCode::EmptyName,
            message: "Spec name is empty".to_string(),
            file: Some(spec.file.clone()),
            line: Some(spec.line),
        });
    }
}

fn empty_name_diagnostic(group: &Group) -> Diagnostic {
    Diagnostic {
        level: DiagnosticLevel::Warning,
        code: DiagnosticCode::EmptyName,
        message: "Group name is empty".to_string(),
        file: Some(group.file.clone()),
        line: group.line,
    }
}

fn nesting_too_deep_diagnostic(group: &Group, depth: usize, max_depth: usize) -> Diagnostic {
    Diagnostic {
        level: DiagnosticLevel::Warning,
        code: DiagnosticCode::NestingTooDeep,
        message: format!("Nesting depth exceeds limit ({depth} > {max_depth})"),
        file: Some(group.file.clone()),
        line: group.line,
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::core::ir::{Language, SpecStatus};

    fn group(name: &str, file: &str) -> Group {
        Group {
            name: name.to_string(),
            file: PathBuf::from(file),
            line: None,
            children: vec![],
            specs: vec![],
        }
    }

    fn spec(name: &str) -> Spec {
        Spec {
            name: name.to_string(),
            file: PathBuf::from("foo.rs"),
            line: 1,
            language: Language::Rust,
            status: SpecStatus::Active,
        }
    }

    /// `depth` 段の縦一直線にネストしたグループを作る。最深部に1つの spec を入れる。
    fn nested_chain(depth: u32) -> Group {
        assert!(depth >= 1, "depth must be >= 1");
        let mut current = group(&format!("depth{depth}"), "foo.rs");
        current.line = Some(depth);
        current.specs.push(spec("テスト"));
        for d in (1..depth).rev() {
            let mut parent = group(&format!("depth{d}"), "foo.rs");
            parent.line = Some(d);
            parent.children.push(current);
            current = parent;
        }
        current
    }

    #[test]
    fn 何の違反もなければ警告は空になる() {
        let mut g = group("foo.rs", "foo.rs");
        g.specs.push(spec("テストa"));
        assert_eq!(validate_groups(&[g], 4), vec![]);
    }

    #[test]
    fn ネスト深さがmax_depthと等しいときは警告を出さない() {
        let g = nested_chain(4);
        assert!(validate_groups(&[g], 4).is_empty());
    }

    #[test]
    fn ネスト深さがmax_depthを1段超えると最初の超過点に警告が出る() {
        let g = nested_chain(5);
        let diags = validate_groups(&[g], 4);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagnosticCode::NestingTooDeep);
        assert_eq!(diags[0].level, DiagnosticLevel::Warning);
        assert_eq!(diags[0].line, Some(5));
    }

    #[test]
    fn ネスト深さがmax_depthを大きく超えても警告は1件だけ出る() {
        let g = nested_chain(8);
        let diags = validate_groups(&[g], 4);
        assert_eq!(
            diags
                .iter()
                .filter(|d| d.code == DiagnosticCode::NestingTooDeep)
                .count(),
            1
        );
    }

    #[test]
    fn 別々の枝で深すぎる場合は枝ごとに警告が出る() {
        let mut root = group("foo.rs", "foo.rs");
        root.children.push(nested_chain(5));
        root.children.push(nested_chain(5));
        let diags = validate_groups(&[root], 4);
        assert_eq!(
            diags
                .iter()
                .filter(|d| d.code == DiagnosticCode::NestingTooDeep)
                .count(),
            2
        );
    }

    #[test]
    fn グループ名が空文字なら警告を出す() {
        let mut g = group("", "foo.rs");
        g.specs.push(spec("テスト"));
        let diags = validate_groups(&[g], 4);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagnosticCode::EmptyName);
    }

    #[test]
    fn spec名が空文字なら警告を出す() {
        let mut g = group("foo.rs", "foo.rs");
        g.specs.push(spec(""));
        let diags = validate_groups(&[g], 4);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagnosticCode::EmptyName);
        assert_eq!(diags[0].line, Some(1));
    }

    #[test]
    fn 警告は決定的な順序で並ぶ() {
        let mut g = group("foo.rs", "foo.rs");
        g.specs.push(spec(""));
        g.specs.push(spec("ok"));
        g.specs.push(spec(""));
        let diags = validate_groups(&[g], 4);
        assert_eq!(diags.len(), 2);
        assert!(diags.iter().all(|d| d.code == DiagnosticCode::EmptyName));
    }

    fn source(name: &str, groups: Vec<Group>) -> Source {
        Source {
            name: name.to_string(),
            groups,
        }
    }

    #[test]
    fn ソースが一つも無ければ警告は空になる() {
        assert_eq!(validate_sources(&[], 4), vec![]);
    }

    #[test]
    fn 全ソースの警告が集約される() {
        let mut backend = group("foo.rs", "foo.rs");
        backend.specs.push(spec(""));
        let frontend = nested_chain(5);

        let diags = validate_sources(
            &[
                source("Backend", vec![backend]),
                source("Frontend", vec![frontend]),
            ],
            4,
        );

        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].code, DiagnosticCode::EmptyName);
        assert_eq!(diags[1].code, DiagnosticCode::NestingTooDeep);
    }

    fn diagnostic(level: DiagnosticLevel) -> Diagnostic {
        Diagnostic {
            level,
            code: DiagnosticCode::ParseError,
            message: String::new(),
            file: None,
            line: None,
        }
    }

    #[test]
    fn 診断が無ければ品質ゲートを通る() {
        assert!(!fails_quality_gate(&[], false));
        assert!(!fails_quality_gate(&[], true));
    }

    #[test]
    fn errorは常に品質ゲートを落とす() {
        let diagnostics = [diagnostic(DiagnosticLevel::Error)];
        assert!(fails_quality_gate(&diagnostics, false));
        assert!(fails_quality_gate(&diagnostics, true));
    }

    #[test]
    fn warningはstrictのときだけ品質ゲートを落とす() {
        let diagnostics = [diagnostic(DiagnosticLevel::Warning)];
        assert!(!fails_quality_gate(&diagnostics, false));
        assert!(fails_quality_gate(&diagnostics, true));
    }

    #[test]
    fn warningに1件でもerrorが混ざれば落とす() {
        let diagnostics = [
            diagnostic(DiagnosticLevel::Warning),
            diagnostic(DiagnosticLevel::Error),
        ];
        assert!(fails_quality_gate(&diagnostics, false));
    }

    #[test]
    fn 警告はソースの宣言順に並ぶ() {
        let mut first = group("", "first.rs");
        first.specs.push(spec("ok"));
        let mut second = group("", "second.rs");
        second.specs.push(spec("ok"));

        let diags = validate_sources(
            &[
                source("Backend", vec![first]),
                source("Frontend", vec![second]),
            ],
            4,
        );

        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].file, Some(PathBuf::from("first.rs")));
        assert_eq!(diags[1].file, Some(PathBuf::from("second.rs")));
    }
}
