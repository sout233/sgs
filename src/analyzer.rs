// src/analyzer.rs
use crate::ast::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Number,
    String,
    Void,
    Bool,
    Function { params: Vec<Type>, ret: Box<Type> },
    Unknown,
    Any,
}

impl Type {
    pub fn from_name(s: &str) -> Self {
        match s {
            "number" | "float" => Type::Number,
            "string" => Type::String,
            "void" => Type::Void,
            "bool" => Type::Bool,
            _ => Type::Unknown,
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Number => write!(f, "number"),
            Type::String => write!(f, "string"),
            Type::Void => write!(f, "void"),
            Type::Bool => write!(f, "bool"),
            Type::Unknown => write!(f, "unknown"),
            Type::Any => write!(f, "any"),

            Type::Function { params, ret } => {
                write!(f, "func(")?;

                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }

                write!(f, ") -> {}", ret)
            }
        }
    }
}

pub struct Symbol {
    pub is_mut: bool,
    pub ty: Type,
    pub decl_span: Span,
}

pub struct StaticCheckError {
    pub title: String,
    pub message: String,
    pub span: Span,
    pub note: Option<(String, Span)>,
}

impl StaticCheckError {
    pub fn new(title: impl Into<String>, message: impl Into<String>, span: Span) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            span,
            note: None,
        }
    }

    pub fn with_note(mut self, msg: impl Into<String>, span: Span) -> Self {
        self.note = Some((msg.into(), span));
        self
    }
}

pub struct Analyzer {
    scopes: Vec<HashMap<String, Symbol>>,
    pub errors: Vec<StaticCheckError>,
    current_return_ty: Option<Type>,
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            errors: Vec::new(),
            current_return_ty: None,
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }
    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn define_var(&mut self, name: String, is_mut: bool, ty: Type, span: &Span) {
        let current_scope = self.scopes.last_mut().unwrap();
        current_scope.insert(
            name,
            Symbol {
                is_mut,
                ty,
                decl_span: span.clone(),
            },
        );
    }

    fn resolve_var(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return Some(symbol);
            }
        }
        None
    }

    /// 注册所有全局函数
    pub fn register_functions(&mut self, sys: &SystemDef) {
        self.scopes[0].insert(
            "println".to_string(),
            Symbol {
                is_mut: false,
                ty: Type::Any,
                decl_span: 0..0,
            },
        );

        for func in &sys.functions {
            let mut param_types = Vec::new();
            for p in &func.params {
                param_types.push(Type::from_name(&p.ty));
            }

            let ret_ty = match &func.return_ty {
                Some(ty_str) => Type::from_name(ty_str),
                None => Type::Void,
            };

            let func_ty = Type::Function {
                params: param_types,
                ret: Box::new(ret_ty),
            };

            self.define_var(func.name.clone(), false, func_ty, &(0..0));
        }
    }

    pub fn check_function(&mut self, func: &FunctionDef) {
        self.push_scope();

        self.current_return_ty = Some(match &func.return_ty {
            Some(ty_str) => Type::from_name(ty_str),
            None => Type::Void,
        });

        for param in &func.params {
            self.define_var(
                param.name.clone(),
                false,
                Type::from_name(&param.ty),
                &(0..0),
            );
        }

        for stmt in &func.statements {
            self.check_stmt(stmt);
        }

        self.pop_scope();
    }

    pub fn check_stmt(&mut self, stmt: &Spanned<Stmt>) {
        let span = &stmt.span;

        match &stmt.node {
            Stmt::Let {
                is_mut,
                name,
                value,
            } => {
                let rhs_ty = self.infer_expr(value, span);
                self.define_var(name.clone(), *is_mut, rhs_ty, span);
            }
            Stmt::Assign(AssignStmt {
                target_path,
                op: _,
                value,
            }) => {
                let rhs_ty = self.infer_expr(value, span);
                let name = &target_path[0];

                let var_info = self
                    .resolve_var(name)
                    .map(|sym| (sym.is_mut, sym.ty.clone(), sym.decl_span.clone()));

                match var_info {
                    Some((is_mut, expected_ty, decl_span)) => {
                        if !is_mut {
                            self.errors.push(
                                StaticCheckError::new(
                                    "不可变真的可变吗？",
                                    format!("无法对不可变变量 '{}' 重新赋值。", name),
                                    span.clone(),
                                )
                                .with_note(
                                    format!("Note：变量 '{}' 在这里被声明为不可变，可尝试改为 let mut {} = ...", name, name),
                                    decl_span,
                                ),
                            );
                        }

                        if expected_ty != Type::Unknown
                            && rhs_ty != Type::Unknown
                            && expected_ty != rhs_ty
                        {
                            self.errors.push(StaticCheckError::new(
                                "类型不匹配",
                                format!(
                                    "试图将 '{}' 赋值给 '{}' 类型的变量 '{}'",
                                    rhs_ty, expected_ty, name
                                ),
                                span.clone(),
                            ));
                        }
                    }
                    None => {
                        self.errors.push(StaticCheckError::new(
                            "找不到变量",
                            format!("找不到变量: '{}'", name),
                            span.clone(),
                        ));
                    }
                }
            }
            Stmt::Expr(expr) => {
                self.infer_expr(expr, span);
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_ty = self.infer_expr(condition, span);
                if cond_ty != Type::Unknown && cond_ty != Type::Bool {
                    self.errors.push(StaticCheckError::new(
                        "条件类型错误",
                        format!("'if' 的条件必须是 'bool' 类型，但得到了 '{}'", cond_ty),
                        span.clone(),
                    ));
                }

                self.push_scope();
                for s in then_branch {
                    self.check_stmt(s);
                }
                self.pop_scope();

                if let Some(else_b) = else_branch {
                    self.check_stmt(else_b);
                }
            }
            Stmt::Return(opt_expr) => {
                let actual_ty = match opt_expr {
                    Some(e) => self.infer_expr(e, span),
                    None => Type::Void,
                };
                self.verify_return(actual_ty, span);
            }
            Stmt::ImplicitReturn(expr) => {
                let actual_ty = self.infer_expr(expr, span);
                self.verify_return(actual_ty, span);
            }
            Stmt::Block(stmts) => {
                self.push_scope();
                for s in stmts {
                    self.check_stmt(s);
                }
                self.pop_scope();
            }
        }
    }

    fn verify_return(&mut self, actual_ty: Type, span: &Span) {
        if let Some(expected_ty) = &self.current_return_ty
            && *expected_ty != Type::Unknown
            && actual_ty != Type::Unknown
            && *expected_ty != actual_ty
        {
            self.errors.push(StaticCheckError::new(
                "返回值类型错误",
                format!(
                    "函数声明返回 '{}'，但实际返回了 '{}'",
                    expected_ty, actual_ty
                ),
                span.clone(),
            ));
        }
    }

    pub fn infer_expr(&mut self, expr: &Expr, fallback_span: &Span) -> Type {
        match expr {
            Expr::Number(_) => Type::Number,
            Expr::StringLit(_) => Type::String,
            Expr::StringInterp(parts) => {
                for part in parts {
                    self.infer_expr(part, fallback_span);
                }
                Type::String
            }
            Expr::Path(path) => {
                let name = &path[0];
                match self.resolve_var(name) {
                    Some(sym) => sym.ty.clone(),
                    None => {
                        self.errors.push(StaticCheckError::new(
                            "变量未定义",
                            format!("找不到变量: '{}'", name),
                            fallback_span.clone(),
                        ));
                        Type::Unknown
                    }
                }
            }
            Expr::Bool(_) => Type::Bool,
            Expr::BinaryOp { left, op, right } => {
                            let l_ty = self.infer_expr(left, fallback_span);
                            let r_ty = self.infer_expr(right, fallback_span);

                            if l_ty == Type::Unknown || r_ty == Type::Unknown {
                                return Type::Unknown;
                            }

                            if op == "+" || op == "-" || op == "*" || op == "/" {
                                if l_ty != Type::Number || r_ty != Type::Number {
                                    self.errors.push(StaticCheckError::new(
                                        "类型错误",
                                        format!("操作符 '{}' 只能用于两个数字，但得到了 '{}' 和 '{}'", op, l_ty, r_ty),
                                        fallback_span.clone(),
                                    ));
                                    return Type::Unknown;
                                }
                                return Type::Number;
                            }
                            else if op == "==" || op == "!=" {
                                if l_ty != r_ty {
                                    self.errors.push(StaticCheckError::new(
                                        "类型不匹配",
                                        format!("无法比较 '{}' 和 '{}'", l_ty, r_ty),
                                        fallback_span.clone(),
                                    ));
                                    return Type::Unknown;
                                }
                                return Type::Bool;
                            }
                            else if op == "<" || op == ">" || op == "<=" || op == ">=" {
                                if l_ty != Type::Number || r_ty != Type::Number {
                                    self.errors.push(StaticCheckError::new(
                                        "类型错误",
                                        format!("操作符 '{}' 只能用于两个数字，但得到了 '{}' 和 '{}'", op, l_ty, r_ty),
                                        fallback_span.clone(),
                                    ));
                                    return Type::Unknown;
                                }
                                return Type::Bool;
                            }
                            Type::Unknown
                        }
            Expr::Call { target, args } => {
                let target_ty = self.infer_expr(target, fallback_span);

                if target_ty == Type::Any {
                    // 内置函数 println 等
                    for arg in args {
                        self.infer_expr(arg, fallback_span);
                    }
                    return Type::Void;
                }

                if let Type::Function { params, ret } = target_ty {
                    if args.len() != params.len() {
                        self.errors.push(StaticCheckError::new(
                            "参数数量错误",
                            format!(
                                "函数需要 {} 个参数，但传入了 {} 个",
                                params.len(),
                                args.len()
                            ),
                            fallback_span.clone(),
                        ));
                    } else {
                        // 检查每个参数的类型
                        for (i, arg) in args.iter().enumerate() {
                            let arg_ty = self.infer_expr(arg, fallback_span);
                            if arg_ty != Type::Unknown
                                && params[i] != Type::Unknown
                                && arg_ty != params[i]
                            {
                                self.errors.push(StaticCheckError::new(
                                    "参数类型错误",
                                    format!(
                                        "第 {} 个参数期待 '{}'，但传入了 '{}'",
                                        i + 1,
                                        params[i],
                                        arg_ty
                                    ),
                                    fallback_span.clone(),
                                ));
                            }
                        }
                    }
                    return *ret;
                } else if target_ty != Type::Unknown {
                    self.errors.push(StaticCheckError::new(
                        "调用错误",
                        format!("试图将 '{}' 类型的变量当作函数调用", target_ty),
                        fallback_span.clone(),
                    ));
                }
                Type::Unknown
            }
            _ => Type::Unknown, // TODO: 闭包等复杂的
        }
    }
}
