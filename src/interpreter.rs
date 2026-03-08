use crate::ast::*;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
    pub value: Value,
    pub is_mut: bool,
}

pub enum ControlFlow {
    None,
    Return(Value),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    String(String),
    Void,
    Closure {
        params: Vec<String>,
        body: Vec<Spanned<Stmt>>,
        captured_env: Vec<HashMap<String, Rc<RefCell<Variable>>>>,
    },
}

pub struct Environment {
    pub scopes: Vec<HashMap<String, Rc<RefCell<Variable>>>>,
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

impl Environment {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()], // 默认放入一个全局作用域
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop().expect("不能弹出全局作用域");
    }

    pub fn define(&mut self, name: String, value: Value, is_mut: bool) {
        let current_scope = self.scopes.last_mut().unwrap();
        current_scope.insert(name, Rc::new(RefCell::new(Variable { value, is_mut })));
    }

    fn find_var(&self, name: &str) -> Option<Rc<RefCell<Variable>>> {
        for scope in self.scopes.iter().rev() {
            if let Some(var) = scope.get(name) {
                return Some(var.clone());
            }
        }
        None
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        self.find_var(name).map(|rc| rc.borrow().value.clone())
    }

    pub fn get_val(&self, name: &str) -> Option<Value> {
        self.get(name)
    }

    pub fn set(&mut self, name: &str, value: Value) -> Result<(), String> {
        if let Some(rc) = self.find_var(name) {
            let mut var = rc.borrow_mut();

            // 核心：权限校验！
            if !var.is_mut {
                return Err(format!(
                    "不可变变量 '{}' 无法被重新赋值，请使用 let mut 声明",
                    name
                ));
            }

            var.value = value;
            Ok(())
        } else {
            Err(format!("未定义的变量: {}", name))
        }
    }
}

// 解释器核心
pub struct Interpreter {
    pub env: Environment,
}

impl Default for Interpreter {
     fn default() -> Self {
         Self::new()
     }
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
                        .ok_or_else(|| format!("找不到变量: {}", path[0]))
                } else {
                    Err(format!("暂不支持复杂路径的读取: {:?}", path))
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
                if let Expr::Path(path) = &**target
                    && path.len() == 1
                    && path[0] == "println"
                {
                    let mut outputs = Vec::new();
                    for arg in args {
                        let val = self.eval_expr(arg)?;
                        match val {
                            Value::Number(n) => outputs.push(n.to_string()),
                            Value::String(s) => outputs.push(s),
                            Value::Closure { .. } => outputs.push("<closure>".to_string()),
                            Value::Void => outputs.push("void".to_string()),
                        }
                    }
                    println!("{}", outputs.join(" "));
                    return Ok(Value::Void);
                }

                let target_val = self.eval_expr(target)?;

                if let Value::Closure {
                    params,
                    body,
                    captured_env,
                } = target_val
                {
                    if args.len() != params.len() {
                        return Err("参数数量不匹配".to_string());
                    }

                    let mut arg_values = Vec::new();
                    for arg in args {
                        arg_values.push(self.eval_expr(arg)?);
                    }

                    let old_scopes = std::mem::replace(&mut self.env.scopes, captured_env);
                    self.env.push_scope();

                    for (name, val) in params.into_iter().zip(arg_values) {
                        self.env.define(name, val, false);
                    }

                    let return_value = self.execute_block(&body).expect("execute_block err");

                    self.env.pop_scope();
                    self.env.scopes = old_scopes;

                    return Ok(return_value);
                }
                Err("调用的目标不是一个可执行的函数或闭包".to_string())
            }
            Expr::Closure { params, body } => {
                let param_names = params.iter().map(|p| p.name.clone()).collect();
                Ok(Value::Closure {
                    params: param_names,
                    body: body.clone(),
                    captured_env: self.env.scopes.clone(),
                })
            }
            Expr::StringInterp(parts) => {
                let mut result_str = String::new();
                for part in parts {
                    let val = self.eval_expr(part)?;
                    match val {
                        Value::Number(n) => result_str.push_str(&n.to_string()),
                        Value::String(s) => result_str.push_str(&s),
                        Value::Void => result_str.push_str("void"),
                        Value::Closure { .. } => result_str.push_str("<closure>"),
                    }
                }
                Ok(Value::String(result_str))
            }
        }
    }

    // 单条执行
    pub fn eval_stmt(&mut self, stmt: &Spanned<Stmt>) -> Result<ControlFlow, (String, Span)> {
        let attach_span = |err: String| (err, stmt.span.clone());

        match &stmt.node {
            Stmt::Let {
                is_mut,
                name,
                value,
            } => {
                let val = self.eval_expr(value).map_err(attach_span)?;
                self.env.define(name.clone(), val, *is_mut);
                Ok(ControlFlow::None)
            }
            Stmt::Assign(AssignStmt {
                target_path,
                op,
                value,
            }) => {
                let right_val = self.eval_expr(value).map_err(attach_span)?;
                let name = &target_path[0];
                let current_val = self
                    .env
                    .get_val(name)
                    .ok_or_else(|| (format!("未定义的变量: {}", name), stmt.span.clone()))?;

                let var = self.env.find_var(name).unwrap();
                if !var.borrow().is_mut {
                    return Err((
                        format!("不可变变量 '{}' 无法被重新赋值", name),
                        stmt.span.clone(),
                    ));
                }

                let new_val = match (current_val, op.as_str(), right_val) {
                    (_, "=", v) => v,
                    (Value::Number(l), "+=", Value::Number(r)) => Value::Number(l + r),
                    (Value::Number(l), "-=", Value::Number(r)) => Value::Number(l - r),
                    (Value::Number(l), "*=", Value::Number(r)) => Value::Number(l * r),
                    (Value::Number(l), "/=", Value::Number(r)) => {
                        if r == 0.0 {
                            return Err(("除数不能为0".to_string(), stmt.span.clone()));
                        }
                        Value::Number(l / r)
                    }
                    _ => return Err(("无效的赋值操作或类型不匹配".to_string(), stmt.span.clone())),
                };

                self.env.set(name, new_val).map_err(attach_span)?;
                Ok(ControlFlow::None)
            }
            Stmt::Expr(expr) => {
                self.eval_expr(expr).map_err(attach_span)?;
                Ok(ControlFlow::None)
            }
            Stmt::Return(Some(expr)) => {
                let val = self.eval_expr(expr).map_err(attach_span)?;
                Ok(ControlFlow::Return(val))
            }
            Stmt::Return(None) => Ok(ControlFlow::Return(Value::Void)),
            Stmt::ImplicitReturn(expr) => {
                let val = self.eval_expr(expr).map_err(attach_span)?;
                Ok(ControlFlow::Return(val))
            }
            Stmt::Block(stmts) => {
                self.env.push_scope();
                let mut res = ControlFlow::None;

                for s in stmts {
                    res = self.eval_stmt(s)?;
                    if matches!(res, ControlFlow::Return(_)) {
                        break;
                    }
                }

                self.env.pop_scope();
                Ok(res) // 把控制流抛给外层
            }
        }
    }

    /// 运行整个函数体
    pub fn execute_function(&mut self, func: &FunctionDef) -> Result<Value, (String, Span)> {
        self.env.push_scope();
        let result = self.execute_block(&func.statements);
        self.env.pop_scope();
        result
    }

    pub fn execute_block(&mut self, body: &[Spanned<Stmt>]) -> Result<Value, (String, Span)> {
        for stmt in body {
            match self.eval_stmt(stmt)? {
                ControlFlow::Return(val) => return Ok(val),
                ControlFlow::None => continue,
            }
        }
        Ok(Value::Void)
    }
}
