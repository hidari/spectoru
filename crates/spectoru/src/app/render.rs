//! 中間表現から静的サイトを書き出すユースケース。

use std::path::Path;

use crate::core::ir::IntermediateRepresentation;
use crate::error::SpectoruError;
use crate::ports::file_writer::FileWriter;
use crate::ports::template_engine::TemplateEngine;

/// 出力ディレクトリ直下に置くエントリポイント。
///
/// 単一 HTML なのでファイルは 1 つだけだが、静的ホスティングがそのまま
/// 配信できるよう `--out` はディレクトリを受け取る。
const INDEX_FILE_NAME: &str = "index.html";

pub struct Renderer<'a> {
    pub template: &'a dyn TemplateEngine,
    pub writer: &'a dyn FileWriter,
}

impl Renderer<'_> {
    /// 複数プロジェクトを 1 つのサイトにまとめて書き出す。
    ///
    /// 単一リポジトリの場合も要素 1 つのスライスとして扱い、集約と経路を分けない。
    pub fn render(
        &self,
        projects: &[IntermediateRepresentation],
        out_dir: &Path,
    ) -> Result<(), SpectoruError> {
        let html = self.template.render_site(projects)?;
        self.writer.write(&out_dir.join(INDEX_FILE_NAME), &html)
    }
}
