use crate::sandbox::MacroSandbox;
use crate::marshalling::{expr_to_json, json_to_expr, json_to_stmt};
use crate::template::resolve_template;
use swc_ecma_ast::*;
use swc_ecma_visit::{Fold, FoldWith, Visit, VisitWith};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use serde_json::Value;

pub struct MacroImportFinder {
    pub macro_imports: HashMap<String, (String, String)>, // local_name -> (module_path, exported_name)
    pub namespace_imports: HashMap<String, String>, // local_namespace -> module_path
}

impl Visit for MacroImportFinder {
    fn visit_import_decl(&mut self, n: &ImportDecl) {
        let src_opt = n.src.value.as_str();
        if let Some(src) = src_opt {
            if src.ends_with(".macro.js") {
                for spec in &n.specifiers {
                    match spec {
                        ImportSpecifier::Named(named) => {
                            let local = named.local.sym.to_string();
                            let imported = named.imported.as_ref().map(|i| {
                                match i {
                                    ModuleExportName::Ident(id) => id.sym.to_string(),
                                    ModuleExportName::Str(s) => s.value.as_str().unwrap_or_default().to_string(),
                                }
                            }).unwrap_or_else(|| local.clone());
                            self.macro_imports.insert(local, (src.to_string(), imported));
                        }
                        ImportSpecifier::Namespace(ns) => {
                            self.namespace_imports.insert(ns.local.sym.to_string(), src.to_string());
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

pub struct Scope {
    bindings: HashSet<String>,
    parent: Option<Box<Scope>>,
}

impl Scope {
    fn new(parent: Option<Box<Scope>>) -> Self {
        Self { bindings: HashSet::new(), parent }
    }
    fn add(&mut self, name: String) { self.bindings.insert(name); }
    fn has(&self, name: &str) -> bool {
        if self.bindings.contains(name) { return true; }
        self.parent.as_ref().map_or(false, |p| p.has(name))
    }
}

pub struct MacroExpander {
    pub sandbox: MacroSandbox,
    pub macro_imports: HashMap<String, (String, String)>,
    pub namespace_imports: HashMap<String, String>,
    pub expanded_this_pass: bool,
    pub depth: usize,
    pub max_depth: usize,
    pub scope: Option<Box<Scope>>,
}

impl MacroExpander {
    fn handle_macro_result(&mut self, res: Value) -> Value {
        if let Some(obj) = res.as_object() {
            if let Some(type_val) = obj.get("type") {
                if type_val == "TemplateResult" {
                    let kind = obj.get("kind").and_then(|v| v.as_str()).unwrap_or("");
                    let code = obj.get("code").and_then(|v| v.as_str()).unwrap_or("");
                    let values = obj.get("values").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                    if let Some(resolved) = resolve_template(kind, code, values) {
                        return resolved;
                    }
                }
            }
        }
        res
    }

    fn enter_scope(&mut self) {
        let old_scope = self.scope.take();
        self.scope = Some(Box::new(Scope::new(old_scope)));
    }

    fn leave_scope(&mut self) {
        if let Some(s) = self.scope.take() {
            self.scope = s.parent;
        }
    }

    fn add_binding(&mut self, name: String) {
        if let Some(ref mut s) = self.scope {
            s.add(name);
        }
    }

    fn is_macro(&self, name: &str) -> bool {
        if let Some(ref s) = self.scope {
            if s.has(name) { return false; }
        }
        self.macro_imports.contains_key(name)
    }
}

impl Fold for MacroExpander {
    fn fold_module_items(&mut self, items: Vec<ModuleItem>) -> Vec<ModuleItem> {
        let mut new_items = Vec::new();
        for item in items {
            match item {
                ModuleItem::ModuleDecl(ModuleDecl::Import(ref import)) if import.src.value.as_str().unwrap_or_default().ends_with(".macro.js") => {
                    continue;
                }
                ModuleItem::Stmt(ref stmt) => {
                    let mut expanded_stmt = None;
                    if let Stmt::Expr(ExprStmt { expr, span, .. }) = stmt {
                        if let Expr::Call(call) = &**expr {
                            let mut resolved = None;
                            if let Callee::Expr(callee_expr) = &call.callee {
                                match &**callee_expr {
                                    Expr::Ident(ident) => {
                                        if self.is_macro(ident.sym.as_str()) {
                                            resolved = self.macro_imports.get(ident.sym.as_str()).cloned();
                                        }
                                    }
                                    Expr::Member(MemberExpr { obj, prop, .. }) => {
                                        if let Expr::Ident(obj_id) = &**obj {
                                            if let MemberProp::Ident(prop_id) = prop {
                                                if let Some(module_path) = self.namespace_imports.get(obj_id.sym.as_str()) {
                                                    resolved = Some((module_path.clone(), prop_id.sym.to_string()));
                                                }
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }

                            if let Some((module_path, exported_name)) = resolved {
                                let args_json = call.args.iter().map(|a| expr_to_json(&a.expr)).collect::<Vec<_>>();
                                if let Ok(res_json) = self.sandbox.call_macro(&module_path, &exported_name, args_json) {
                                    let final_res = self.handle_macro_result(res_json);
                                    if let Some(new_stmt) = json_to_stmt(final_res, *span) {
                                        expanded_stmt = Some(new_stmt);
                                    }
                                }
                            }
                        }
                    }

                    if let Some(mut new_s) = expanded_stmt {
                        self.expanded_this_pass = true;
                        new_s = new_s.fold_with(self);
                        new_items.push(ModuleItem::Stmt(new_s));
                    } else {
                        new_items.push(item.clone().fold_with(self));
                    }
                }
                _ => {
                    new_items.push(item.clone().fold_with(self));
                }
            }
        }
        new_items
    }

    fn fold_expr(&mut self, n: Expr) -> Expr {
        if let Expr::Call(ref call) = n {
            let span = call.span;
            let mut resolved = None;
            if let Callee::Expr(callee_expr) = &call.callee {
                match &**callee_expr {
                    Expr::Ident(ident) => {
                        if self.is_macro(ident.sym.as_str()) {
                            resolved = self.macro_imports.get(ident.sym.as_str()).cloned();
                        }
                    }
                    Expr::Member(MemberExpr { obj, prop, .. }) => {
                        if let Expr::Ident(obj_id) = &**obj {
                            if let MemberProp::Ident(prop_id) = prop {
                                if let Some(module_path) = self.namespace_imports.get(obj_id.sym.as_str()) {
                                    resolved = Some((module_path.clone(), prop_id.sym.to_string()));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            if let Some((module_path, exported_name)) = resolved {
                let args_json = call.args.iter().map(|a| expr_to_json(&a.expr)).collect::<Vec<_>>();
                if let Ok(res_json) = self.sandbox.call_macro(&module_path, &exported_name, args_json) {
                    let final_res = self.handle_macro_result(res_json);
                    if let Some(new_expr) = json_to_expr(final_res, span) {
                        self.expanded_this_pass = true;
                        return (*new_expr).fold_with(self);
                    }
                }
            }
        }
        n.fold_children_with(self)
    }

    fn fold_fn_decl(&mut self, mut n: FnDecl) -> FnDecl {
        self.add_binding(n.ident.sym.to_string());
        self.enter_scope();
        for p in &n.function.params {
            if let Pat::Ident(ref id) = p.pat {
                self.add_binding(id.id.sym.to_string());
            }
        }
        n.function = n.function.fold_with(self);
        self.leave_scope();
        n
    }

    fn fold_fn_expr(&mut self, mut n: FnExpr) -> FnExpr {
        self.enter_scope();
        if let Some(ref id) = n.ident {
            self.add_binding(id.sym.to_string());
        }
        for p in &n.function.params {
            if let Pat::Ident(ref id) = p.pat {
                self.add_binding(id.id.sym.to_string());
            }
        }
        n.function = n.function.fold_with(self);
        self.leave_scope();
        n
    }

    fn fold_arrow_expr(&mut self, mut n: ArrowExpr) -> ArrowExpr {
        self.enter_scope();
        for p in &n.params {
            if let Pat::Ident(id) = p {
                self.add_binding(id.id.sym.to_string());
            }
        }
        n.body = n.body.fold_with(self);
        self.leave_scope();
        n
    }
}

pub fn expand_macros(mut module: Module, mut sandbox: MacroSandbox) -> anyhow::Result<Module> {
    let mut finder = MacroImportFinder { 
        macro_imports: HashMap::new(),
        namespace_imports: HashMap::new(),
    };
    module.visit_with(&mut finder);

    // Load macro modules
    let mut loaded_modules = std::collections::HashSet::new();
    let all_module_paths: HashSet<_> = finder.macro_imports.values().map(|(p, _)| p.clone())
        .chain(finder.namespace_imports.values().cloned()).collect();
        
    for path_str in all_module_paths {
        if !loaded_modules.contains(&path_str) {
            let path = Path::new(&path_str);
            if let Ok(code) = std::fs::read_to_string(path) {
                sandbox.load_macro_module(&path_str, &code)?;
                loaded_modules.insert(path_str.clone());
            }
        }
    }

    let mut expander = MacroExpander {
        sandbox,
        macro_imports: finder.macro_imports,
        namespace_imports: finder.namespace_imports,
        expanded_this_pass: false,
        depth: 0,
        max_depth: 256,
        scope: None,
    };
    
    loop {
        expander.expanded_this_pass = false;
        expander.scope = None; // Reset scope for each pass
        module = module.fold_with(&mut expander);
        expander.depth += 1;
        
        if !expander.expanded_this_pass || expander.depth >= expander.max_depth {
            break;
        }
    }
    
    Ok(module)
}
