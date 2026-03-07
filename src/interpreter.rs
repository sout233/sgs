use crate::ast::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    String(String),
    Void,
}

pub struct Environment {
    pub variables: HashMap<String, Value>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    pub fn define(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        self.variables.get(name).cloned()
    }

    pub fn set(&mut self, name: &str, value: Value) -> Result<(), String> {
        if self.variables.contains_key(name) {
            self.variables.insert(name.to_string(), value);
            Ok(())
        } else {
            Err(format!("var 404: {}", name))
        }
    }
}

// 解释器核心
pub struct Interpreter {
    pub env: Environment,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            env: Environment::new(),
        }
    }

    pub fn eval_expr(&mut self, expr: &Expr) -> Result<Value, String> {
        match expr {
            Expr::Number(n) => Ok(Value::Number(*n)),
            Expr::StringLit(s) => Ok(Value::String(s.clone())),
            Expr::Path(path) => {
                // TODO: 支持复杂变量路径
                if path.len() == 1 {
                    self.env
                        .get(&path[0])
                        .ok_or_else(|| format!("var 404: {}", path[0]))
                } else {
                    Err(format!("暂不支持复杂路径: {:?}", path))
                }
            }
            Expr::BinaryOp { left, op, right } => {
                let left_val = self.eval_expr(left)?;
                let right_val = self.eval_expr(right)?;

                match (left_val, op.as_str(), right_val) {
                    (Value::Number(l), "+", Value::Number(r)) => Ok(Value::Number(l + r)),
                    (Value::Number(l), "-", Value::Number(r)) => Ok(Value::Number(l - r)),
                    (Value::Number(l), "*", Value::Number(r)) => Ok(Value::Number(l * r)),
                    (Value::Number(l), "/", Value::Number(r)) => {
                        if r == 0.0 {
                            Err("divided by 0".to_string())
                        } else {
                            Ok(Value::Number(l / r))
                        }
                    }
                    _ => Err(format!("unsupported operation or type mismatch: {}", op)),
                }
            }
            Expr::Call { target, args } => {
                if let Expr::Path(path) = &**target {
                    if path.len() == 1 {
                        let func_name = &path[0];

                        if func_name == "println" {
                            let mut outputs = Vec::new();
                            for arg in args {
                                let val = self.eval_expr(arg)?;
                                match val {
                                    Value::Number(n) => outputs.push(n.to_string()),
                                    Value::String(s) => outputs.push(s),
                                    Value::Void => outputs.push("void".to_string()),
                                }
                            }
                            println!("{}", outputs.join(" "));
                            return Ok(Value::Void);
                        }

                        return Err(format!("找不到fn: {}", func_name));
                    }
                }
                Err("暂不支持复杂的函数调用目标".to_string())
            }
            _ => Err("该表达式类型的计算尚未实现".to_string()),
        }
    }

    // 单条执行
    pub fn eval_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Let { name, value } => {
                let val = self.eval_expr(value)?;
                // 存入
                self.env.define(name.clone(), val);
                Ok(())
            }
            Stmt::Assign(AssignStmt {
                target_path,
                op,
                value,
            }) => {
                if target_path.len() == 1 {
                    let name = &target_path[0];
                    let right_val = self.eval_expr(value)?;
                    let current_val = self
                        .env
                        .get(name)
                        .ok_or_else(|| format!("undefinitely var: {}", name))?;

                    let new_val = match (current_val, op.as_str(), right_val) {
                        (_, "=", v) => v, // 直接赋值
                        (Value::Number(l), "+=", Value::Number(r)) => Value::Number(l + r),
                        (Value::Number(l), "-=", Value::Number(r)) => Value::Number(l - r),
                        (Value::Number(l), "*=", Value::Number(r)) => Value::Number(l * r),
                        (Value::Number(l), "/=", Value::Number(r)) => Value::Number(l / r),
                        _ => return Err("无效的赋值操作或类型不匹配".to_string()),
                    };

                    self.env.set(name, new_val)?;
                    Ok(())
                } else {
                    Err("暂不支持复杂路径的赋值修改".to_string())
                }
            }
            Stmt::Expr(expr) => {
                self.eval_expr(expr)?;
                Ok(())
            }
        }
    }

    // 运行整个函数体
    pub fn execute_function(&mut self, func: &FunctionDef) -> Result<(), String> {
        for stmt in &func.statements {
            self.eval_stmt(stmt)?;
        }
        Ok(())
    }
}
