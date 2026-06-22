//! The abstract syntax tree.
//!
//! Idiomatic Rust enums (vs. FreeMat's homogeneous `Tree` of typed tokens).
//! Every node carries a byte [`Span`]. The three roots a source unit can parse
//! into are captured by [`Program`].
//!
//! Node taxonomy:
//! - [`Expr`] — expressions (literals, operators, indexing, matrices, ...).
//! - [`Stmt`] — statements (assignments, control flow, declarations, ...).
//! - [`FunctionDef`] — a `function ... end` definition.

use crate::span::Span;

/// What a whole source unit parses into.
#[derive(Debug, Clone, PartialEq)]
pub enum Program {
    /// A sequence of statements (a `.m` script or REPL input).
    Script(Vec<Stmt>),
    /// One or more `function` definitions (a `.m` function file).
    Functions(Vec<FunctionDef>),
}

/// A `function [out...] = name(in...) body end` definition.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDef {
    /// Function name.
    pub name: String,
    /// Output parameter names (`[a, b]` or a single `a`; empty if none).
    pub outputs: Vec<String>,
    /// Input parameters.
    pub inputs: Vec<Param>,
    /// The function body.
    pub body: Vec<Stmt>,
    /// Nested function definitions declared within this function.
    pub nested: Vec<FunctionDef>,
    /// Span of the whole definition.
    pub span: Span,
}

/// A function input parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    /// Parameter name.
    pub name: String,
    /// Pass-by-reference (`&name`, a FreeMat extension).
    pub by_ref: bool,
    /// Span of the parameter.
    pub span: Span,
}

/// Statement-terminator style, which controls whether a result is displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Terminator {
    /// `;` — suppress display of the statement's result.
    Quiet,
    /// `,` or newline — display the statement's result.
    Display,
}

/// A statement, plus how it was terminated.
#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    /// What the statement does.
    pub kind: StmtKind,
    /// How it was terminated (controls echo).
    pub terminator: Terminator,
    /// Span of the statement (excluding the terminator).
    pub span: Span,
}

/// The kinds of statements.
#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    /// A bare expression evaluated for its value/effect (e.g. `1+2` or `foo()`).
    Expr(Expr),
    /// `lhs = rhs` (single LHS, possibly indexed/field-accessed).
    Assign { lhs: Expr, rhs: Expr },
    /// `[a, b, ...] = rhs` multi-output assignment.
    MultiAssign { lhs: Vec<Expr>, rhs: Expr },
    /// Command syntax: `hold on` → `hold('on')`. Args are string literals.
    Command { name: String, args: Vec<String> },
    /// `for var = range body end`.
    For {
        var: String,
        range: Expr,
        body: Vec<Stmt>,
    },
    /// `while cond body end`.
    While { cond: Expr, body: Vec<Stmt> },
    /// `if ... elseif ... else ... end`.
    If {
        cond: Expr,
        body: Vec<Stmt>,
        elseifs: Vec<ElseIf>,
        else_body: Option<Vec<Stmt>>,
    },
    /// `switch expr case ... otherwise ... end`.
    Switch {
        scrutinee: Expr,
        cases: Vec<Case>,
        otherwise: Option<Vec<Stmt>>,
    },
    /// `try body catch [ident] handler end`.
    Try {
        body: Vec<Stmt>,
        catch: Option<Vec<Stmt>>,
    },
    /// `global a b c`.
    Global(Vec<String>),
    /// `persistent a b c`.
    Persistent(Vec<String>),
    /// A keyword statement with no operands.
    Keyword(KeywordStmt),
    /// A nested `function` defined inside a script/function body.
    FunctionDef(Box<FunctionDef>),
}

/// Operand-less keyword statements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeywordStmt {
    /// `break`
    Break,
    /// `continue`
    Continue,
    /// `return`
    Return,
    /// `retall`
    Retall,
    /// `keyboard`
    Keyboard,
    /// `quit`
    Quit,
    /// `dbup`
    DbUp,
    /// `dbdown`
    DbDown,
}

/// An `elseif cond body` arm.
#[derive(Debug, Clone, PartialEq)]
pub struct ElseIf {
    /// The arm's condition.
    pub cond: Expr,
    /// The arm's body.
    pub body: Vec<Stmt>,
    /// Span of the arm.
    pub span: Span,
}

/// A `case label body` arm of a `switch`.
#[derive(Debug, Clone, PartialEq)]
pub struct Case {
    /// The case label expression (a value or a cell of values).
    pub label: Expr,
    /// The case body.
    pub body: Vec<Stmt>,
    /// Span of the arm.
    pub span: Span,
}

/// An expression node.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    /// What the expression is.
    pub kind: ExprKind,
    /// Span of the expression.
    pub span: Span,
}

/// The kinds of expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// A real numeric literal; `single` marks an `f` suffix.
    Real { text: String, single: bool },
    /// An imaginary numeric literal; `single` marks an `f` suffix.
    Imag { text: String, single: bool },
    /// A string literal (already unescaped).
    Str(String),
    /// An identifier reference.
    Ident(String),
    /// `end` used inside an index context.
    End,
    /// The bare `:` magic-colon (e.g. `A(:)`).
    Colon,
    /// A unary prefix operator applied to an operand.
    Unary { op: UnaryOp, operand: Box<Expr> },
    /// A binary operator applied to two operands.
    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    /// A range `start:stop` or `start:step:stop`.
    Range {
        start: Box<Expr>,
        step: Option<Box<Expr>>,
        stop: Box<Expr>,
    },
    /// A postfix transpose (`'`) or element-wise transpose (`.'`).
    Transpose { operand: Box<Expr>, conjugate: bool },
    /// `base(args)` — paren indexing / function call.
    Index { base: Box<Expr>, args: Vec<Expr> },
    /// `base(args, /key=val, /key)` — an IDL-style keyword call. Produced only
    /// when at least one `/keyword` argument is present; ordinary calls stay
    /// [`Index`](ExprKind::Index). `args` holds the positional arguments (in
    /// source order); `keywords` holds the named ones, each a `(name, value)`
    /// where `value` is `None` for the bare `/name` form (binds `logical(1)`).
    KeywordCall {
        base: Box<Expr>,
        args: Vec<Expr>,
        keywords: Vec<(String, Option<Expr>)>,
    },
    /// `base{args}` — cell-content (brace) indexing.
    CellIndex { base: Box<Expr>, args: Vec<Expr> },
    /// `base.field` — static field access.
    Field { base: Box<Expr>, name: String },
    /// `base.(expr)` — dynamic field access.
    DynField { base: Box<Expr>, name: Box<Expr> },
    /// A `[...]` matrix literal (rows of elements).
    Matrix(Vec<Vec<Expr>>),
    /// A `{...}` cell-array literal (rows of elements).
    Cell(Vec<Vec<Expr>>),
    /// `@name` — a named function handle.
    FuncHandle(String),
    /// `@(args) body` — an anonymous function.
    AnonFunc {
        params: Vec<String>,
        body: Box<Expr>,
    },
}

/// Unary prefix operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// `+x`
    Plus,
    /// `-x`
    Minus,
    /// `~x` (logical not)
    Not,
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    RDiv,
    /// `\`
    LDiv,
    /// `^`
    Pow,
    /// `.*`
    ElMul,
    /// `./`
    ElRDiv,
    /// `.\`
    ElLDiv,
    /// `.^`
    ElPow,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `<=`
    Le,
    /// `>=`
    Ge,
    /// `==`
    Eq,
    /// `~=`
    Ne,
    /// `&`
    And,
    /// `|`
    Or,
    /// `&&`
    ShortAnd,
    /// `||`
    ShortOr,
}
