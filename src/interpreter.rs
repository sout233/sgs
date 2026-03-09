// interpreter.rs
/// 解释器
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
    Break,
    Continue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    String(String),
    Bool(bool),
    Void,
    Array(Rc<RefCell<Vec<Value>>>),
    Closure {
        params: Vec<String>,
        body: Vec<Spanned<Stmt>>,
        captured_env: Vec<HashMap<String, Rc<RefCell<Variable>>>>,
    },
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Void => write!(f, "void"),
            Value::Closure { .. } => write!(f, "<closure>"),
            Value::Array(arr) => {
                write!(f, "[")?;
                let elements = arr.borrow();
                for (i, val) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", val)?;
                }
                write!(f, "]")
            }
        }
    }
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
            scopes: vec![HashMap::new()],
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
            Expr::Bool(b) => Ok(Value::Bool(*b)),
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
                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;

                match op.as_str() {
                    "++" => {
                        if let (Value::String(ls), Value::String(rs)) = (&l, &r) {
                            Ok(Value::String(format!("{}{}", ls, rs)))
                        } else {
                            Err("字符串拼接 '++' 只能用于两个字符串".to_string())
                        }
                    }
                    "+" => {
                        if let (Value::Number(ln), Value::Number(rn)) = (&l, &r) {
                            Ok(Value::Number(ln + rn))
                        } else {
                            Err("加法只能用于数字".to_string())
                        }
                    }
                    "-" => {
                        if let (Value::Number(ln), Value::Number(rn)) = (&l, &r) {
                            Ok(Value::Number(ln - rn))
                        } else {
                            Err("减法只能用于数字".to_string())
                        }
                    }
                    "*" => {
                        if let (Value::Number(ln), Value::Number(rn)) = (&l, &r) {
                            Ok(Value::Number(ln * rn))
                        } else {
                            Err("乘法只能用于数字".to_string())
                        }
                    }
                    "/" => {
                        if let (Value::Number(ln), Value::Number(rn)) = (&l, &r) {
                            if *rn == 0.0 {
                                return Err("divided by 0".to_string());
                            }
                            Ok(Value::Number(ln / rn))
                        } else {
                            Err("除法只能用于数字".to_string())
                        }
                    }
                    "==" => Ok(Value::Bool(l == r)),
                    "!=" => Ok(Value::Bool(l != r)),
                    "<" | ">" | "<=" | ">=" => {
                        if let (Value::Number(ln), Value::Number(rn)) = (&l, &r) {
                            let res = match op.as_str() {
                                "<" => ln < rn,
                                ">" => ln > rn,
                                "<=" => ln <= rn,
                                ">=" => ln >= rn,
                                _ => unreachable!(),
                            };
                            Ok(Value::Bool(res))
                        } else {
                            Err("关系运算符只能用于数字".to_string())
                        }
                    }
                    _ => Err(format!("unsupported operation: {}", op)),
                }
            }
            Expr::Array(elements) => {
                let mut arr = Vec::new();
                for el in elements {
                    arr.push(self.eval_expr(el)?);
                }
                Ok(Value::Array(Rc::new(RefCell::new(arr))))
            }
            Expr::Index { target, index } => {
                let target_val = self.eval_expr(target)?;
                let index_val = self.eval_expr(index)?;

                if let Value::Number(n) = index_val {
                    if n < 0.0 || n.fract() != 0.0 {
                        return Err("数组索引必须是正整数".to_string());
                    }
                    let idx = n as usize;

                    if let Value::Array(arr) = target_val {
                        let arr_ref = arr.borrow();
                        if idx >= arr_ref.len() {
                            return Err(format!(
                                "索引越界：数组长度为 {}，但尝试访问 {}",
                                arr_ref.len(),
                                idx
                            ));
                        }
                        Ok(arr_ref[idx].clone())
                    } else {
                        Err("试图对非数组进行索引访问".to_string())
                    }
                } else {
                    Err("数组索引必须是数字".to_string())
                }
            }
            Expr::Call { target, args } => {
                if let Expr::Path(path) = &**target
                    && path.len() == 1
                    && (path[0] == "println" || path[0] == "print")
                {
                    let mut outputs = Vec::new();
                    for arg in args {
                        let val = self.eval_expr(arg)?;
                        outputs.push(val.to_string());
                    }

                    let out_str = outputs.join(" ");

                    if path[0] == "println" {
                        println!("{}", out_str);
                    } else {
                        use std::io::Write;
                        print!("{}", out_str);
                        std::io::stdout().flush().unwrap();
                    }
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
            Expr::MethodCall {
                target,
                method,
                args,
            } => {
                let target_val = self.eval_expr(target)?;

                match method.as_str() {
                    "len" => match target_val {
                        Value::Array(arr) => Ok(Value::Number(arr.borrow().len() as f64)),
                        Value::String(s) => Ok(Value::Number(s.len() as f64)),
                        _ => Err("只有数组和字符串可以调用 len()".to_string()),
                    },
                    "push" => {
                        if let Value::Array(arr) = target_val {
                            let arg_val = self.eval_expr(&args[0])?;
                            arr.borrow_mut().push(arg_val);
                            Ok(Value::Void)
                        } else {
                            Err("只有数组可以调用 push()".to_string())
                        }
                    }
                    "pop" => {
                        if let Value::Array(arr) = target_val {
                            let mut arr_ref = arr.borrow_mut();
                            if let Some(val) = arr_ref.pop() {
                                Ok(val)
                            } else {
                                Err("无法从空数组中 pop 元素".to_string())
                            }
                        } else {
                            Err("只有数组可以调用 pop()".to_string())
                        }
                    }
                    "slice" => {
                        let start_val = self.eval_expr(&args[0])?;
                        let end_val = self.eval_expr(&args[1])?;

                        let (start, end) = match (start_val, end_val) {
                            (Value::Number(s), Value::Number(e)) => (s as usize, e as usize),
                            _ => return Err("slice 的参数必须是数字".to_string()),
                        };

                        match target_val {
                            Value::Array(arr) => {
                                let b = arr.borrow();
                                let len = b.len();
                                let s = start.min(len);
                                let e = end.min(len).max(s);
                                Ok(Value::Array(std::rc::Rc::new(std::cell::RefCell::new(
                                    b[s..e].to_vec(),
                                ))))
                            }
                            Value::String(str) => {
                                let chars: Vec<char> = str.chars().collect();
                                let len = chars.len();
                                let s = start.min(len);
                                let e = end.min(len).max(s);
                                Ok(Value::String(chars[s..e].iter().collect()))
                            }
                            _ => Err("只有数组和字符串可以调用 slice()".to_string()),
                        }
                    }
                    "remove" => {
                        let idx_val = self.eval_expr(&args[0])?;
                        let idx = if let Value::Number(n) = idx_val {
                            n as usize
                        } else {
                            return Err("索引必须是数字".to_string());
                        };

                        if let Value::Array(arr) = target_val {
                            let mut b = arr.borrow_mut();
                            if idx < b.len() {
                                Ok(b.remove(idx))
                            } else {
                                Err(format!("移除失败：索引 {} 越界 (长度 {})", idx, b.len()))
                            }
                        } else {
                            Err("只有数组可以调用 remove()".to_string())
                        }
                    }
                    "insert" => {
                        let idx_val = self.eval_expr(&args[0])?;
                        let val = self.eval_expr(&args[1])?;
                        let idx = if let Value::Number(n) = idx_val {
                            n as usize
                        } else {
                            return Err("索引必须是数字".to_string());
                        };

                        if let Value::Array(arr) = target_val {
                            let mut b = arr.borrow_mut();
                            if idx <= b.len() {
                                b.insert(idx, val);
                                Ok(Value::Void)
                            } else {
                                Err(format!("插入失败：索引 {} 越界 (长度 {})", idx, b.len()))
                            }
                        } else {
                            Err("只有数组可以调用 insert()".to_string())
                        }
                    }
                    "clear" => {
                        if let Value::Array(arr) = target_val {
                            arr.borrow_mut().clear();
                            Ok(Value::Void)
                        } else {
                            Err("只有数组可以调用 clear()".to_string())
                        }
                    }
                    _ => Err(format!("未知的方法: {}", method)),
                }
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
                    result_str.push_str(&val.to_string());
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
                index,
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

                if let Some(idx_expr) = index {
                    let idx_val = self.eval_expr(idx_expr).map_err(attach_span)?;
                    let idx = if let Value::Number(n) = idx_val {
                        if n < 0.0 || n.fract() != 0.0 {
                            return Err(("数组索引必须是正整数".into(), stmt.span.clone()));
                        }
                        n as usize
                    } else {
                        return Err(("数组索引必须是数字".into(), stmt.span.clone()));
                    };

                    if let Value::Array(arr) = current_val {
                        let mut arr_ref = arr.borrow_mut();
                        if idx >= arr_ref.len() {
                            return Err((
                                format!("索引越界：长度 {}, 访问 {}", arr_ref.len(), idx),
                                stmt.span.clone(),
                            ));
                        }

                        let elem_val = arr_ref[idx].clone();
                        let new_val = match (elem_val, op.as_str(), right_val) {
                            (_, "=", v) => v,
                            (Value::String(l), "++=", Value::String(r)) => {
                                Value::String(format!("{}{}", l, r))
                            }
                            (Value::Number(l), "+=", Value::Number(r)) => Value::Number(l + r),
                            (Value::Number(l), "-=", Value::Number(r)) => Value::Number(l - r),
                            (Value::Number(l), "*=", Value::Number(r)) => Value::Number(l * r),
                            (Value::Number(l), "/=", Value::Number(r)) => {
                                if r == 0.0 {
                                    return Err(("除数不能为0".to_string(), stmt.span.clone()));
                                }
                                Value::Number(l / r)
                            }
                            _ => {
                                return Err((
                                    "无效的赋值操作或类型不匹配".to_string(),
                                    stmt.span.clone(),
                                ));
                            }
                        };

                        arr_ref[idx] = new_val;
                        return Ok(ControlFlow::None);
                    } else {
                        return Err(("不能对非数组进行索引赋值".into(), stmt.span.clone()));
                    }
                }

                let new_val = match (current_val, op.as_str(), right_val) {
                    (_, "=", v) => v,
                    (Value::String(l), "++=", Value::String(r)) => {
                        Value::String(format!("{}{}", l, r))
                    }
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
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_val = self.eval_expr(condition).map_err(attach_span)?;

                let is_true = match cond_val {
                    Value::Bool(b) => b,
                    _ => return Err(("if 的条件必须是布尔值".to_string(), stmt.span.clone())),
                };

                if is_true {
                    self.env.push_scope();
                    let mut res = ControlFlow::None;
                    for s in then_branch {
                        res = self.eval_stmt(s)?;
                        if matches!(res, ControlFlow::Return(_)) {
                            break;
                        }
                    }
                    self.env.pop_scope();
                    Ok(res)
                } else if let Some(else_b) = else_branch {
                    self.eval_stmt(else_b)
                } else {
                    Ok(ControlFlow::None)
                }
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
                    if !matches!(res, ControlFlow::None) {
                        break;
                    }
                }

                self.env.pop_scope();
                Ok(res)
            }
            Stmt::While { condition, body } => {
                loop {
                    let cond_val = self.eval_expr(condition).map_err(attach_span)?;
                    let is_true = match cond_val {
                        Value::Bool(b) => b,
                        _ => return Err(("while 条件必须是布尔值".to_string(), stmt.span.clone())),
                    };

                    if !is_true {
                        break; // 结束循环
                    }

                    self.env.push_scope();
                    let mut flow = ControlFlow::None;

                    for s in body {
                        flow = self.eval_stmt(s)?;

                        match flow {
                            ControlFlow::Return(_) => break,
                            ControlFlow::Break => break,
                            ControlFlow::Continue => break,
                            ControlFlow::None => {}
                        }
                    }
                    self.env.pop_scope();

                    match flow {
                        ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        ControlFlow::Break => break,
                        ControlFlow::Continue => continue,
                        ControlFlow::None => {}
                    }
                }
                Ok(ControlFlow::None)
            }
            Stmt::For {
                item_name,
                iterable,
                body,
            } => {
                let iter_val = self.eval_expr(iterable).map_err(attach_span)?;

                if let Value::Array(arr) = iter_val {
                    let elements = arr.borrow().clone();

                    for elem in elements {
                        self.env.push_scope();
                        self.env.define(item_name.clone(), elem, false);

                        let mut flow = ControlFlow::None;
                        for s in body {
                            flow = self.eval_stmt(s)?;
                            match flow {
                                ControlFlow::Return(_) => break,
                                ControlFlow::Break => break,
                                ControlFlow::Continue => break,
                                ControlFlow::None => {}
                            }
                        }
                        self.env.pop_scope();

                        match flow {
                            ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                            ControlFlow::Break => break,
                            ControlFlow::Continue => continue,
                            ControlFlow::None => {}
                        }
                    }
                    Ok(ControlFlow::None)
                } else {
                    Err(("for 循环只能遍历数组".to_string(), stmt.span.clone()))
                }
            }
            Stmt::Break => Ok(ControlFlow::Break),
            Stmt::Continue => Ok(ControlFlow::Continue),
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
                ControlFlow::Break => break,
                ControlFlow::Continue => continue,
            }
        }
        Ok(Value::Void)
    }
}
