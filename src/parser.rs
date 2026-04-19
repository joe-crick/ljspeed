use swc_common::{
    errors::Handler,
    sync::Lrc,
    FileName, SourceMap,
};
use swc_ecma_ast::Module;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};

pub fn parse_js(source_code: &str, filename: &str) -> anyhow::Result<(Module, Lrc<SourceMap>)> {
    let cm = Lrc::new(SourceMap::default());
    let handler = Handler::with_emitter_writer(Box::new(std::io::stderr()), Some(cm.clone()));

    let fm = cm.new_source_file(FileName::Custom(filename.to_string()).into(), source_code.to_string());

    let lexer = Lexer::new(
        Syntax::Es(Default::default()),
        Default::default(),
        StringInput::from(&*fm),
        None,
    );

    let mut parser = Parser::new_from(lexer);

    for e in parser.take_errors() {
        e.into_diagnostic(&handler).emit();
    }

    let module = parser
        .parse_module()
        .map_err(|e| anyhow::anyhow!("Parse error: {:?}", e))?;

    Ok((module, cm))
}
