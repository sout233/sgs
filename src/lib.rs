// src/lib.rs
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

    // Fix: Use a macro instead of a closure.
    // A closure holds a mutable borrow for its entire lifetime.
    // A macro expands at the call site, avoiding borrow conflict.
    macro_rules! finalize_current_node {
        () => {
            if !current_type.is_empty() {
                match current_type.as_str() {
                    "Entity" => ast_nodes.push(SgsNode::EntityDef(EntityDef {
                        name: current_name.clone(),
                        // std::mem::take leaves an empty Vec in place of the old one
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
                            // Close out the previous node before starting a new one
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
                        let first = inner.next().unwrap().as_str();

                        let (is_mut, name) = if first == "mut" {
                            (true, inner.next().unwrap().as_str().to_string())
                        } else {
                            (false, first.to_string())
                        };

                        system_reqs.push(RequiredComponent { is_mut, name });
                    }
                    Rule::fn_stmt => {
                        let mut inner = stmt_inner.into_inner();
                        let fn_name = inner.next().unwrap().as_str();

                        let block = inner.last().unwrap();
                        let mut assigns = Vec::new();

                        for block_stmt in block.into_inner() {
                            let assign = block_stmt.into_inner().next().unwrap();
                            let mut assign_parts = assign.into_inner();

                            let path_pair = assign_parts.next().unwrap();
                            let target_path = path_pair
                                .into_inner()
                                .map(|i| i.as_str().to_string())
                                .collect();

                            let op = assign_parts.next().unwrap().as_str().to_string();

                            let expr_pair = assign_parts.next().unwrap().into_inner().next().unwrap();
                            let expr = match expr_pair.as_rule() {
                                Rule::number => Expr::Number(expr_pair.as_str().parse().unwrap()),
                                Rule::path => Expr::Path(
                                    expr_pair.into_inner().map(|i| i.as_str().to_string()).collect(),
                                ),
                                _ => unreachable!(),
                            };

                            assigns.push(AssignStmt {
                                target_path,
                                op,
                                value: expr,
                            });
                        }

                        system_fns.push(FunctionDef {
                            name: fn_name.to_string(),
                            return_ty: None,
                            statements: assigns,
                        });
                    }
                    _ => {}
                }
            }
            Rule::EOI => {
                // Finalize the very last node in the file
                finalize_current_node!();
            }
            _ => unreachable!(),
        }
    }

    Ok(ast_nodes)
}
