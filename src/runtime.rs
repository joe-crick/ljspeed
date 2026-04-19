use swc_ecma_ast::*;
use swc_ecma_visit::{Visit, VisitWith};
use std::collections::HashSet;
use swc_common::DUMMY_SP;

pub struct FreeIdFinder {
    pub free_ids: HashSet<String>,
    pub declared_ids: HashSet<String>,
}

impl Visit for FreeIdFinder {
    fn visit_ident(&mut self, n: &Ident) {
        let name = n.sym.to_string();
        if !self.declared_ids.contains(&name) {
            self.free_ids.insert(name);
        }
    }

    fn visit_fn_decl(&mut self, n: &FnDecl) {
        self.declared_ids.insert(n.ident.sym.to_string());
        n.visit_children_with(self);
    }

    fn visit_var_declarator(&mut self, n: &VarDeclarator) {
        if let Pat::Ident(ref id) = n.name {
            self.declared_ids.insert(id.id.sym.to_string());
        }
        n.visit_children_with(self);
    }

    fn visit_import_specifier(&mut self, n: &ImportSpecifier) {
        match n {
            ImportSpecifier::Named(s) => { self.declared_ids.insert(s.local.sym.to_string()); }
            ImportSpecifier::Default(s) => { self.declared_ids.insert(s.local.sym.to_string()); }
            ImportSpecifier::Namespace(s) => { self.declared_ids.insert(s.local.sym.to_string()); }
        }
    }
}

pub fn inject_runtime_imports(mut module: Module) -> Module {
    let mut finder = FreeIdFinder {
        free_ids: HashSet::new(),
        declared_ids: HashSet::new(),
    };
    
    let globals = ["console", "Math", "JSON", "String", "Array", "Object", "Set", "Map", "setTimeout", "clearTimeout"];
    for g in globals {
        finder.declared_ids.insert(g.to_string());
    }

    module.visit_with(&mut finder);

    let ljsp_runtime = ["map", "filter", "reduce", "first", "rest", "cons", "list"];
    let mut to_import = Vec::new();
    for id in finder.free_ids {
        if ljsp_runtime.contains(&id.as_str()) {
            to_import.push(id);
        }
    }

    if !to_import.is_empty() {
        to_import.sort();
        let specifiers = to_import.into_iter().map(|name| {
            ImportSpecifier::Named(ImportNamedSpecifier {
                span: DUMMY_SP,
                local: Ident::new(name.clone().into(), DUMMY_SP, Default::default()),
                imported: None,
                is_type_only: false,
            })
        }).collect();

        let import_decl = ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
            span: DUMMY_SP,
            specifiers,
            src: Box::new(Str {
                span: DUMMY_SP,
                value: "ljsp".into(),
                raw: Some("'ljsp'".into()),
            }),
            type_only: false,
            with: None,
            phase: Default::default(),
        }));

        module.body.insert(0, import_decl);
    }

    module
}
