// src/ast.rs
//! # AST Definition
//! 抽象至极的语法树，为ECS优化

pub type Span = std::ops::Range<usize>;

#[derive(Debug, PartialEq, Clone)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

/// 主要的节点
#[derive(Debug, PartialEq, Clone)]
pub enum SgsNode {
    EntityDef(EntityDef),
    ComponentDef(ComponentDef),
    SystemDef(SystemDef),
    StructDef(StructDef),
    ExternFunctionDef(ExternFunctionDef),
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
    pub statements: Vec<Spanned<Stmt>>,
}

///
#[derive(Debug, Clone, PartialEq)]
pub struct ExternFunctionDef {
    pub name: String,
    pub params: Vec<FnParam>,
    pub return_ty: Option<String>,
}

/// 结构体定义
#[derive(Debug, Clone, PartialEq)]
pub struct StructDef {
    /// 结构体的名字
    pub name: String,
    /// 应该是字段吧，前面是字段名，后面是类型
    pub fields: Vec<(String, String)>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct FnParam {
    pub is_mut: bool,
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
    /// 有分号的返回语句
    Return(Option<Expr>),
    /// 没分号的返回语句
    ImplicitReturn(Expr),
    /// 块语句
    Block(Vec<Spanned<Stmt>>),
    /// if 语句罢了
    If {
        condition: Expr,
        then_branch: Vec<Spanned<Stmt>>,
        /// 里面可能是 else if 或者 else什么的
        else_branch: Option<Box<Spanned<Stmt>>>,
    },
    /// while 语句
    While {
        condition: Expr,
        body: Vec<Spanned<Stmt>>,
    },
    /// break 关键字
    Break,
    /// continue 关键字
    Continue,
    /// for 循环
    For {
        item_name: String,
        iterable: Expr,
        body: Vec<Spanned<Stmt>>,
    },
}

/// 赋值语句
#[derive(Debug, PartialEq, Clone)]
pub struct AssignStmt {
    /// 左值的路径
    pub target_path: Vec<String>,
    /// 可选的索引下标
    pub index: Option<Expr>,
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
    /// 内插字符串
    StringInterp(Vec<Expr>),
    /// 路径访问（点）表达式
    Path(Vec<String>),
    /// 闭包
    Closure {
        params: Vec<FnParam>,
        body: Vec<Spanned<Stmt>>,
    },
    /// 函数调用
    Call {
        target: Box<Expr>,
        args: Vec<Expr>,
    },
    /// 方法调用
    MethodCall {
        /// 点号左边的主体（恩情！）
        target: Box<Expr>,
        /// 点号右边的方法名
        method: String,
        /// 括号里的参数
        args: Vec<Expr>,
    },
    /// 二元运算符表达式
    BinaryOp {
        left: Box<Expr>,
        op: String,
        right: Box<Expr>,
    },
    /// bool罢了
    Bool(bool),
    /// 数组字面量
    Array(Vec<Expr>),
    /// 索引读取
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
    },
    /// 结构体声明
    StructInit {
        name: String,
        fields: Vec<(String, Expr)>,
    },
    /// 类型转换表达式
    /// 比如 114 as string 这种
    Cast {
        expr: Box<Expr>,
        ty_name: String,
    },
}
