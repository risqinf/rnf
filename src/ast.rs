// ─────────────────────────────────────────────────────────────────────────────
//  RNF AST  –  Abstract Syntax Tree definitions
// ─────────────────────────────────────────────────────────────────────────────

// ── Types ─────────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum Type {
    Int,
    Float,
    Str,
    Bool,
    Void,
    Ptr(Box<Type>),
    Chan(Box<Type>),
    Array(Box<Type>, Option<usize>),
    Custom(String),
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int        => write!(f, "int"),
            Type::Float      => write!(f, "float"),
            Type::Str        => write!(f, "str"),
            Type::Bool       => write!(f, "bool"),
            Type::Void       => write!(f, "void"),
            Type::Ptr(t)     => write!(f, "ptr<{}>", t),
            Type::Chan(t)    => write!(f, "chan<{}>", t),
            Type::Array(t,n) => match n {
                Some(sz) => write!(f, "[{}; {}]", t, sz),
                None     => write!(f, "[{}]", t),
            },
            Type::Custom(n)  => write!(f, "{}", n),
        }
    }
}

// ── Parameter ─────────────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty:   Type,
}

// ── Binary / Unary Operators ──────────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Ne, Lt, Gt, Le, Ge,
    And, Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnOp {
    Neg,
    Not,
}

// ── Expressions ───────────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Expr {
    /// 42
    Int(i64),
    /// 3.14
    Float(f64),
    /// "hello"
    Str(String),
    /// true / false
    Bool(bool),
    /// nil
    Nil,
    /// variable_name
    Ident(String),
    /// left op right
    Binary { op: BinOp, left: Box<Expr>, right: Box<Expr> },
    /// op expr
    Unary { op: UnOp, expr: Box<Expr> },
    /// func(args...)
    Call { name: String, args: Vec<Expr> },
    /// arr[idx]
    Index { array: Box<Expr>, index: Box<Expr> },
    /// obj.field
    Field { object: Box<Expr>, field: String },
    /// &expr  (address-of)
    Ref(Box<Expr>),
    /// *expr  (dereference)
    Deref(Box<Expr>),
    /// raw_mem(0xDEAD)
    RawMem(Box<Expr>),
    /// make_chan()
    MakeChan(Type),
    /// <-ch  (receive from channel)
    ChanRecv(Box<Expr>),
    /// Struct { field: val, … }
    StructInit { name: String, fields: Vec<(String, Expr)> },
    /// start..end
    Range { start: Box<Expr>, end: Box<Expr> },
    /// [a, b, c]
    Array(Vec<Expr>),
    /// (expr as Type)
    Cast { expr: Box<Expr>, to: Type },
}

// ── Statements ────────────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub enum Stmt {
    /// Standalone expression (function call, assignment, etc.)
    Expr(Expr),
    /// let name[: type] = value   OR   name = value  (shell-like)
    Let { name: String, ty: Option<Type>, value: Expr },
    /// target = value
    Assign { target: Expr, value: Expr },
    /// if cond { … } else { … }
    If { cond: Expr, then_body: Vec<Stmt>, else_body: Option<Vec<Stmt>> },
    /// while cond { … }
    While { cond: Expr, body: Vec<Stmt> },
    /// loop var in range { … }
    Loop { var: String, range: Expr, body: Vec<Stmt> },
    /// return [expr]
    Return(Option<Expr>),
    /// exec "system command"
    Exec(Expr),
    /// print(args…)
    Print(Vec<Expr>),
    /// go fn_call
    Go(Expr),
    /// send(ch, val)
    Send { chan: Expr, value: Expr },
    /// asm { "raw asm" }
    Asm(String),
    /// ptr name = &expr
    PtrDecl { name: String, expr: Expr },
    /// break
    Break,
    /// continue
    Continue,
}

// ── Top-level declarations ────────────────────────────────────────────────────
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FunctionDecl {
    pub name:     String,
    pub params:   Vec<Param>,
    pub ret_type: Option<Type>,
    pub body:     Vec<Stmt>,
    pub is_pub:   bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StructDecl {
    pub name:   String,
    pub fields: Vec<(String, Type)>,
    pub is_pub: bool,
}

#[derive(Debug, Clone)]
pub struct ImplDecl {
    pub target:  String,
    pub methods: Vec<FunctionDecl>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum TopLevel {
    Function(FunctionDecl),
    Struct(StructDecl),
    Impl(ImplDecl),
    Use(String),
    GlobalVar { name: String, ty: Option<Type>, value: Expr },
}

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<TopLevel>,
}
