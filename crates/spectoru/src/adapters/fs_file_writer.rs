//! 標準ライブラリによる [`FileWriter`](crate::ports::file_writer::FileWriter) 実装。
//!
//! 外部 crate を使わないが、テスト時にメモリ上の Fake へ差し替えられるよう
//! ポート越しに使う点は他の adapter と同じ。

use std::fs;
use std::path::Path;

use crate::error::SpectoruError;
use crate::ports::file_writer::FileWriter;

#[derive(Debug, Default, Clone, Copy)]
pub struct FsFileWriter;

impl FileWriter for FsFileWriter {
    fn write(&self, path: &Path, contents: &str) -> Result<(), SpectoruError> {
        // `--out dist/site.html` のように未作成のディレクトリを指定できる必要がある。
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent).map_err(|source| SpectoruError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        fs::write(path, contents).map_err(|source| SpectoruError::Io {
            path: path.to_path_buf(),
            source,
        })
    }
}
