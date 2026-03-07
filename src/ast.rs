// src/ast.rs
//! # AST Definition
//! 抽象至极的语法树，为ECS优化

/// 主要的节点
#[derive(Debug, PartialEq, Clone)]
pub enum SgsNode {
    EntityDef(EntityDef),
    ComponentDef(ComponentDef),
    SystemDef(SystemDef),
}

/// Entity 定义
#[derive(Debug, PartialEq, Clone)]
pub struct EntityDef {
    pub name: String,
    pub components: Vec<String>,
}

/// Component 定义
#[derive(Debug, PartialEq, Clone)]
pub struct ComponentDef {
    pub name: String,
    pub params: Vec<Param>,
}

/// System 定义
#[derive(Debug, PartialEq, Clone)]
pub struct SystemDef {
    pub name: String,
    pub required_components: Vec<RequiredComponent>,
    pub functions: Vec<FunctionDef>,
}

/// Param 定义
#[derive(Debug, PartialEq, Clone)]
pub struct Param {
    pub name: String,
    /// 其实这是type
    pub ty: String,
}

/// 用于System里的require关键字
#[derive(Debug, PartialEq, Clone)]
pub struct RequiredComponent {
    /// Component是否可变
    pub is_mut: bool,
    /// Component的名称
    pub name: String,
}

/// 函数定义
#[derive(Debug, PartialEq, Clone)]
pub struct FunctionDef {
    pub name: String,
    pub params: Vec<FnParam>,
    /// 返回值的类型 (-> ???)
    pub return_ty: Option<String>,
    /// 函数体里的东西
    pub statements: Vec<Stmt>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct FnParam {
    pub name: String,
    /// 其实是type
    pub ty: String,
}

/// 语句
#[derive(Debug, PartialEq, Clone)]
pub enum Stmt {
    /// 声明语句
    Let {
        is_mut: bool,
        name: String,
        value: Expr,
    },
    /// 赋值语句
    Assign(AssignStmt),
    /// 表达式语句
    Expr(Expr),
}

/// 赋值语句
#[derive(Debug, PartialEq, Clone)]
pub struct AssignStmt {
    /// 左值的路径
    pub target_path: Vec<String>,
    /// 运算符
    pub op: String,
    /// 右值的表达式
    pub value: Expr,
}

/// 表达式
#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    Number(f64),
    StringLit(String),
    /// 路径访问（点）表达式
    Path(Vec<String>),
    /// 闭包
    Closure {
        params: Vec<FnParam>,
        body: Vec<Stmt>,
    },
    /// 函数调用
    Call {
        target: Box<Expr>,
        args: Vec<Expr>,
    },
    /// 二元运算符表达式
    BinaryOp {
        left: Box<Expr>,
        op: String,
        right: Box<Expr>,
    },
}
