//! `ignore` crate による [`FileWalker`](crate::ports::file_walker::FileWalker) 実装。
//!
//! `.gitignore` を尊重しつつ、ビルド生成物ディレクトリを除外して対象ファイルを
//! 列挙する。
//!
//! 決定性を最優先する。同じ木からは常に同じ順序・同じ集合が返らなければ、
//! JSON フラグメントが実行のたびに変わって差分が読めなくなる。そのために
//! 列挙結果を必ずパス順にソートし、実行環境ごとに異なるグローバル gitignore は
//! 読まない。

use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use crate::error::SpectoruError;
use crate::ports::file_walker::{DiscoveredFile, FileWalker};

/// `.gitignore` に載っていなくても常に除外するディレクトリ名。
const EXCLUDED_DIRECTORIES: &[&str] = &["target", "node_modules"];

#[derive(Debug, Default, Clone, Copy)]
pub struct IgnoreFileWalker;

impl FileWalker for IgnoreFileWalker {
    fn walk(
        &self,
        roots: &[PathBuf],
        extensions: &[&str],
    ) -> Result<Vec<DiscoveredFile>, SpectoruError> {
        let mut found = Vec::new();

        for root in roots {
            if !root.exists() {
                return Err(SpectoruError::FileWalk {
                    root: root.clone(),
                    message: "パスが存在しない".to_string(),
                });
            }
            collect(root, extensions, &mut found)?;
        }

        // root が重なっていても（例: `["./", "src/"]`）同じファイルは 1 度だけ返す。
        found.sort_by_key(absolute_path);
        found.dedup_by_key(|file| absolute_path(file));

        Ok(found)
    }

    fn read_to_string(&self, path: &Path) -> Result<String, SpectoruError> {
        std::fs::read_to_string(path).map_err(|source| SpectoruError::Io {
            path: path.to_path_buf(),
            source,
        })
    }
}

fn collect(
    root: &Path,
    extensions: &[&str],
    out: &mut Vec<DiscoveredFile>,
) -> Result<(), SpectoruError> {
    let walker = WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .git_exclude(true)
        // 利用者のホームに置かれたグローバル gitignore は読まない。
        // 実行環境によって抽出結果が変わってしまうため。
        .git_global(false)
        // `.git` が無くても `.gitignore` を適用する。リポジトリの外に置かれた
        // ソースツリーでも挙動を揃えるため。
        .require_git(false)
        .filter_entry(|entry| entry.depth() == 0 || !is_excluded_directory(entry.path()))
        .build();

    for entry in walker {
        let entry = entry.map_err(|error| SpectoruError::FileWalk {
            root: root.to_path_buf(),
            message: error.to_string(),
        })?;

        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        if !has_extension(entry.path(), extensions) {
            continue;
        }

        out.push(DiscoveredFile {
            root: root.to_path_buf(),
            relative: relative_of(root, entry.path()),
        });
    }

    Ok(())
}

fn is_excluded_directory(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| EXCLUDED_DIRECTORIES.contains(&name))
}

fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extensions.contains(&extension))
}

fn relative_of(root: &Path, path: &Path) -> PathBuf {
    match path.strip_prefix(root) {
        // root がファイルそのものを指していた場合、相対パスは空になる。
        Ok(relative) if relative.as_os_str().is_empty() => {
            PathBuf::from(path.file_name().unwrap_or_default())
        }
        Ok(relative) => relative.to_path_buf(),
        Err(_) => path.to_path_buf(),
    }
}

fn absolute_path(file: &DiscoveredFile) -> PathBuf {
    file.root.join(&file.relative)
}
