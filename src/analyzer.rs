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
    Array(Box<Type>),
    Struct(String),
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
            custom_type => Type::Struct(custom_type.to_string()),
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
            Type::Array(inner) => write!(f, "{}[]", inner),
            Type::Unknown => write!(f, "unknown"),
            Type::Any => write!(f, "any"),
            Type::Struct(name) => write!(f, "{}", name),
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

pub struct StaticCheckWarning {
    pub title: String,
    pub message: String,
    pub span: Span,
}

impl StaticCheckWarning {
    pub fn new(title: impl Into<String>, message: impl Into<String>, span: Span) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            span,
        }
    }
}

pub struct Analyzer {
    scopes: Vec<HashMap<String, Symbol>>,
    pub errors: Vec<StaticCheckError>,
    pub warnings: Vec<StaticCheckWarning>,
    current_return_ty: Option<Type>,
    loop_depth: usize,
    iterating_vars: Vec<String>,
    pub struct_defs: HashMap<String, HashMap<String, Type>>,
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
            warnings: Vec::new(),
            current_return_ty: None,
            loop_depth: 0,
            iterating_vars: Vec::new(),
            struct_defs: HashMap::new(),
        }
    }

    pub fn register_struct(&mut self, struct_def: &StructDef) {
        let mut fields = HashMap::new();
        for (f_name, f_ty_str) in &struct_def.fields {
            fields.insert(f_name.clone(), Type::from_name(f_ty_str));
        }
        self.struct_defs.insert(struct_def.name.clone(), fields);
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

        self.scopes[0].insert(
            "print".to_string(),
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
                index,
            }) => {
                let rhs_ty = self.infer_expr(value, span);
                let name = &target_path[0];

                if self.iterating_vars.contains(name) {
                    self.warnings.push(StaticCheckWarning::new(
                        "遍历中修改集合",
                        format!("
                            你正在对 '{}' 进行 for 循环遍历，同时又在这里修改了它。
                            SGS 使用快照机制，这不会导致死循环，但新增/删除的元素不会在当前循环中生效，请注意逻辑预期", name),
                        span.clone(),
                    ));
                }

                let var_info = self
                    .resolve_var(name)
                    .map(|sym| (sym.is_mut, sym.ty.clone(), sym.decl_span.clone()));

                match var_info {
                    Some((is_mut, mut expected_ty, decl_span)) => {
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

                        for field in target_path.iter().skip(1) {
                            if let Type::Struct(struct_name) = &expected_ty {
                                if let Some(fields_map) = self.struct_defs.get(struct_name) {
                                    if let Some(field_ty) = fields_map.get(field) {
                                        expected_ty = field_ty.clone(); // 成功进入下一层
                                    } else {
                                        expected_ty = Type::Unknown;
                                        break;
                                    }
                                }
                            } else {
                                expected_ty = Type::Unknown;
                                break;
                            }
                        }

                        let actual_target_ty = if let Some(idx_expr) = index {
                            let idx_ty = self.infer_expr(idx_expr, span);
                            if idx_ty != Type::Unknown && idx_ty != Type::Number {
                                self.errors.push(StaticCheckError::new(
                                    "索引错误",
                                    "数组索引必须是 'number' 类型",
                                    span.clone(),
                                ));
                            }

                            if let Type::Array(inner) = &expected_ty {
                                *inner.clone()
                            } else {
                                if expected_ty != Type::Unknown {
                                    self.errors.push(StaticCheckError::new(
                                        "类型错误",
                                        format!("无法对非数组类型 '{}' 进行索引赋值", expected_ty),
                                        span.clone(),
                                    ));
                                }
                                Type::Unknown
                            }
                        } else {
                            expected_ty.clone()
                        };

                        if actual_target_ty != Type::Unknown
                            && rhs_ty != Type::Unknown
                            && actual_target_ty != rhs_ty
                        {
                            self.errors.push(StaticCheckError::new(
                                "类型不匹配",
                                format!(
                                    "试图将 '{}' 赋值给 '{}' 类型的坑位",
                                    rhs_ty, actual_target_ty
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
            Stmt::While { condition, body } => {
                let cond_ty = self.infer_expr(condition, span);
                if cond_ty != Type::Unknown && cond_ty != Type::Bool {
                    self.errors.push(StaticCheckError::new(
                        "条件类型错误",
                        format!("'while' 的条件必须是 'bool' 类型，但得到了 '{}'", cond_ty),
                        span.clone(),
                    ));
                }

                self.loop_depth += 1;
                self.push_scope();
                for s in body {
                    self.check_stmt(s);
                }
                self.pop_scope();
                self.loop_depth -= 1; // 离开循环
            }
            Stmt::For {
                item_name,
                iterable,
                body,
            } => {
                let iter_ty = self.infer_expr(iterable, span);

                let item_ty = if let Type::Array(inner) = iter_ty {
                    *inner
                } else if iter_ty != Type::Unknown {
                    self.errors.push(StaticCheckError::new(
                        "类型错误",
                        format!("'for' 循环只能遍历数组，但得到了 '{}'", iter_ty),
                        span.clone(),
                    ));
                    Type::Unknown
                } else {
                    Type::Unknown
                };

                self.loop_depth += 1;
                self.push_scope();

                self.define_var(item_name.clone(), false, item_ty, span);

                let mut locked_var = None;
                if let Expr::Path(path) = iterable {
                    if path.len() == 1 {
                        let name = path[0].clone();
                        self.iterating_vars.push(name.clone());
                        locked_var = Some(name);
                    }
                }

                for s in body {
                    self.check_stmt(s);
                }

                self.pop_scope();
                self.loop_depth -= 1;

                if locked_var.is_some() {
                    self.iterating_vars.pop();
                }
            }
            Stmt::Break => {
                if self.loop_depth == 0 {
                    self.errors.push(StaticCheckError::new(
                        "非法控制流",
                        "'break' 只能在循环体内使用",
                        span.clone(),
                    ));
                }
            }
            Stmt::Continue => {
                if self.loop_depth == 0 {
                    self.errors.push(StaticCheckError::new(
                        "非法控制流",
                        "'continue' 只能在循环体内使用",
                        span.clone(),
                    ));
                }
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
                let root_name = &path[0];
                let mut current_ty = match self.resolve_var(root_name) {
                    Some(sym) => sym.ty.clone(),
                    None => {
                        self.errors.push(StaticCheckError::new(
                            "变量未定义",
                            format!("找不到变量: '{}'", root_name),
                            fallback_span.clone(),
                        ));
                        return Type::Unknown;
                    }
                };

                for field in path.iter().skip(1) {
                    if let Type::Struct(struct_name) = &current_ty {
                        if let Some(fields_map) = self.struct_defs.get(struct_name) {
                            if let Some(field_ty) = fields_map.get(field) {
                                current_ty = field_ty.clone();
                            } else {
                                self.errors.push(StaticCheckError::new(
                                    "字段不存在",
                                    format!("结构体 '{}' 没有 '{}' 字段", struct_name, field),
                                    fallback_span.clone(),
                                ));
                                return Type::Unknown;
                            }
                        } else {
                            return Type::Unknown;
                        }
                    } else {
                        self.errors.push(StaticCheckError::new(
                            "属性读取错误",
                            format!(
                                "无法读取 '{}' 上的属性 '.{}'，因为它不是结构体",
                                current_ty, field
                            ),
                            fallback_span.clone(),
                        ));
                        return Type::Unknown;
                    }
                }
                current_ty
            }
            Expr::Bool(_) => Type::Bool,
            Expr::Array(elements) => {
                if elements.is_empty() {
                    return Type::Array(Box::new(Type::Any));
                }

                let first_ty = self.infer_expr(&elements[0], fallback_span);

                for item in elements.iter().skip(1) {
                    let item_ty = self.infer_expr(item, fallback_span);
                    if item_ty != Type::Unknown && item_ty != first_ty {
                        self.errors.push(StaticCheckError::new(
                            "数组类型不一致",
                            format!(
                                "数组元素类型必须统一，期待 '{}'，但得到了 '{}'",
                                first_ty, item_ty
                            ),
                            fallback_span.clone(),
                        ));
                    }
                }
                Type::Array(Box::new(first_ty))
            }
            Expr::Index { target, index } => {
                let target_ty = self.infer_expr(target, fallback_span);
                let index_ty = self.infer_expr(index, fallback_span);

                if index_ty != Type::Unknown && index_ty != Type::Number {
                    self.errors.push(StaticCheckError::new(
                        "索引错误",
                        "数组的索引必须是 'number' 类型",
                        fallback_span.clone(),
                    ));
                }

                if let Type::Array(inner) = target_ty {
                    *inner
                } else if target_ty != Type::Unknown {
                    self.errors.push(StaticCheckError::new(
                        "类型错误",
                        format!("只能对数组类型进行索引访问，但得到了 '{}'", target_ty),
                        fallback_span.clone(),
                    ));
                    Type::Unknown
                } else {
                    Type::Unknown
                }
            }
            Expr::BinaryOp { left, op, right } => {
                let l_ty = self.infer_expr(left, fallback_span);
                let r_ty = self.infer_expr(right, fallback_span);

                if l_ty == Type::Unknown || r_ty == Type::Unknown {
                    return Type::Unknown;
                }

                if op == "++" {
                    if l_ty != Type::String || r_ty != Type::String {
                        self.errors.push(StaticCheckError::new(
                            "类型错误",
                            format!(
                                "'++' 只能用于连接两个字符串，但这里是 '{}' 和 '{}'",
                                l_ty, r_ty
                            ),
                            fallback_span.clone(),
                        ));
                        return Type::Unknown;
                    }
                    return Type::String;
                } else if op == "+" || op == "-" || op == "*" || op == "/" {
                    if op == "+" && (l_ty == Type::String || r_ty == Type::String) {
                        self.errors.push(
                            StaticCheckError::new(
                                "操作符错误",
                                format!("不能使用 '+' 来操作字符串 ('{}' + '{}')", l_ty, r_ty),
                                fallback_span.clone(),
                            )
                            .with_note(
                                "SGS 使用 '++' 来进行字符串拼接，请尝试将 '+' 替换为 '++'",
                                fallback_span.clone(),
                            ),
                        );
                        return Type::Unknown;
                    }

                    // 纯数字校验
                    if l_ty != Type::Number || r_ty != Type::Number {
                        self.errors.push(StaticCheckError::new(
                            "类型错误",
                            format!(
                                "操作符 '{}' 只能用于两个数字，但这里是 '{}' 和 '{}'",
                                op, l_ty, r_ty
                            ),
                            fallback_span.clone(),
                        ));
                        return Type::Unknown;
                    }
                    return Type::Number;
                } else if op == "==" || op == "!=" {
                    if l_ty != r_ty {
                        self.errors.push(StaticCheckError::new(
                            "类型不匹配",
                            format!("无法比较 '{}' 和 '{}'", l_ty, r_ty),
                            fallback_span.clone(),
                        ));
                        return Type::Unknown;
                    }
                    return Type::Bool;
                } else if op == "<" || op == ">" || op == "<=" || op == ">=" {
                    if l_ty != Type::Number || r_ty != Type::Number {
                        self.errors.push(StaticCheckError::new(
                            "类型错误",
                            format!(
                                "操作符 '{}' 只能用于两个数字，但这里写的是 '{}' 和 '{}'",
                                op, l_ty, r_ty
                            ),
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
                        // 检查参数类型
                        for (i, arg) in args.iter().enumerate() {
                            let arg_ty = self.infer_expr(arg, fallback_span);
                            if arg_ty != Type::Unknown
                                && params[i] != Type::Unknown
                                && arg_ty != params[i]
                            {
                                self.errors.push(StaticCheckError::new(
                                    "参数类型错误",
                                    format!(
                                        "第 {} 个参数应该是 '{}'，但传入了 '{}'",
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
                        format!("为啥要把 '{}' 类型的变量当作函数调用", target_ty),
                        fallback_span.clone(),
                    ));
                }
                Type::Unknown
            }
            Expr::MethodCall {
                target,
                method,
                args,
            } => {
                let target_ty = self.infer_expr(target, fallback_span);
                if target_ty == Type::Unknown {
                    return Type::Unknown;
                }

                match method.as_str() {
                    "len" => {
                        if args.len() != 0 {
                            self.errors.push(StaticCheckError::new(
                                "参数错误",
                                "len() 不需要参数",
                                fallback_span.clone(),
                            ));
                        }
                        match target_ty {
                            Type::Array(_) | Type::String => Type::Number,
                            _ => {
                                self.errors.push(StaticCheckError::new(
                                    "类型错误",
                                    format!("类型 '{}' 没有 len() 方法", target_ty),
                                    fallback_span.clone(),
                                ));
                                Type::Unknown
                            }
                        }
                    }
                    "push" => {
                        if args.len() != 1 {
                            self.errors.push(StaticCheckError::new(
                                "参数错误",
                                "push() 需要 1 个参数",
                                fallback_span.clone(),
                            ));
                            return Type::Unknown;
                        }

                        if let Expr::Path(path) = &**target {
                            let name = &path[0];
                            if self.iterating_vars.contains(name) {
                                self.warnings.push(StaticCheckWarning::new(
                                    "遍历中追加元素",
                                    format!("在 for 循环内部调用 '{}.push()'。注意：这只会修改原数组，新元素不会参与本次遍历。", name),
                                    fallback_span.clone(),
                                ));
                            }
                        }

                        if let Type::Array(inner_ty) = &target_ty {
                            let arg_ty = self.infer_expr(&args[0], fallback_span);
                            if arg_ty != Type::Unknown
                                && **inner_ty != Type::Unknown
                                && arg_ty != **inner_ty
                            {
                                self.errors.push(StaticCheckError::new(
                                    "类型不匹配",
                                    format!("无法将 '{}' push 到 '{}' 的数组中", arg_ty, inner_ty),
                                    fallback_span.clone(),
                                ));
                            }
                            Type::Void
                        } else {
                            self.errors.push(StaticCheckError::new(
                                "类型错误",
                                format!("只有数组有 push() 方法，但得到了 '{}'", target_ty),
                                fallback_span.clone(),
                            ));
                            Type::Unknown
                        }
                    }
                    "pop" => {
                        if args.len() != 0 {
                            self.errors.push(StaticCheckError::new(
                                "参数错误",
                                "pop() 不需要参数",
                                fallback_span.clone(),
                            ));
                            return Type::Unknown;
                        }

                        if let Expr::Path(path) = &**target {
                            let name = &path[0];
                            if self.iterating_vars.contains(name) {
                                self.warnings.push(StaticCheckWarning::new(
                                    "遍历中弹出元素",
                                    format!("在 for 循环内部调用 '{}.pop()'。注意：循环仍会按照进入前的快照长度继续执行。", name),
                                    fallback_span.clone(),
                                ));
                            }
                        }

                        if let Type::Array(inner_ty) = target_ty {
                            *inner_ty
                        } else {
                            self.errors.push(StaticCheckError::new(
                                "类型错误",
                                format!("只有数组有 pop() 方法，但得到了 '{}'", target_ty),
                                fallback_span.clone(),
                            ));
                            Type::Unknown
                        }
                    }
                    "slice" => {
                        if args.len() != 2 {
                            self.errors.push(StaticCheckError::new(
                                "参数错误",
                                "slice() 需要 2 个参数 (start, end)",
                                fallback_span.clone(),
                            ));
                            return Type::Unknown;
                        }
                        let start_ty = self.infer_expr(&args[0], fallback_span);
                        let end_ty = self.infer_expr(&args[1], fallback_span);

                        if (start_ty != Type::Unknown && start_ty != Type::Number)
                            || (end_ty != Type::Unknown && end_ty != Type::Number)
                        {
                            self.errors.push(StaticCheckError::new(
                                "参数类型错误",
                                "slice() 的参数必须是数字",
                                fallback_span.clone(),
                            ));
                        }

                        match &target_ty {
                            Type::Array(_) | Type::String => target_ty.clone(), // 返回同样的类型
                            _ => {
                                self.errors.push(StaticCheckError::new(
                                    "类型错误",
                                    format!("类型 '{}' 没有 slice() 方法", target_ty),
                                    fallback_span.clone(),
                                ));
                                Type::Unknown
                            }
                        }
                    }
                    "remove" => {
                        if args.len() != 1 {
                            self.errors.push(StaticCheckError::new(
                                "参数错误",
                                "remove() 需要 1 个参数 (index)",
                                fallback_span.clone(),
                            ));
                            return Type::Unknown;
                        }

                        if let Expr::Path(path) = &**target {
                            if self.iterating_vars.contains(&path[0]) {
                                self.warnings.push(StaticCheckWarning::new("遍历中移除元素", format!("在 for 循环内部调用 '{}.remove()'。注意：循环长度已锁定，这可能导致逻辑偏移。", path[0]), fallback_span.clone()));
                            }
                        }

                        let idx_ty = self.infer_expr(&args[0], fallback_span);
                        if idx_ty != Type::Unknown && idx_ty != Type::Number {
                            self.errors.push(StaticCheckError::new(
                                "参数类型错误",
                                "remove() 的索引必须是数字",
                                fallback_span.clone(),
                            ));
                        }

                        if let Type::Array(inner_ty) = target_ty {
                            *inner_ty // 返回被移除的元素类型
                        } else {
                            self.errors.push(StaticCheckError::new(
                                "类型错误",
                                format!("只有数组有 remove() 方法，但得到了 '{}'", target_ty),
                                fallback_span.clone(),
                            ));
                            Type::Unknown
                        }
                    }
                    "insert" => {
                        if args.len() != 2 {
                            self.errors.push(StaticCheckError::new(
                                "参数错误",
                                "insert() 需要 2 个参数 (index, value)",
                                fallback_span.clone(),
                            ));
                            return Type::Unknown;
                        }

                        if let Expr::Path(path) = &**target {
                            if self.iterating_vars.contains(&path[0]) {
                                self.warnings.push(StaticCheckWarning::new(
                                    "遍历中插入元素",
                                    format!("在 for 循环内部调用 '{}.insert()'。", path[0]),
                                    fallback_span.clone(),
                                ));
                            }
                        }

                        let idx_ty = self.infer_expr(&args[0], fallback_span);
                        if idx_ty != Type::Unknown && idx_ty != Type::Number {
                            self.errors.push(StaticCheckError::new(
                                "参数类型错误",
                                "insert() 的索引必须是数字",
                                fallback_span.clone(),
                            ));
                        }

                        if let Type::Array(inner_ty) = &target_ty {
                            let arg_ty = self.infer_expr(&args[1], fallback_span);
                            if arg_ty != Type::Unknown
                                && **inner_ty != Type::Unknown
                                && arg_ty != **inner_ty
                            {
                                self.errors.push(StaticCheckError::new(
                                    "类型不匹配",
                                    format!("无法将 '{}' 插入到 '{}' 的数组中", arg_ty, inner_ty),
                                    fallback_span.clone(),
                                ));
                            }
                            Type::Void
                        } else {
                            self.errors.push(StaticCheckError::new(
                                "类型错误",
                                format!("只有数组有 insert() 方法，得到了 '{}'", target_ty),
                                fallback_span.clone(),
                            ));
                            Type::Unknown
                        }
                    }
                    "clear" => {
                        if args.len() != 0 {
                            self.errors.push(StaticCheckError::new(
                                "参数错误",
                                "clear() 不需要参数",
                                fallback_span.clone(),
                            ));
                        }

                        if let Expr::Path(path) = &**target {
                            if self.iterating_vars.contains(&path[0]) {
                                self.warnings.push(StaticCheckWarning::new("遍历中清空数组", format!("在 for 循环内部调用 '{}.clear()'。当前的遍历会继续基于旧的快照执行完。", path[0]), fallback_span.clone()));
                            }
                        }

                        if let Type::Array(_) = target_ty {
                            Type::Void
                        } else {
                            self.errors.push(StaticCheckError::new(
                                "类型错误",
                                format!("只有数组有 clear() 方法，得到了 '{}'", target_ty),
                                fallback_span.clone(),
                            ));
                            Type::Unknown
                        }
                    }
                    _ => {
                        self.errors.push(StaticCheckError::new(
                            "方法不存在",
                            format!("找不到方法: '{}'", method),
                            fallback_span.clone(),
                        ));
                        Type::Unknown
                    }
                }
            }
            Expr::StructInit { name, fields } => {
                let struct_blueprint = match self.struct_defs.get(name) {
                    Some(bp) => bp.clone(),
                    None => {
                        self.errors.push(StaticCheckError::new(
                            "未知类型",
                            format!("找不到名为 '{}' 的结构体", name),
                            fallback_span.clone(),
                        ));
                        return Type::Unknown;
                    }
                };

                let mut provided_fields = HashMap::new();

                for (f_name, f_expr) in fields {
                    let expr_ty = self.infer_expr(f_expr, fallback_span);
                    provided_fields.insert(f_name.clone(), true);

                    if let Some(expected_ty) = struct_blueprint.get(f_name) {
                        if expr_ty != Type::Unknown
                            && *expected_ty != Type::Unknown
                            && expr_ty != *expected_ty
                        {
                            self.errors.push(StaticCheckError::new(
                                "字段类型不匹配",
                                format!(
                                    "实例化 '{}' 时，字段 '{}' 期待 '{}'，但得到了 '{}'",
                                    name, f_name, expected_ty, expr_ty
                                ),
                                fallback_span.clone(),
                            ));
                        }
                    } else {
                        self.errors.push(StaticCheckError::new(
                            "不存在的字段",
                            format!("结构体 '{}' 根本没有名为 '{}' 的字段", name, f_name),
                            fallback_span.clone(),
                        ));
                    }
                }

                for (req_field, _) in struct_blueprint {
                    if !provided_fields.contains_key(&req_field) {
                        self.errors.push(StaticCheckError::new(
                            "漏写字段",
                            format!("实例化 '{}' 时，缺少必要的字段: '{}'", name, req_field),
                            fallback_span.clone(),
                        ));
                    }
                }

                Type::Struct(name.clone())
            }
            _ => Type::Unknown, // TODO: 闭包等复杂的
        }
    }
}
