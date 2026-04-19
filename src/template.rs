use swc_ecma_ast::*;
use swc_ecma_visit::{Fold, FoldWith};
use crate::marshalling::{json_to_expr, json_to_stmt, expr_to_json, stmt_to_json};
use crate::parser::parse_js;
use serde_json::{Value, json};
use swc_common::DUMMY_SP;

pub struct TemplateResolver {
    pub values: Vec<Value>,
}

impl Fold for TemplateResolver {
    fn fold_module_items(&mut self, items: Vec<ModuleItem>) -> Vec<ModuleItem> {
        let mut new_items = Vec::new();
        for item in items {
            let mut expanded = None;
            if let ModuleItem::Stmt(Stmt::Expr(ExprStmt { ref expr, .. })) = item {
                if let Expr::Ident(ref ident) = **expr {
                    if ident.sym.starts_with("__MACRO_INTERP_") {
                        let idx_str = ident.sym.replace("__MACRO_INTERP_", "").replace("__", "");
                        if let Ok(idx) = idx_str.parse::<usize>() {
                            if let Some(val) = self.values.get(idx) {
                                if val.is_array() {
                                    for v in val.as_array().unwrap() {
                                        if let Some(s) = json_to_stmt(v.clone(), DUMMY_SP) {
                                            new_items.push(ModuleItem::Stmt(s));
                                        }
                                    }
                                    expanded = Some(());
                                } else if let Some(s) = json_to_stmt(val.clone(), DUMMY_SP) {
                                    new_items.push(ModuleItem::Stmt(s));
                                    expanded = Some(());
                                }
                            }
                        }
                    }
                }
            }
            
            if expanded.is_none() {
                new_items.push(item.fold_with(self));
            }
        }
        new_items
    }

    fn fold_stmts(&mut self, stmts: Vec<Stmt>) -> Vec<Stmt> {
        let mut new_stmts = Vec::new();
        for stmt in stmts {
            let mut expanded = None;
            if let Stmt::Expr(ExprStmt { ref expr, .. }) = stmt {
                if let Expr::Ident(ref ident) = **expr {
                    if ident.sym.starts_with("__MACRO_INTERP_") {
                        let idx_str = ident.sym.replace("__MACRO_INTERP_", "").replace("__", "");
                        if let Ok(idx) = idx_str.parse::<usize>() {
                            if let Some(val) = self.values.get(idx) {
                                if val.is_array() {
                                    for v in val.as_array().unwrap() {
                                        if let Some(s) = json_to_stmt(v.clone(), DUMMY_SP) {
                                            new_stmts.push(s);
                                        }
                                    }
                                    expanded = Some(());
                                } else if let Some(s) = json_to_stmt(val.clone(), DUMMY_SP) {
                                    new_stmts.push(s);
                                    expanded = Some(());
                                }
                            }
                        }
                    }
                }
            }

            if expanded.is_none() {
                new_stmts.push(stmt.fold_with(self));
            }
        }
        new_stmts
    }

    fn fold_expr_or_spreads(&mut self, args: Vec<ExprOrSpread>) -> Vec<ExprOrSpread> {
        let mut new_args = Vec::new();
        for arg in args {
            let mut expanded = None;
            if let Expr::Ident(ref ident) = *arg.expr {
                if ident.sym.starts_with("__MACRO_INTERP_") {
                    let idx_str = ident.sym.replace("__MACRO_INTERP_", "").replace("__", "");
                    if let Ok(idx) = idx_str.parse::<usize>() {
                        if let Some(val) = self.values.get(idx) {
                            if val.is_array() {
                                for v in val.as_array().unwrap() {
                                    if let Some(e) = json_to_expr(v.clone(), DUMMY_SP) {
                                        new_args.push(ExprOrSpread { spread: None, expr: e });
                                    }
                                }
                                expanded = Some(());
                            } else if let Some(e) = json_to_expr(val.clone(), DUMMY_SP) {
                                new_args.push(ExprOrSpread { spread: None, expr: e });
                                expanded = Some(());
                            }
                        }
                    }
                }
            }
            
            if expanded.is_none() {
                new_args.push(arg.fold_with(self));
            }
        }
        new_args
    }

    fn fold_expr(&mut self, n: Expr) -> Expr {
        if let Expr::Ident(ref ident) = n {
            if ident.sym.starts_with("__MACRO_INTERP_") {
                let idx_str = ident.sym.replace("__MACRO_INTERP_", "").replace("__", "");
                if let Ok(idx) = idx_str.parse::<usize>() {
                    if let Some(val) = self.values.get(idx) {
                        if let Some(expr) = json_to_expr(val.clone(), DUMMY_SP) {
                            return *expr;
                        }
                    }
                }
            }
        }
        n.fold_children_with(self)
    }
}

pub fn resolve_template(kind: &str, code: &str, values: Vec<Value>) -> Option<Value> {
    let wrap_code = if kind == "expression" { format!("({})", code) } else { code.to_string() };
    match parse_js(&wrap_code, "template.js") {
        Ok((module, _)) => {
            let mut resolver = TemplateResolver { values };
            let resolved_module = module.fold_with(&mut resolver);
            
            match kind {
                "expression" => {
                    if let Some(ModuleItem::Stmt(Stmt::Expr(ExprStmt { expr, .. }))) = resolved_module.body.get(0) {
                        Some(expr_to_json(expr))
                    } else {
                        None
                    }
                }
                "statement" => {
                    if let Some(ModuleItem::Stmt(stmt)) = resolved_module.body.get(0) {
                        Some(stmt_to_json(stmt))
                    } else {
                        None
                    }
                }
                "program" => {
                    Some(json!({
                        "type": "BlockStatement",
                        "body": resolved_module.body.iter().filter_map(|item| {
                            match item {
                                ModuleItem::Stmt(s) => Some(stmt_to_json(s)),
                                _ => None
                            }
                        }).collect::<Vec<_>>()
                    }))
                }
                _ => None
            }
        }
        Err(_) => None
    }
}
