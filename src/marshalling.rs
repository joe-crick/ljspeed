use swc_ecma_ast::*;
use serde_json::{Value, json};
use swc_common::{SyntaxContext, Span};

pub fn expr_to_json(expr: &Expr) -> Value {
    match expr {
        Expr::Ident(id) => json!({
            "type": "Identifier",
            "name": id.sym.as_str().to_string()
        }),
        Expr::Lit(Lit::Num(n)) => json!({
            "type": "Literal",
            "value": n.value,
            "raw": n.value.to_string()
        }),
        Expr::Lit(Lit::Str(s)) => json!({
            "type": "Literal",
            "value": s.value.as_str().unwrap_or_default().to_string(),
            "raw": format!("\"{}\"", s.value.as_str().unwrap_or_default())
        }),
        Expr::Array(arr) => json!({
            "type": "ArrayExpression",
            "elements": arr.elems.iter().map(|e| e.as_ref().map(|ee| expr_to_json(&ee.expr))).collect::<Vec<_>>()
        }),
        Expr::Object(obj) => json!({
            "type": "ObjectExpression",
            "properties": obj.props.iter().map(|p| match p {
                PropOrSpread::Prop(prop) => match &**prop {
                    Prop::KeyValue(KeyValueProp { key, value }) => json!({
                        "type": "Property",
                        "key": match key {
                            PropName::Ident(id) => json!({ "type": "Identifier", "name": id.sym.as_str() }),
                            _ => json!(null)
                        },
                        "value": expr_to_json(value),
                        "kind": "init"
                    }),
                    _ => json!(null)
                },
                _ => json!(null)
            }).collect::<Vec<_>>()
        }),
        Expr::Arrow(arrow) => json!({
            "type": "ArrowFunctionExpression",
            "params": arrow.params.iter().map(|p| match p {
                Pat::Ident(id) => json!({ "type": "Identifier", "name": id.id.sym.as_str() }),
                _ => json!(null)
            }).collect::<Vec<_>>(),
            "body": match &*arrow.body {
                BlockStmtOrExpr::BlockStmt(b) => stmt_to_json(&Stmt::Block(b.clone())),
                BlockStmtOrExpr::Expr(e) => expr_to_json(e)
            },
            "expression": matches!(&*arrow.body, BlockStmtOrExpr::Expr(_))
        }),
        Expr::Bin(BinExpr { left, right, op, .. }) => json!({
            "type": "BinaryExpression",
            "left": expr_to_json(left),
            "right": expr_to_json(right),
            "operator": op.as_str()
        }),
        Expr::Call(CallExpr { callee, args, .. }) => json!({
            "type": "CallExpression",
            "callee": match callee {
                Callee::Expr(e) => expr_to_json(e),
                _ => json!(null),
            },
            "arguments": args.iter().map(|a| expr_to_json(&a.expr)).collect::<Vec<_>>()
        }),
        Expr::Member(MemberExpr { obj, prop, .. }) => json!({
            "type": "MemberExpression",
            "object": expr_to_json(obj),
            "property": match prop {
                MemberProp::Ident(id) => json!({ "type": "Identifier", "name": id.sym.as_str().to_string() }),
                MemberProp::Computed(c) => expr_to_json(&c.expr),
                _ => json!({ "type": "Unsupported" }),
            },
            "computed": matches!(prop, MemberProp::Computed(_))
        }),
        Expr::Unary(UnaryExpr { op, arg, .. }) => json!({
            "type": "UnaryExpression",
            "operator": op.as_str(),
            "argument": expr_to_json(arg),
            "prefix": true
        }),
        Expr::Paren(ParenExpr { expr, .. }) => json!({
            "type": "ParenExpression",
            "expression": expr_to_json(expr)
        }),
        _ => json!({ "type": "Unsupported", "detail": format!("{:?}", expr) })
    }
}

pub fn stmt_to_json(stmt: &Stmt) -> Value {
    match stmt {
        Stmt::Expr(ExprStmt { expr, .. }) => json!({
            "type": "ExpressionStatement",
            "expression": expr_to_json(expr)
        }),
        Stmt::Block(BlockStmt { stmts, .. }) => json!({
            "type": "BlockStatement",
            "body": stmts.iter().map(stmt_to_json).collect::<Vec<_>>()
        }),
        Stmt::If(IfStmt { test, cons, alt, .. }) => json!({
            "type": "IfStatement",
            "test": expr_to_json(test),
            "consequent": stmt_to_json(cons),
            "alternate": alt.as_ref().map(|a| stmt_to_json(a))
        }),
        Stmt::Return(ReturnStmt { arg, .. }) => json!({
            "type": "ReturnStatement",
            "argument": arg.as_ref().map(|a| expr_to_json(a))
        }),
        _ => json!({ "type": "Unsupported", "detail": format!("{:?}", stmt) })
    }
}

pub fn json_to_expr(val: Value, target_span: Span) -> Option<Box<Expr>> {
    let obj = val.as_object()?;
    let node_type = obj.get("type")?.as_str()?;

    match node_type {
        "Identifier" => {
            let name = obj.get("name")?.as_str()?;
            Some(Box::new(Expr::Ident(Ident::new(name.into(), target_span, SyntaxContext::empty()))))
        }
        "Literal" => {
            let v = obj.get("value")?;
            if v.is_number() {
                Some(Box::new(Expr::Lit(Lit::Num(Number {
                    value: v.as_f64()?,
                    span: target_span,
                    raw: None,
                }))))
            } else if v.is_string() {
                Some(Box::new(Expr::Lit(Lit::Str(Str {
                    value: v.as_str()?.into(),
                    span: target_span,
                    raw: None,
                }))))
            } else {
                None
            }
        }
        "ArrayExpression" => {
            let mut elems = Vec::new();
            for e in obj.get("elements")?.as_array()? {
                if e.is_null() {
                    elems.push(None);
                } else {
                    elems.push(Some(ExprOrSpread { spread: None, expr: json_to_expr(e.clone(), target_span)? }));
                }
            }
            Some(Box::new(Expr::Array(ArrayLit { span: target_span, elems })))
        }
        "ArrowFunctionExpression" => {
            let params = obj.get("params")?.as_array()?.iter().map(|p| {
                let name = p.get("name").unwrap().as_str().unwrap();
                Pat::Ident(BindingIdent {
                    id: Ident::new(name.into(), target_span, Default::default()),
                    type_ann: None,
                })
            }).collect();
            
            let is_expr = obj.get("expression").and_then(|v| v.as_bool()).unwrap_or(false);
            let body_val = obj.get("body")?.clone();
            
            let body = Box::new(if is_expr {
                BlockStmtOrExpr::Expr(json_to_expr(body_val, target_span)?)
            } else {
                match json_to_stmt(body_val, target_span)? {
                    Stmt::Block(b) => BlockStmtOrExpr::BlockStmt(b),
                    _ => return None
                }
            });

            Some(Box::new(Expr::Arrow(ArrowExpr {
                span: target_span,
                params,
                body,
                is_async: false,
                is_generator: false,
                type_params: None,
                return_type: None,
                ctxt: Default::default(),
            })))
        }
        "BinaryExpression" => {
            let left = json_to_expr(obj.get("left")?.clone(), target_span)?;
            let right = json_to_expr(obj.get("right")?.clone(), target_span)?;
            let op_str = obj.get("operator")?.as_str()?;
            let op = match op_str {
                "===" => BinaryOp::EqEqEq,
                "==" => BinaryOp::EqEq,
                "+" => BinaryOp::Add,
                "-" => BinaryOp::Sub,
                "*" => BinaryOp::Mul,
                "/" => BinaryOp::Div,
                ">" => BinaryOp::Gt,
                "<" => BinaryOp::Lt,
                _ => return None,
            };
            Some(Box::new(Expr::Bin(BinExpr {
                span: target_span,
                op,
                left,
                right,
            })))
        }
        "UnaryExpression" => {
            let arg = json_to_expr(obj.get("argument")?.clone(), target_span)?;
            let op_str = obj.get("operator")?.as_str()?;
            let op = match op_str {
                "!" => UnaryOp::Bang,
                "~" => UnaryOp::Tilde,
                "-" => UnaryOp::Minus,
                "+" => UnaryOp::Plus,
                _ => return None,
            };
            Some(Box::new(Expr::Unary(UnaryExpr {
                span: target_span,
                op,
                arg,
            })))
        }
        "MemberExpression" => {
            let obj_node = json_to_expr(obj.get("object")?.clone(), target_span)?;
            let prop_val = obj.get("property")?;
            let computed = obj.get("computed")?.as_bool()?;
            
            let prop = if computed {
                MemberProp::Computed(ComputedPropName {
                    span: target_span,
                    expr: json_to_expr(prop_val.clone(), target_span)?,
                })
            } else {
                let name = prop_val.get("name")?.as_str()?;
                MemberProp::Ident(IdentName::new(name.into(), target_span))
            };

            Some(Box::new(Expr::Member(MemberExpr {
                span: target_span,
                obj: obj_node,
                prop,
            })))
        }
        "CallExpression" => {
            let callee_val = obj.get("callee")?;
            let callee = Callee::Expr(json_to_expr(callee_val.clone(), target_span)?);
            let args_val = obj.get("arguments")?.as_array()?;
            let mut args = Vec::new();
            for v in args_val {
                args.push(ExprOrSpread {
                    spread: None,
                    expr: json_to_expr(v.clone(), target_span)?,
                });
            }
            Some(Box::new(Expr::Call(CallExpr {
                span: target_span,
                ctxt: SyntaxContext::empty(),
                callee,
                args,
                type_args: None,
                ..Default::default()
            })))
        }
        "ParenExpression" => {
            let expr = json_to_expr(obj.get("expression")?.clone(), target_span)?;
            Some(Box::new(Expr::Paren(ParenExpr {
                span: target_span,
                expr,
            })))
        }
        _ => {
            None
        }
    }
}

pub fn json_to_stmt(val: Value, target_span: Span) -> Option<Stmt> {
    let obj = val.as_object()?;
    let node_type = obj.get("type")?.as_str()?;

    match node_type {
        "BlockStatement" => {
            let body_val = obj.get("body")?.as_array()?;
            let mut stmts = Vec::new();
            for v in body_val {
                if let Some(s) = json_to_stmt(v.clone(), target_span) {
                    stmts.push(s);
                }
            }
            Some(Stmt::Block(BlockStmt {
                span: target_span,
                stmts,
                ..Default::default()
            }))
        }
        "IfStatement" => {
            let test = json_to_expr(obj.get("test")?.clone(), target_span)?;
            let cons = Box::new(json_to_stmt(obj.get("consequent")?.clone(), target_span)?);
            let alt = if let Some(a) = obj.get("alternate") {
                if !a.is_null() {
                    Some(Box::new(json_to_stmt(a.clone(), target_span)?))
                } else {
                    None
                }
            } else {
                None
            };
            Some(Stmt::If(IfStmt {
                span: target_span,
                test,
                cons,
                alt,
                ..Default::default()
            }))
        }
        "ExpressionStatement" => {
            let expr = json_to_expr(obj.get("expression")?.clone(), target_span)?;
            Some(Stmt::Expr(ExprStmt {
                span: target_span,
                expr,
            }))
        }
        "CallExpression" => {
            let expr = json_to_expr(val.clone(), target_span)?;
            Some(Stmt::Expr(ExprStmt {
                span: target_span,
                expr,
            }))
        }
        _ => {
            None
        }
    }
}
