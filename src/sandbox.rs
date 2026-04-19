use deno_core::anyhow;
use deno_core::JsRuntime;
use deno_core::RuntimeOptions;
use deno_core::FastString;
use deno_core::v8;
use deno_core::scope;
use crate::template::resolve_template;

pub struct MacroSandbox {
    runtime: JsRuntime,
}

impl MacroSandbox {
    pub fn new() -> anyhow::Result<Self> {
        let runtime = JsRuntime::new(RuntimeOptions {
            ..Default::default()
        });

        let mut sandbox = Self { runtime };
        sandbox.bootstrap()?;
        Ok(sandbox)
    }

    fn bootstrap(&mut self) -> anyhow::Result<()> {
        let code = r#"
            globalThis.macros = new Map();
            globalThis.defineMacro = (fn) => ({ isMacro: true, fn });
            
            const createSyntaxParser = (kind) => {
                return (strings, ...values) => {
                    let code = strings[0];
                    for (let i = 0; i < values.length; i++) {
                        code += `__MACRO_INTERP_${i}__` + strings[i + 1];
                    }
                    return globalThis.__parseFragment(kind, code, values);
                };
            };

            globalThis.ctx = {
                gensym: (prefix = 'tmp') => ({ type: 'Identifier', name: `__m_${prefix}_${globalThis.__gensymCount++}` }),
                ident: (name) => ({ type: 'Identifier', name }),
                literal: (value) => ({ type: 'Literal', value, raw: String(value) }),
                block: (body) => ({ type: 'BlockStatement', body: Array.isArray(body) ? body : [body] }),
                call: (callee, args = []) => ({ type: 'CallExpression', callee, arguments: args }),
                member: (object, property, options = {}) => ({ 
                    type: 'MemberExpression', 
                    object, 
                    property: typeof property === 'string' ? { type: 'Identifier', name: property } : property,
                    computed: !!options.computed 
                }),
                return: (argument) => ({ type: 'ReturnStatement', argument }),
                var: (kind, id, init) => ({ 
                    type: 'VariableDeclaration', 
                    kind, 
                    declarations: [{ type: 'VariableDeclarator', id: typeof id === 'string' ? { type: 'Identifier', name: id } : id, init }] 
                }),
                function: (params, body, options = {}) => ({
                    type: options.expression ? 'ArrowFunctionExpression' : 'FunctionExpression',
                    params: params.map(p => typeof p === 'string' ? { type: 'Identifier', name: p } : p),
                    body: Array.isArray(body) ? { type: 'BlockStatement', body } : body,
                    expression: !!options.expression
                }),
                clone: (node) => JSON.parse(JSON.stringify(node)),
                error: (message) => { throw new Error(message); },
                syntax: {
                    expression: createSyntaxParser('expression'),
                    statement: createSyntaxParser('statement'),
                    program: createSyntaxParser('program')
                }
            };
            globalThis.__gensymCount = 0;
            globalThis.macroExports = {};
        "#;
        self.runtime.execute_script("<bootstrap>", FastString::from_static(code))?;

        scope!(scope, &mut self.runtime);
        let context = scope.get_current_context();
        let global = context.global(scope);

        let key = v8::String::new(scope, "__parseFragment").unwrap();
        let tmpl = v8::FunctionTemplate::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let kind = args.get(0).to_rust_string_lossy(scope);
            let code = args.get(1).to_rust_string_lossy(scope);
            let values_v8 = args.get(2);
            
            let val_v8_global = v8::Global::new(scope, values_v8);
            let values_local = v8::Local::new(scope, val_v8_global);
            
            let values: Vec<serde_json::Value> = serde_v8::from_v8(scope, values_local).unwrap_or_default();
            
            if let Some(res) = resolve_template(&kind, &code, values) {
                let res_v8 = serde_v8::to_v8(scope, res).unwrap();
                rv.set(res_v8);
            }
        });
        let func = tmpl.get_function(scope).unwrap();
        global.set(scope, key.into(), func.into());

        Ok(())
    }

    pub fn load_macro_module(&mut self, path: &str, code: &str) -> anyhow::Result<()> {
        let transpiled = code.replace("import { defineMacro } from \"@ljsp/macro-runtime\"", "")
                             .replace("import { defineMacro } from '@ljsp/macro-runtime'", "")
                             .replace("export const ", "exports.")
                             .replace("export function ", "exports.");
                               
        let wrapper = format!(
            "(function(exports) {{ 
                const {{ defineMacro }} = globalThis;
                {} 
            }})(globalThis.macroExports['{}'] = {{}});", 
            transpiled,
            path
        );
        
        self.runtime.execute_script(FastString::from(path.to_string()), FastString::from(wrapper))?;
        Ok(())
    }

    pub fn call_macro(&mut self, module_path: &str, name: &str, args: Vec<serde_json::Value>) -> anyhow::Result<serde_json::Value> {
        let js_args = serde_json::to_string(&args)?;
        let call_code = format!("globalThis.macroExports['{}']['{}'].fn(globalThis.ctx, ...{})", module_path, name, js_args);
        let global_val = self.runtime.execute_script("<call>", FastString::from(call_code))?;
        
        scope!(scope, &mut self.runtime);
        let local_val = v8::Local::new(scope, global_val);
        
        let deserialized_value = serde_v8::from_v8::<serde_json::Value>(scope, local_val)?;
        Ok(deserialized_value)
    }
}
