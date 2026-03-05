//! # Sout's Game Script (SGS)
//!
//! SGS is a domain-specific language (DSL) designed for SOUT ENGINE.
//! It provides a concise and expressive syntax for defining entities, components,
//! and systems (ECS) within a game engine.
//!

pub mod ast;

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
                            return_ty = Some(block_pair.as_str().replace(" ", "").replace("->", ""));
                            block_pair = inner.next().unwrap();
                        }

                        let statements = block_pair.into_inner().map(parse_stmt).collect();

                        system_fns.push(FunctionDef {
                            name: fn_name.to_string(),
                            params,
                            return_ty,
                            statements,
                        });
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
    pair.into_inner().map(|p| {
        let mut inner = p.into_inner();
        FnParam {
            name: inner.next().unwrap().as_str().to_string(),
            ty: inner.next().unwrap().as_str().replace(" ", ""),
        }
    }).collect()
}

fn parse_stmt(pair: pest::iterators::Pair<Rule>) -> Stmt {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::let_stmt => {
            let mut parts = inner.into_inner();
            let name = parts.next().unwrap().as_str().to_string();
            let expr = parse_expr(parts.next().unwrap());
            Stmt::Let { name, value: expr }
        }
        Rule::assign_stmt => {
            let mut parts = inner.into_inner();
            let target_path = parts.next().unwrap().into_inner().map(|i| i.as_str().to_string()).collect();
            let op = parts.next().unwrap().as_str().to_string();
            let value = parse_expr(parts.next().unwrap());
            Stmt::Assign(AssignStmt { target_path, op, value })
        }
        Rule::expr_stmt => {
            Stmt::Expr(parse_expr(inner.into_inner().next().unwrap()))
        }
        _ => unreachable!(),
    }
}

fn parse_expr(pair: pest::iterators::Pair<Rule>) -> Expr {
    match pair.as_rule() {
        // 一层一层剥开我的心
        Rule::expr => parse_expr(pair.into_inner().next().unwrap()),

        Rule::number => Expr::Number(pair.as_str().parse().unwrap()),
        Rule::string_lit => Expr::StringLit(pair.into_inner().next().unwrap().as_str().to_string()),
        Rule::path => Expr::Path(pair.into_inner().map(|i| i.as_str().to_string()).collect()),

        Rule::closure => {
            let mut inner = pair.into_inner();
            let params = parse_fn_params(inner.next().unwrap());
            let body = inner.next().unwrap().into_inner().map(parse_stmt).collect();
            Expr::Closure { params, body }
        }

        Rule::call => {
            let mut inner = pair.into_inner();
            let path_pair = inner.next().unwrap();
            let target = Box::new(Expr::Path(path_pair.into_inner().map(|i| i.as_str().to_string()).collect()));
            let args = inner.map(parse_expr).collect();
            Expr::Call { target, args }
        }

        _ => unreachable!("parse_expr 爆了: {:?}", pair.as_rule()),
    }
}
