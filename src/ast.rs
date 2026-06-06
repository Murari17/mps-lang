#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Float32,
    String,
    Bool,
    Void,
    PyObject,
    Custom(String),
    Optional(Box<Type>),
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::Float32 => write!(f, "float32"),
            Type::String => write!(f, "string"),
            Type::Bool => write!(f, "bool"),
            Type::Void => write!(f, "void"),
            Type::PyObject => write!(f, "PyObject"),
            Type::Custom(s) => write!(f, "{}", s),
            Type::Optional(inner) => write!(f, "optional<{}>", inner),
            Type::Function { params, return_type } => {
                let param_strs: Vec<String> = params.iter().map(|p| p.to_string()).collect();
                write!(f, "fn({}) -> {}", param_strs.join(", "), return_type)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

impl std::fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let op = match self {
            UnaryOp::Neg => "-",
            UnaryOp::Not => "!",
        };
        write!(f, "{}", op)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Percent,
    Pow,
    And,
    Or,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl std::fmt::Display for BinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let op = match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Percent => "%",
            BinOp::Pow => "**",
            BinOp::And => "and",
            BinOp::Or => "or",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
        };
        write!(f, "{}", op)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum FStringPart {
    Text(String),
    Expr(Box<Expr>),
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Literal),
    Identifier(String),
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
    MemberAccess {
        object: Box<Expr>,
        member: String,
    },
    MemberCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
    OptionalMemberAccess {
        object: Box<Expr>,
        member: String,
    },
    OptionalMemberCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
    Subscript {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    ListLiteral(Vec<Expr>),
    DictLiteral(Vec<(Expr, Expr)>),
    TupleLiteral(Vec<Expr>),
    SuperCall {
        method: String,
        args: Vec<Expr>,
    },
    Lambda {
        params: Vec<Param>,
        return_type: Type,
        body: Box<Expr>,
    },
    AwaitExpr(Box<Expr>),
    FString {
        parts: Vec<FStringPart>,
    },
    Super,
    Slice {
        object: Box<Expr>,
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
    },
    ListComprehension {
        element: Box<Expr>,
        var_name: String,
        iterable: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub param_type: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitMethodSignature {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchPattern {
    Literal(Literal),
    Wildcard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchCase {
    pub pattern: MatchPattern,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    FunctionDecl {
        name: String,
        params: Vec<Param>,
        return_type: Type,
        body: Vec<Stmt>,
        is_async: bool,
        decorators: Vec<String>,
    },
    ClassDecl {
        name: String,
        base_class: Option<String>,
        members: Vec<Stmt>,
    },
    TraitDecl {
        name: String,
        methods: Vec<TraitMethodSignature>,
    },
    PyImport {
        library: String,
        alias: Option<String>,
    },
    Import {
        path: Vec<String>,
        alias: Option<String>,
    },
    FromImport {
        path: Vec<String>,
        symbols: Vec<String>,
    },
    VariableDecl {
        name: String,
        is_const: bool,
        var_type: Option<Type>,
        init: Option<Expr>,
    },
    AssignStmt {
        lhs: Expr,
        value: Expr,
    },
    IfStmt {
        condition: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Option<Vec<Stmt>>,
    },
    WhileStmt {
        condition: Expr,
        body: Vec<Stmt>,
    },
    ForStmt {
        var_name: String,
        iterable: Expr,
        body: Vec<Stmt>,
    },
    TryCatchStmt {
        try_branch: Vec<Stmt>,
        catch_var: String,
        catch_branch: Vec<Stmt>,
        finally_branch: Option<Vec<Stmt>>,
    },
    RaiseStmt(Expr),
    MatchStmt {
        value: Expr,
        cases: Vec<MatchCase>,
    },
    TupleUnpack {
        vars: Vec<String>,
        init: Expr,
    },
    ExprStmt(Expr),
    ReturnStmt(Option<Expr>),
    BreakStmt,
    ContinueStmt,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}
