//! # Sout's Game Script (SGS)
//!
//! SGS is a domain-specific language (DSL) designed for SOUT ENGINE.
//! It provides a concise and expressive syntax for defining entities, components,
//! and systems (ECS) within a game engine.
//!

// lib.rs

pub mod analyzer;
pub mod ast;
pub mod interpreter;

use ast::*;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "sgs.pest"]
pub struct SgsParser;

pub fn parse_program(source: &str) -> Result<Vec<SgsNode>, pest::error::Error<Rule>> {
    let mut ast_nodes = Vec::new();
    let program = SgsParser::parse(Rule::program, source)?.next().unwrap();

    let mut current_type = String::new();
    let mut current_name = String::new();

    let mut entity_components = Vec::new();
    let mut component_params = Vec::new();
    let mut system_reqs = Vec::new();
    let mut system_fns = Vec::new();

    macro_rules! finalize_current_node {
        () => {
            if !current_type.is_empty() {
                match current_type.as_str() {
                    "Entity" => ast_nodes.push(SgsNode::EntityDef(EntityDef {
                        name: current_name.clone(),
                        components: std::mem::take(&mut entity_components),
                    })),
                    "Component" => ast_nodes.push(SgsNode::ComponentDef(ComponentDef {
                        name: current_name.clone(),
                        params: std::mem::take(&mut component_params),
                    })),
                    "System" => ast_nodes.push(SgsNode::SystemDef(SystemDef {
                        name: current_name.clone(),
                        required_components: std::mem::take(&mut system_reqs),
                        functions: std::mem::take(&mut system_fns),
                    })),
                    _ => {}
                }
            }
        };
    }

    for pair in program.into_inner() {
        match pair.as_rule() {
            Rule::stmt => {
                let stmt_inner = pair.into_inner().next().unwrap();
                match stmt_inner.as_rule() {
                    Rule::annotation => {
                        let mut inner = stmt_inner.into_inner();
                        let key = inner.next().unwrap().as_str();
                        let value = inner.next().unwrap().as_str();

                        if key == "type" {
                            finalize_current_node!();
                            current_type = value.to_string();
                            current_name = String::new();
                        } else if key == "name" {
                            current_name = value.to_string();
                        }
                    }
                    Rule::mount_stmt => {
                        let component_name = stmt_inner.into_inner().next().unwrap().as_str();
                        entity_components.push(component_name.to_string());
                    }
                    Rule::param_stmt => {
                        let mut inner = stmt_inner.into_inner();
                        let param_name = inner.next().unwrap().as_str();
                        let param_ty = inner.next().unwrap().as_str();
                        component_params.push(Param {
                            name: param_name.to_string(),
                            ty: param_ty.to_string(),
                        });
                    }
                    Rule::require_stmt => {
                        let mut inner = stmt_inner.into_inner();
                        let first_pair = inner.next().unwrap();

                        // 第一个是mut那便下一个是ident
                        let (is_mut, name) = if first_pair.as_rule() == Rule::is_mut {
                            (true, inner.next().unwrap().as_str().to_string())
                        } else {
                            (false, first_pair.as_str().to_string())
                        };

                        system_reqs.push(RequiredComponent { is_mut, name });
                    }
                    Rule::fn_stmt => {
                        let mut inner = stmt_inner.into_inner();
                        let fn_name = inner.next().unwrap().as_str();

                        let params_pair = inner.next().unwrap();
                        let params = parse_fn_params(params_pair);

                        let mut return_ty = None;
                        let mut block_pair = inner.next().unwrap();

                        if block_pair.as_rule() == Rule::return_ty {
                            return_ty =
                                Some(block_pair.as_str().replace(" ", "").replace("->", ""));
                            block_pair = inner.next().unwrap();
                        }

                        let statements = parse_block(block_pair);

                        system_fns.push(FunctionDef {
                            name: fn_name.to_string(),
                            params,
                            return_ty,
                            statements,
                        });
                    }
                    Rule::struct_def => {
                        let mut inner = stmt_inner.into_inner();
                        let name = inner.next().unwrap().as_str().to_string();

                        let mut fields = Vec::new();
                        for field_pair in inner {
                            let mut f_inner = field_pair.into_inner();
                            let f_name = f_inner.next().unwrap().as_str().to_string();
                            let f_type = f_inner.next().unwrap().as_str().to_string();

                            fields.push((f_name, f_type));
                        }

                        ast_nodes.push(SgsNode::StructDef(StructDef { name, fields }));
                    }
                    Rule::extern_fn_stmt => {
                        let mut inner = stmt_inner.into_inner();
                        let fn_name = inner.next().unwrap().as_str();

                        let params_pair = inner.next().unwrap();
                        let params = parse_fn_params(params_pair);

                        let mut return_ty = None;
                        if let Some(ret_pair) = inner.next() {
                            return_ty = Some(ret_pair.as_str().replace(" ", "").replace("->", ""));
                        }

                        ast_nodes.push(SgsNode::ExternFunctionDef(ExternFunctionDef {
                            name: fn_name.to_string(),
                            params,
                            return_ty,
                        }));
                    }
                    _ => {}
                }
            }
            Rule::EOI => {
                finalize_current_node!();
            }
            _ => unreachable!(),
        }
    }

    Ok(ast_nodes)
}

fn parse_fn_params(pair: pest::iterators::Pair<Rule>) -> Vec<FnParam> {
    pair.into_inner()
        .map(|p| {
            let mut inner = p.into_inner();
            let mut next_pair = inner.next().unwrap();
            let mut is_mut = false;

            if next_pair.as_rule() == Rule::is_mut {
                is_mut = true;
                next_pair = inner.next().unwrap();
            }

            FnParam {
                is_mut,
                name: next_pair.as_str().to_string(),
                ty: inner.next().unwrap().as_str().replace(" ", ""),
            }
        })
        .collect()
}

fn parse_stmt(pair: pest::iterators::Pair<Rule>) -> Spanned<Stmt> {
    let span = pair.as_span();
    let byte_range = span.start()..span.end();

    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::let_stmt => {
            let mut parts = inner.into_inner();

            let mut is_mut = false;
            let mut next_pair = parts.next().unwrap();

            if next_pair.as_rule() == Rule::is_mut {
                is_mut = true;
                next_pair = parts.next().unwrap();
            }

            let name = next_pair.as_str().to_string();
            let expr = parse_expr(parts.next().unwrap());

            Spanned {
                node: Stmt::Let {
                    is_mut,
                    name,
                    value: expr,
                },
                span: byte_range,
            }
        }
        Rule::assign_stmt => {
            let mut parts = inner.into_inner();
            let target_path = parts
                .next()
                .unwrap()
                .into_inner()
                .map(|i| i.as_str().to_string())
                .collect();

            let mut next_pair = parts.next().unwrap();
            let mut index = None;
            if next_pair.as_rule() == Rule::expr {
                index = Some(parse_expr(next_pair));
                next_pair = parts.next().unwrap();
            }

            let op = next_pair.as_str().to_string();
            let value = parse_expr(parts.next().unwrap());

            Spanned {
                node: Stmt::Assign(AssignStmt {
                    target_path,
                    index,
                    op,
                    value,
                }),
                span: byte_range,
            }
        }
        Rule::expr_stmt => Spanned {
            node: Stmt::Expr(parse_expr(inner.into_inner().next().unwrap())),
            span: byte_range,
        },
        Rule::return_stmt => {
            let mut parts = inner.into_inner();
            let next_pair = parts.next().unwrap();

            if next_pair.as_rule() == Rule::expr {
                Spanned {
                    node: Stmt::Return(Some(parse_expr(next_pair))),
                    span: byte_range,
                }
            } else {
                Spanned {
                    node: Stmt::Return(None),
                    span: byte_range,
                }
            }
        }
        Rule::block => Spanned {
            node: Stmt::Block(parse_block(inner)),
            span: byte_range,
        },
        Rule::if_stmt => Spanned {
            node: parse_if_internal(inner.into_inner()),
            span: byte_range,
        },
        Rule::while_stmt => {
            let mut parts = inner.into_inner();
            let condition = parse_expr(parts.next().unwrap());
            let body = parse_block(parts.next().unwrap());
            Spanned {
                node: Stmt::While { condition, body },
                span: byte_range,
            }
        }
        Rule::for_stmt => {
            let mut parts = inner.into_inner();
            let mut is_mut = false;

            let mut next_pair = parts.next().unwrap();

            if next_pair.as_rule() == Rule::is_mut {
                is_mut = true;
                next_pair = parts.next().unwrap();
            }

            let item_name = next_pair.as_str().to_string();
            let iterable = parse_expr(parts.next().unwrap());
            let body = parse_block(parts.next().unwrap());

            Spanned {
                node: Stmt::For {
                    is_mut,
                    item_name,
                    iterable,
                    body,
                },
                span: byte_range,
            }
        }
        Rule::break_stmt => Spanned {
            node: Stmt::Break,
            span: byte_range,
        },
        Rule::continue_stmt => Spanned {
            node: Stmt::Continue,
            span: byte_range,
        },
        _ => unreachable!(),
    }
}

fn parse_if_internal(mut parts: pest::iterators::Pairs<Rule>) -> Stmt {
    let condition = parse_expr(parts.next().unwrap());
    let then_branch = parse_block(parts.next().unwrap());

    let else_branch = parts.next().map(|else_pair| {
        let span = else_pair.as_span();
        let byte_range = span.start()..span.end();

        let node = match else_pair.as_rule() {
            Rule::if_stmt => parse_if_internal(else_pair.into_inner()),
            Rule::block => Stmt::Block(parse_block(else_pair)),
            _ => unreachable!(),
        };
        Box::new(Spanned {
            node,
            span: byte_range,
        })
    });

    Stmt::If {
        condition,
        then_branch,
        else_branch,
    }
}

fn parse_expr(pair: pest::iterators::Pair<Rule>) -> Expr {
    match pair.as_rule() {
        Rule::expr => {
            let mut inner = pair.into_inner();
            let mut left = parse_math_expr(inner.next().unwrap());

            while let Some(op_pair) = inner.next() {
                let op = op_pair.as_str().to_string();
                let right = parse_math_expr(inner.next().unwrap());
                left = Expr::BinaryOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                };
            }
            left
        }
        Rule::math_expr => parse_math_expr(pair),
        Rule::term => parse_term(pair),
        Rule::factor => parse_factor(pair),
        _ => unreachable!("parse_expr 爆了: {:?}", pair.as_rule()),
    }
}

fn parse_math_expr(pair: pest::iterators::Pair<Rule>) -> Expr {
    let mut inner = pair.into_inner();
    let mut left = parse_term(inner.next().unwrap());

    while let Some(op_pair) = inner.next() {
        let op = op_pair.as_str().to_string();
        let right = parse_term(inner.next().unwrap());
        left = Expr::BinaryOp {
            left: Box::new(left),
            op,
            right: Box::new(right),
        };
    }
    left
}

fn parse_term(pair: pest::iterators::Pair<Rule>) -> Expr {
    let mut inner = pair.into_inner();
    let mut left = parse_factor(inner.next().unwrap());

    while let Some(op_pair) = inner.next() {
        let op = op_pair.as_str().to_string();
        let right = parse_factor(inner.next().unwrap());
        left = Expr::BinaryOp {
            left: Box::new(left),
            op,
            right: Box::new(right),
        };
    }
    left
}

fn parse_factor(pair: pest::iterators::Pair<Rule>) -> Expr {
    let mut inner_pairs = pair.into_inner();

    let base_inner = inner_pairs.next().unwrap();

    let mut current_expr = match base_inner.as_rule() {
        Rule::number => Expr::Number(base_inner.as_str().parse().unwrap()),
        Rule::bool_lit => Expr::Bool(base_inner.as_str() == "true"),
        Rule::string_lit => {
            Expr::StringLit(base_inner.into_inner().next().unwrap().as_str().to_string())
        }
        Rule::path => Expr::Path(
            base_inner
                .into_inner()
                .map(|i| i.as_str().to_string())
                .collect(),
        ),
        Rule::closure => {
            let mut closure_inner = base_inner.into_inner();
            let params = parse_fn_params(closure_inner.next().unwrap());
            let body = parse_block(closure_inner.next().unwrap());
            Expr::Closure { params, body }
        }
        Rule::call => {
            let mut parts = base_inner.into_inner();

            let path_pair = parts.next().unwrap();
            let mut path: Vec<String> = path_pair
                .into_inner()
                .map(|i| i.as_str().to_string())
                .collect();

            let mut args = Vec::new();
            for p in parts {
                args.push(parse_expr(p));
            }

            if path.len() > 1 {
                let method = path.pop().unwrap();
                Expr::MethodCall {
                    target: Box::new(Expr::Path(path)),
                    method,
                    args,
                }
            } else {
                Expr::Call {
                    target: Box::new(Expr::Path(path)),
                    args,
                }
            }
        }
        Rule::expr => parse_expr(base_inner),
        Rule::interp_string => {
            let mut parts = Vec::new();
            for part in base_inner.into_inner() {
                match part.as_rule() {
                    Rule::interp_text => parts.push(Expr::StringLit(part.as_str().to_string())),
                    Rule::interp_expr => parts.push(parse_expr(part.into_inner().next().unwrap())),
                    _ => unreachable!("意外的内插规则: {:?}", part.as_rule()),
                }
            }
            Expr::StringInterp(parts)
        }
        Rule::array_lit => {
            let elements = base_inner.into_inner().map(parse_expr).collect();
            Expr::Array(elements)
        }
        Rule::index_access => {
            let mut inner_parts = base_inner.into_inner();
            let path = Expr::Path(
                inner_parts
                    .next()
                    .unwrap()
                    .into_inner()
                    .map(|i| i.as_str().to_string())
                    .collect(),
            );
            let index = parse_expr(inner_parts.next().unwrap());
            Expr::Index {
                target: Box::new(path),
                index: Box::new(index),
            }
        }
        Rule::struct_init => {
            let mut inner_parts = base_inner.into_inner();
            let name = inner_parts.next().unwrap().as_str().to_string();

            let mut fields = Vec::new();
            for field_pair in inner_parts {
                let mut f_inner = field_pair.into_inner();
                let f_name = f_inner.next().unwrap().as_str().to_string();
                let f_expr = parse_expr(f_inner.next().unwrap());

                fields.push((f_name, f_expr));
            }

            Expr::StructInit { name, fields }
        }
        Rule::dict_lit => {
            let mut entries = Vec::new();
            for entry_pair in base_inner.into_inner() {
                let mut parts = entry_pair.into_inner();
                let key_pair = parts.next().unwrap();
                let key = match key_pair.as_rule() {
                    Rule::ident => key_pair.as_str().to_string(),
                    Rule::string_lit => key_pair.into_inner().next().unwrap().as_str().to_string(),
                    _ => unreachable!(),
                };
                let value = parse_expr(parts.next().unwrap());
                entries.push((key, Box::new(value)));
            }
            Expr::Dict(entries)
        }
        _ => unreachable!("parse_factor 爆了: {:?}", base_inner.as_rule()),
    };

    for next_pair in inner_pairs {
        if next_pair.as_rule() == Rule::type_name {
            let ty_name = next_pair.as_str().to_string();
            current_expr = Expr::Cast {
                expr: Box::new(current_expr),
                ty_name,
            };
        }
    }

    current_expr
}

fn parse_block(pair: pest::iterators::Pair<Rule>) -> Vec<Spanned<Stmt>> {
    let mut stmts = Vec::new();
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::block_stmt => {
                stmts.push(parse_stmt(inner));
            }
            Rule::expr => {
                let span = inner.as_span();
                let byte_range = span.start()..span.end();

                stmts.push(Spanned {
                    node: Stmt::ImplicitReturn(parse_expr(inner)),
                    span: byte_range,
                });
            }
            _ => unreachable!(),
        }
    }
    stmts
}
