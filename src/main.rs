mod parser;
mod sandbox;
mod expander;
mod recur;
mod codegen;
mod marshalling;
mod template;
mod runtime;

use parser::parse_js;
use sandbox::MacroSandbox;
use expander::expand_macros;
use recur::lower_recur;
use codegen::generate_js;
use runtime::inject_runtime_imports;
use anyhow::Context;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <input.js>", args[0]);
        return Ok(());
    }

    let input_path = &args[1];
    let source_code = std::fs::read_to_string(input_path).context("Failed to read input file")?;

    // 1. Parse
    let (module, cm) = parse_js(&source_code, input_path)?;

    // 2. Setup Sandbox
    let sandbox = MacroSandbox::new()?;

    // 3. Expand Macros
    let module = expand_macros(module, sandbox)?;

    // 4. Lower Recur
    let module = lower_recur(module)?;
    
    // 5. Inject Runtime Imports
    let module = inject_runtime_imports(module);

    // 6. Generate Code
    let (output_code, source_map) = generate_js(&module, cm)?;

    let output_path = input_path.replace(".js", ".out.js");
    let map_path = format!("{}.map", output_path);
    
    std::fs::write(&output_path, &output_code).context("Failed to write output file")?;
    std::fs::write(&map_path, &source_map).context("Failed to write source map file")?;

    println!("Generated: {}", output_path);
    println!("Source Map: {}", map_path);
    
    Ok(())
}
