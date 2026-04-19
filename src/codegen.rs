use swc_common::sync::Lrc;
use swc_common::{SourceMap, FileName, source_map::SourceMapGenConfig};
use swc_ecma_ast::Module;
use swc_ecma_codegen::{text_writer::JsWriter, Config, Emitter};

struct MyConfig;
impl SourceMapGenConfig for MyConfig {
    fn file_name_to_source(&self, f: &FileName) -> String {
        f.to_string()
    }

    fn inline_sources_content(&self, _: &FileName) -> bool {
        true
    }
}

pub fn generate_js(module: &Module, cm: Lrc<SourceMap>) -> anyhow::Result<(String, String)> {
    let mut buf = Vec::new();
    let mut src_map_buf = Vec::new();

    {
        let mut emitter = Emitter {
            cfg: Config::default().with_minify(false),
            cm: cm.clone(),
            comments: None,
            wr: Box::new(JsWriter::new(cm.clone(), "\n", &mut buf, Some(&mut src_map_buf))),
        };

        emitter.emit_module(module)?;
    }

    let code = String::from_utf8(buf)?;
    let mut map_buf = Vec::new();
    cm.build_source_map(&src_map_buf, None, &MyConfig).to_writer(&mut map_buf)?;
    let map = String::from_utf8(map_buf)?;

    Ok((code, map))
}
