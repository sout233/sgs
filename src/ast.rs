// src/ast.rs
#[derive(Debug, PartialEq)]
pub enum SgsNode {
    EntityDef(EntityDef),
    ComponentDef(ComponentDef),
    SystemDef(SystemDef),
}

#[derive(Debug, PartialEq)]
pub struct EntityDef {
    pub name: String,
    pub components: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct ComponentDef {
    pub name: String,
    pub params: Vec<Param>,
}

#[derive(Debug, PartialEq)]
pub struct SystemDef {
    pub name: String,
    pub required_components: Vec<RequiredComponent>,
    pub functions: Vec<FunctionDef>,
}

#[derive(Debug, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: String,
}

#[derive(Debug, PartialEq)]
pub struct RequiredComponent {
    pub is_mut: bool,
    pub name: String,
}

#[derive(Debug, PartialEq)]
pub struct FunctionDef {
    pub name: String,
    pub params: Vec<FnParam>,
    pub return_ty: Option<String>,
    pub statements: Vec<Stmt>,
}

#[derive(Debug, PartialEq)]
pub struct FnParam {
    pub name: String,
    pub ty: String,
}

#[derive(Debug, PartialEq)]
pub enum Stmt {
    Let { name: String, value: Expr },
    Assign(AssignStmt),
    Expr(Expr),
}

#[derive(Debug, PartialEq)]
pub struct AssignStmt {
    pub target_path: Vec<String>,
    pub op: String,
    pub value: Expr,
}

#[derive(Debug, PartialEq)]
pub enum Expr {
    Number(f64),
    StringLit(String),
    Path(Vec<String>),
    Closure {
        params: Vec<FnParam>,
        body: Vec<Stmt>,
    },
    Call {
        target: Box<Expr>,
        args: Vec<Expr>,
    },
}
