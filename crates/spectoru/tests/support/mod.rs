//! application 層のテスト用 Fake 群。
//!
//! Fake を用意するのは I/O に触れるポートだけに絞っている。TOML / JSON codec と
//! 両パーサは入力から出力を決める純粋な変換であり、本物を使った方が
//! テストの意味が強くなる（Fake を挟むと「実際にパースできるのか」が
//! 検証されなくなる）。
//!
//! いずれも「テスト内で内容を宣言する」だけの単純な構造体に留め、
//! 呼び出し回数の検証のようなモック的な使い方はしない。

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use spectoru::core::ir::IntermediateRepresentation;
use spectoru::error::SpectoruError;
use spectoru::ports::clock::Clock;
use spectoru::ports::file_walker::{DiscoveredFile, FileWalker};
use spectoru::ports::file_writer::FileWriter;
use spectoru::ports::git_provider::GitProvider;
use spectoru::ports::template_engine::TemplateEngine;

/// メモリ上の仮想ファイルツリー。
///
/// 実物の `IgnoreFileWalker` と同じ契約（パス順で決定的、root からの相対パスを
/// 返す、存在しない root はエラー）を満たすように作る。
#[derive(Debug, Default)]
pub struct FakeFileWalker {
    files: BTreeMap<PathBuf, String>,
}

impl FakeFileWalker {
    pub fn new(files: &[(&str, &str)]) -> Self {
        Self {
            files: files
                .iter()
                .map(|(path, contents)| (PathBuf::from(path), (*contents).to_string()))
                .collect(),
        }
    }

    fn exists_under(&self, root: &Path) -> bool {
        self.files
            .keys()
            .any(|path| path == root || path.starts_with(root))
    }
}

impl FileWalker for FakeFileWalker {
    fn walk(
        &self,
        roots: &[PathBuf],
        extensions: &[&str],
    ) -> Result<Vec<DiscoveredFile>, SpectoruError> {
        let mut found = Vec::new();

        for root in roots {
            if !self.exists_under(root) {
                return Err(SpectoruError::FileWalk {
                    root: root.clone(),
                    message: "パスが存在しない".to_string(),
                });
            }

            for path in self.files.keys() {
                if !path.starts_with(root) {
                    continue;
                }
                let matches_extension = path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extensions.contains(&extension));
                if !matches_extension {
                    continue;
                }
                found.push(DiscoveredFile {
                    root: root.clone(),
                    relative: path.strip_prefix(root).unwrap_or(path).to_path_buf(),
                });
            }
        }

        found.sort_by_key(|file| file.root.join(&file.relative));
        found.dedup_by_key(|file| file.root.join(&file.relative));
        Ok(found)
    }

    fn read_to_string(&self, path: &Path) -> Result<String, SpectoruError> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| SpectoruError::Io {
                path: path.to_path_buf(),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
            })
    }
}

/// 特定のファイルだけ読み込みに失敗するウォーカー。
///
/// 探索では見つかるのに読めない状況（権限・競合）を再現する。
#[derive(Debug)]
pub struct UnreadableFileWalker {
    inner: FakeFileWalker,
    unreadable: PathBuf,
}

impl UnreadableFileWalker {
    pub fn new(files: &[(&str, &str)], unreadable: &str) -> Self {
        Self {
            inner: FakeFileWalker::new(files),
            unreadable: PathBuf::from(unreadable),
        }
    }
}

impl FileWalker for UnreadableFileWalker {
    fn walk(
        &self,
        roots: &[PathBuf],
        extensions: &[&str],
    ) -> Result<Vec<DiscoveredFile>, SpectoruError> {
        self.inner.walk(roots, extensions)
    }

    fn read_to_string(&self, path: &Path) -> Result<String, SpectoruError> {
        if path == self.unreadable {
            return Err(SpectoruError::Io {
                path: path.to_path_buf(),
                source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
            });
        }
        self.inner.read_to_string(path)
    }
}

/// 書き込み内容をメモリに溜めるライター。
#[derive(Debug, Default)]
pub struct FakeFileWriter {
    written: Mutex<Vec<(PathBuf, String)>>,
}

impl FakeFileWriter {
    pub fn written(&self) -> Vec<(PathBuf, String)> {
        self.written.lock().expect("lock").clone()
    }

    pub fn contents_of(&self, path: &str) -> Option<String> {
        self.written()
            .into_iter()
            .find(|(written, _)| written == Path::new(path))
            .map(|(_, contents)| contents)
    }
}

impl FileWriter for FakeFileWriter {
    fn write(&self, path: &Path, contents: &str) -> Result<(), SpectoruError> {
        self.written
            .lock()
            .expect("lock")
            .push((path.to_path_buf(), contents.to_string()));
        Ok(())
    }
}

/// 固定の時刻を返す時計。これがないと extract の出力が実行時刻に依存する。
#[derive(Debug)]
pub struct FixedClock(pub &'static str);

impl Default for FixedClock {
    fn default() -> Self {
        Self("2026-04-13T12:00:00Z")
    }
}

impl Clock for FixedClock {
    fn now_iso8601(&self) -> String {
        self.0.to_string()
    }
}

/// 固定の revision を返す（`None` で「取得できない環境」を表す）。
#[derive(Debug, Default)]
pub struct FakeGitProvider(pub Option<&'static str>);

impl GitProvider for FakeGitProvider {
    fn current_revision(&self, _repo_root: &Path) -> Option<String> {
        self.0.map(str::to_string)
    }
}

/// 受け取った IR を検証できる形の文字列に落とすだけのテンプレートエンジン。
///
/// 実装は Phase 7 で入る。ここで確かめたいのは「レンダラが何を、いくつ、
/// どの順で渡すか」であって HTML の中身ではない。
#[derive(Debug, Default)]
pub struct FakeTemplateEngine;

impl TemplateEngine for FakeTemplateEngine {
    fn render_site(
        &self,
        projects: &[IntermediateRepresentation],
    ) -> Result<String, SpectoruError> {
        let names: Vec<&str> = projects
            .iter()
            .map(|project| project.project.name.as_str())
            .collect();
        Ok(format!("projects={}", names.join(",")))
    }
}

/// 常に失敗するテンプレートエンジン。
#[derive(Debug, Default)]
pub struct FailingTemplateEngine;

impl TemplateEngine for FailingTemplateEngine {
    fn render_site(
        &self,
        _projects: &[IntermediateRepresentation],
    ) -> Result<String, SpectoruError> {
        Err(SpectoruError::TemplateRender {
            message: "描画に失敗".to_string(),
        })
    }
}
