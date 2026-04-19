use swc_ecma_ast::*;
use swc_ecma_visit::{Fold, FoldWith, Visit, VisitWith};
use swc_common::{Span, DUMMY_SP};

pub struct RecurFinder {
    pub has_recur: bool,
    pub invalid_recur_spans: Vec<(Span, String)>,
    pub in_function: bool,
    pub function_params_count: usize,
    in_try_with_finally: bool,
    in_finally: bool,
    is_tail: bool,
}

impl RecurFinder {
    fn check_stmt(&mut self, stmt: &Stmt, is_last: bool) {
        let old_tail = self.is_tail;
        self.is_tail = is_last;
        stmt.visit_with(self);
        self.is_tail = old_tail;
    }

    fn check_stmts(&mut self, stmts: &[Stmt]) {
        for (i, stmt) in stmts.iter().enumerate() {
            self.check_stmt(stmt, i == stmts.len() - 1);
        }
    }
}

impl Visit for RecurFinder {
    fn visit_call_expr(&mut self, n: &CallExpr) {
        if let Callee::Expr(ref expr) = n.callee {
            if let Expr::Ident(ref ident) = **expr {
                if ident.sym == "recur" {
                    self.has_recur = true;
                    if !self.in_function {
                        self.invalid_recur_spans.push((ident.span, "recur is only allowed inside a function".to_string()));
                    } else {
                        if !self.is_tail {
                             self.invalid_recur_spans.push((ident.span, "recur is only allowed in tail position".to_string()));
                        }
                        if n.args.len() != self.function_params_count {
                            self.invalid_recur_spans.push((ident.span, format!("recur arity mismatch: expected {} arguments, got {}", self.function_params_count, n.args.len())));
                        }
                    }
                    
                    if self.in_finally {
                        self.invalid_recur_spans.push((ident.span, "recur is invalid inside a finally block".to_string()));
                    } else if self.in_try_with_finally {
                        self.invalid_recur_spans.push((ident.span, "recur is invalid across a finally clause".to_string()));
                    }
                }
            }
        }
        // Don't mark arguments as tail position
        let old_tail = self.is_tail;
        self.is_tail = false;
        n.visit_children_with(self);
        self.is_tail = old_tail;
    }

    fn visit_block_stmt(&mut self, n: &BlockStmt) {
        self.check_stmts(&n.stmts);
    }

    fn visit_if_stmt(&mut self, n: &IfStmt) {
        let old_tail = self.is_tail;
        // Test is never tail position
        self.is_tail = false;
        n.test.visit_with(self);
        
        // Branches inherit tail position if the IfStmt itself is in tail position
        self.is_tail = old_tail;
        n.cons.visit_with(self);
        if let Some(ref alt) = n.alt {
            alt.visit_with(self);
        }
    }

    fn visit_return_stmt(&mut self, n: &ReturnStmt) {
        let old_tail = self.is_tail;
        // Expression in return is tail position
        self.is_tail = true; 
        n.visit_children_with(self);
        self.is_tail = old_tail;
    }

    fn visit_try_stmt(&mut self, n: &TryStmt) {
        let old_in_try = self.in_try_with_finally;
        if n.finalizer.is_some() {
            self.in_try_with_finally = true;
        }
        
        // try/catch blocks inherit tail position? 
        // Spec: "tail position inside try when there is no finally"
        n.block.visit_with(self);
        if let Some(handler) = &n.handler {
            handler.visit_with(self);
        }
        self.in_try_with_finally = old_in_try;

        if let Some(finally) = &n.finalizer {
            let old_in_finally = self.in_finally;
            let old_tail = self.is_tail;
            self.in_finally = true;
            self.is_tail = false; // finally is never tail position for outer function
            finally.visit_with(self);
            self.in_finally = old_in_finally;
            self.is_tail = old_tail;
        }
    }

    fn visit_fn_decl(&mut self, n: &FnDecl) {
        let old_in_function = self.in_function;
        let old_params_count = self.function_params_count;
        let old_tail = self.is_tail;
        self.in_function = true;
        self.function_params_count = n.function.params.len();
        self.is_tail = false; // Top of function body is tail position for its stmts
        
        if let Some(ref body) = n.function.body {
            self.check_stmts(&body.stmts);
        }
        
        self.in_function = old_in_function;
        self.function_params_count = old_params_count;
        self.is_tail = old_tail;
    }

    fn visit_fn_expr(&mut self, n: &FnExpr) {
        let old_in_function = self.in_function;
        let old_params_count = self.function_params_count;
        let old_tail = self.is_tail;
        self.in_function = true;
        self.function_params_count = n.function.params.len();
        self.is_tail = false;
        
        if let Some(ref body) = n.function.body {
            self.check_stmts(&body.stmts);
        }

        self.in_function = old_in_function;
        self.function_params_count = old_params_count;
        self.is_tail = old_tail;
    }

    fn visit_arrow_expr(&mut self, n: &ArrowExpr) {
        let old_in_function = self.in_function;
        let old_params_count = self.function_params_count;
        let old_tail = self.is_tail;
        self.in_function = true;
        self.function_params_count = n.params.len();
        self.is_tail = false;

        match &*n.body {
            BlockStmtOrExpr::BlockStmt(body) => {
                self.check_stmts(&body.stmts);
            }
            BlockStmtOrExpr::Expr(expr) => {
                // Expression body: is it tail position? 
                // v11.1: "recur is NOT allowed in arrow functions with expression bodies"
                let mut sub_finder = RecurFinder {
                    has_recur: false,
                    invalid_recur_spans: vec![],
                    in_function: true,
                    function_params_count: n.params.len(),
                    in_try_with_finally: self.in_try_with_finally,
                    in_finally: self.in_finally,
                    is_tail: true, // It is technically tail, but forbidden
                };
                expr.visit_with(&mut sub_finder);
                if sub_finder.has_recur {
                    self.invalid_recur_spans.push((n.span, "recur is NOT allowed in arrow functions with expression bodies".to_string()));
                }
                self.invalid_recur_spans.extend(sub_finder.invalid_recur_spans);
            }
        }
        
        self.in_function = old_in_function;
        self.function_params_count = old_params_count;
        self.is_tail = old_tail;
    }
}

pub struct FunctionRewriter {
    params: Vec<String>,
    pub has_recur: bool,
}

impl FunctionRewriter {
    pub fn new(params: Vec<String>) -> Self {
        Self { params, has_recur: false }
    }
}

impl Fold for FunctionRewriter {
    fn fold_stmt(&mut self, n: Stmt) -> Stmt {
        match n {
            Stmt::Return(ReturnStmt { arg: Some(expr), span, .. }) => {
                if let Expr::Call(call) = *expr {
                    if let Callee::Expr(ref callee_expr) = call.callee {
                        if let Expr::Ident(ref ident) = **callee_expr {
                            if ident.sym == "recur" {
                                self.has_recur = true;
                                return self.lower_recur_call(call, span);
                            }
                        }
                    }
                    Stmt::Return(ReturnStmt { arg: Some(Box::new(Expr::Call(call))), span })
                } else {
                    Stmt::Return(ReturnStmt { arg: Some(expr), span })
                }
            }
            Stmt::Expr(ExprStmt { expr, span, .. }) => {
                if let Expr::Call(call) = *expr {
                    if let Callee::Expr(ref callee_expr) = call.callee {
                        if let Expr::Ident(ref ident) = **callee_expr {
                            if ident.sym == "recur" {
                                self.has_recur = true;
                                return self.lower_recur_call(call, span);
                            }
                        }
                    }
                    Stmt::Expr(ExprStmt { expr: Box::new(Expr::Call(call)), span })
                } else {
                    Stmt::Expr(ExprStmt { expr, span })
                }
            }
            _ => n.fold_children_with(self),
        }
    }

    fn fold_fn_decl(&mut self, n: FnDecl) -> FnDecl { n }
    fn fold_fn_expr(&mut self, n: FnExpr) -> FnExpr { n }
    fn fold_arrow_expr(&mut self, n: ArrowExpr) -> ArrowExpr { n }
}

impl FunctionRewriter {
    fn lower_recur_call(&self, call: CallExpr, _span: Span) -> Stmt {
        let mut stmts = Vec::new();
        
        let mut next_vars = Vec::new();
        for (i, arg) in call.args.into_iter().enumerate() {
            let var_name = format!("__next{}", i);
            next_vars.push(var_name.clone());
            stmts.push(Stmt::Decl(Decl::Var(Box::new(VarDecl {
                span: DUMMY_SP,
                kind: VarDeclKind::Const,
                declare: false,
                decls: vec![VarDeclarator {
                    span: DUMMY_SP,
                    name: Pat::Ident(BindingIdent {
                        id: Ident::new(var_name.into(), DUMMY_SP, Default::default()),
                        type_ann: None,
                    }),
                    init: Some(arg.expr),
                    definite: false,
                }],
                ctxt: Default::default(),
            }))));
        }

        for (i, var_name) in next_vars.into_iter().enumerate() {
            if let Some(param_name) = self.params.get(i) {
                stmts.push(Stmt::Expr(ExprStmt {
                    span: DUMMY_SP,
                    expr: Box::new(Expr::Assign(AssignExpr {
                        span: DUMMY_SP,
                        op: AssignOp::Assign,
                        left: AssignTarget::Simple(SimpleAssignTarget::Ident(BindingIdent {
                            id: Ident::new(param_name.clone().into(), DUMMY_SP, Default::default()),
                            type_ann: None,
                        })),
                        right: Box::new(Expr::Ident(Ident::new(var_name.into(), DUMMY_SP, Default::default()))),
                    })),
                }));
            }
        }

        stmts.push(Stmt::Continue(ContinueStmt {
            span: DUMMY_SP,
            label: None,
        }));

        Stmt::Block(BlockStmt {
            span: DUMMY_SP,
            stmts,
            ctxt: Default::default(),
        })
    }
}

pub struct RecurLowerer;

impl Fold for RecurLowerer {
    fn fold_fn_decl(&mut self, mut n: FnDecl) -> FnDecl {
        let params = n.function.params.iter().filter_map(|p| {
            if let Pat::Ident(ref id) = p.pat {
                Some(id.id.sym.to_string())
            } else {
                None
            }
        }).collect();

        let mut rewriter = FunctionRewriter::new(params);
        n.function = Box::new(*n.function.fold_with(&mut rewriter));

        if rewriter.has_recur {
            if let Some(ref mut body) = n.function.body {
                let old_stmts = std::mem::take(&mut body.stmts);
                body.stmts = vec![Stmt::While(WhileStmt {
                    span: DUMMY_SP,
                    test: Box::new(Expr::Lit(Lit::Bool(Bool { span: DUMMY_SP, value: true }))),
                    body: Box::new(Stmt::Block(BlockStmt {
                        span: DUMMY_SP,
                        stmts: old_stmts,
                        ctxt: Default::default(),
                    })),
                })];
            }
        }

        n.fold_children_with(self)
    }

    fn fold_fn_expr(&mut self, mut n: FnExpr) -> FnExpr {
        let params = n.function.params.iter().filter_map(|p| {
            if let Pat::Ident(ref id) = p.pat {
                Some(id.id.sym.to_string())
            } else {
                None
            }
        }).collect();

        let mut rewriter = FunctionRewriter::new(params);
        n.function = Box::new(*n.function.fold_with(&mut rewriter));

        if rewriter.has_recur {
            if let Some(ref mut body) = n.function.body {
                let old_stmts = std::mem::take(&mut body.stmts);
                body.stmts = vec![Stmt::While(WhileStmt {
                    span: DUMMY_SP,
                    test: Box::new(Expr::Lit(Lit::Bool(Bool { span: DUMMY_SP, value: true }))),
                    body: Box::new(Stmt::Block(BlockStmt {
                        span: DUMMY_SP,
                        stmts: old_stmts,
                        ctxt: Default::default(),
                    })),
                })];
            }
        }

        n.fold_children_with(self)
    }

    fn fold_arrow_expr(&mut self, mut n: ArrowExpr) -> ArrowExpr {
        let params = n.params.iter().filter_map(|p| {
            if let Pat::Ident(id) = p {
                Some(id.id.sym.to_string())
            } else {
                None
            }
        }).collect();

        let mut rewriter = FunctionRewriter::new(params);
        n.body = Box::new(*n.body.fold_with(&mut rewriter));

        if rewriter.has_recur {
            if let BlockStmtOrExpr::BlockStmt(ref mut body) = *n.body {
                let old_stmts = std::mem::take(&mut body.stmts);
                body.stmts = vec![Stmt::While(WhileStmt {
                    span: DUMMY_SP,
                    test: Box::new(Expr::Lit(Lit::Bool(Bool { span: DUMMY_SP, value: true }))),
                    body: Box::new(Stmt::Block(BlockStmt {
                        span: DUMMY_SP,
                        stmts: old_stmts,
                        ctxt: Default::default(),
                    })),
                })];
            }
        }

        n.fold_children_with(self)
    }
}

pub fn lower_recur(module: Module) -> anyhow::Result<Module> {
    let mut finder = RecurFinder { 
        has_recur: false, 
        invalid_recur_spans: vec![],
        in_function: false,
        function_params_count: 0,
        in_try_with_finally: false,
        in_finally: false,
        is_tail: false,
    };
    module.visit_with(&mut finder);

    if !finder.invalid_recur_spans.is_empty() {
        let mut err_msg = String::new();
        for (span, msg) in finder.invalid_recur_spans {
            err_msg.push_str(&format!("[AnalysisError] {}: {:?}\n", msg, span));
        }
        return Err(anyhow::anyhow!(err_msg));
    }

    let mut lowerer = RecurLowerer;
    Ok(module.fold_with(&mut lowerer))
}
