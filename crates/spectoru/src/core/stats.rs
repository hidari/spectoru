//! IR からの集計（純関数）。
//!
//! `compute_stats` は spec 数・言語別内訳・警告数を 1 パスで計算する。
//! skipped 状態の spec も `total_specs` に含む（仕様文として存在しているため）。

use crate::core::ir::{Diagnostic, DiagnosticLevel, Group, Language, Stats};

/// `groups` ツリーと `diagnostics` から集計値を計算する純関数。
#[must_use]
pub fn compute_stats(groups: &[Group], diagnostics: &[Diagnostic]) -> Stats {
    let mut stats = Stats::default();
    for group in groups {
        accumulate(group, &mut stats);
    }
    stats.warnings = diagnostics
        .iter()
        .filter(|d| d.level == DiagnosticLevel::Warning)
        .count();
    stats
}

fn accumulate(group: &Group, stats: &mut Stats) {
    for spec in &group.specs {
        stats.total_specs += 1;
        match spec.language {
            Language::Rust => stats.languages.rust += 1,
            Language::TypeScript => stats.languages.typescript += 1,
        }
    }
    for child in &group.children {
        accumulate(child, stats);
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::core::ir::{DiagnosticCode, Languages, Spec, SpecStatus};

    fn group(name: &str) -> Group {
        Group {
            name: name.to_string(),
            file: PathBuf::from("foo.rs"),
            line: None,
            children: vec![],
            specs: vec![],
        }
    }

    fn spec(name: &str, language: Language, status: SpecStatus) -> Spec {
        Spec {
            name: name.to_string(),
            file: PathBuf::from("foo.rs"),
            line: 1,
            language,
            status,
        }
    }

    fn warning(code: DiagnosticCode) -> Diagnostic {
        Diagnostic {
            level: DiagnosticLevel::Warning,
            code,
            message: String::new(),
            file: None,
            line: None,
        }
    }

    fn error(code: DiagnosticCode) -> Diagnostic {
        Diagnostic {
            level: DiagnosticLevel::Error,
            code,
            message: String::new(),
            file: None,
            line: None,
        }
    }

    #[test]
    fn 何もないグループ列はゼロ集計を返す() {
        assert_eq!(compute_stats(&[], &[]), Stats::default());
    }

    #[test]
    fn 単一spec数を正しくカウントする() {
        let mut g = group("foo.rs");
        g.specs
            .push(spec("テスト", Language::Rust, SpecStatus::Active));
        let stats = compute_stats(&[g], &[]);
        assert_eq!(stats.total_specs, 1);
        assert_eq!(
            stats.languages,
            Languages {
                rust: 1,
                typescript: 0,
            }
        );
    }

    #[test]
    fn 言語別の内訳を正しく分離する() {
        let mut g = group("foo.rs");
        g.specs.push(spec("a", Language::Rust, SpecStatus::Active));
        g.specs.push(spec("b", Language::Rust, SpecStatus::Active));
        g.specs
            .push(spec("c", Language::TypeScript, SpecStatus::Active));
        let stats = compute_stats(&[g], &[]);
        assert_eq!(stats.total_specs, 3);
        assert_eq!(
            stats.languages,
            Languages {
                rust: 2,
                typescript: 1,
            }
        );
    }

    #[test]
    fn ネストしたグループのspecも合計に含める() {
        let mut leaf = group("inner");
        leaf.specs
            .push(spec("nested", Language::Rust, SpecStatus::Active));
        let mut root = group("foo.rs");
        root.specs
            .push(spec("top", Language::Rust, SpecStatus::Active));
        root.children.push(leaf);
        let stats = compute_stats(&[root], &[]);
        assert_eq!(stats.total_specs, 2);
    }

    #[test]
    fn skipped状態のspecもtotal_specsに含める() {
        let mut g = group("foo.rs");
        g.specs
            .push(spec("active", Language::Rust, SpecStatus::Active));
        g.specs
            .push(spec("ignored", Language::Rust, SpecStatus::Skipped));
        let stats = compute_stats(&[g], &[]);
        assert_eq!(stats.total_specs, 2);
        assert_eq!(stats.languages.rust, 2);
    }

    #[test]
    fn diagnosticsのwarning数だけがwarningsに反映される() {
        let diags = vec![
            warning(DiagnosticCode::EmptyName),
            warning(DiagnosticCode::NestingTooDeep),
            error(DiagnosticCode::ParseError),
        ];
        let stats = compute_stats(&[], &diags);
        assert_eq!(stats.warnings, 2);
    }
}
