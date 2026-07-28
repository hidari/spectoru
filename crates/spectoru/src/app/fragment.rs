//! 中間表現を JSON フラグメントとして読み書きするユースケース。
//!
//! 複数リポジトリ集約パターンでは、各リポジトリが `extract` でフラグメントを
//! 書き出し、spec-site リポジトリがそれらを読み込んで `render` する。
//! その境界がこのモジュールになる。

use std::path::Path;

use crate::core::ir::IntermediateRepresentation;
use crate::error::SpectoruError;
use crate::ports::file_walker::FileWalker;
use crate::ports::file_writer::FileWriter;
use crate::ports::json_codec::JsonCodec;

pub struct FragmentStore<'a> {
    pub json_codec: &'a dyn JsonCodec,
    pub walker: &'a dyn FileWalker,
    pub writer: &'a dyn FileWriter,
}

impl FragmentStore<'_> {
    pub fn save(&self, ir: &IntermediateRepresentation, path: &Path) -> Result<(), SpectoruError> {
        let json = self.json_codec.encode(ir)?;
        self.writer.write(path, &json)
    }

    /// 複数のフラグメントを渡された順に読み込む。
    ///
    /// 1 つでも壊れていれば全体を失敗させる。欠けたまま生成すると、
    /// 「あるはずの仕様がサイトに無い」ことに気づけないため。
    pub fn load_all(
        &self,
        paths: &[impl AsRef<Path>],
    ) -> Result<Vec<IntermediateRepresentation>, SpectoruError> {
        paths.iter().map(|path| self.load(path.as_ref())).collect()
    }

    fn load(&self, path: &Path) -> Result<IntermediateRepresentation, SpectoruError> {
        let json = self.walker.read_to_string(path)?;

        // JSON codec はファイル専用ではない汎用の codec なので、
        // 「どのフラグメントが壊れているか」はここで与える。
        self.json_codec
            .decode(&json)
            .map_err(|error| SpectoruError::Fragment {
                path: path.to_path_buf(),
                message: error.to_string(),
            })
    }
}
