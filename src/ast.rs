use std::num::NonZeroI64;

mod eval;
mod parse;
#[cfg(feature = "spanned")]
mod span;
mod token;

#[cfg(feature = "spanned")]
pub use span::Span;

// Aliases

type Input = char;
type Error = chumsky::error::Simple<char>;

// Items

pub struct Ident {
    #[cfg(feature = "spanned")]
    _span: Span,
    val: String,
}

impl Ident {
    pub fn as_str(&self) -> &str {
        &self.val
    }
}

pub struct IntLit {
    #[cfg(feature = "spanned")]
    span: Span,
    val: i64,
}

pub struct NonZeroIntLit {
    #[cfg(feature = "spanned")]
    _span: Span,
    val: NonZeroI64,
}

pub struct StringContent {
    #[cfg(feature = "spanned")]
    _span: Span,
    val: String,
}

pub struct SingleStringLit {
    _start: token::SingleQuote,
    content: StringContent,
    _end: token::SingleQuote,
}

pub struct DoubleStringLit {
    _start: token::DoubleQuote,
    content: StringContent,
    _end: token::DoubleQuote,
}

pub enum StringLit {
    Single(SingleStringLit),
    Double(DoubleStringLit),
}

impl StringLit {
    fn as_str(&self) -> &str {
        match self {
            StringLit::Single(s) => &s.content.val,
            StringLit::Double(s) => &s.content.val,
        }
    }
}

pub struct BoolLit {
    #[cfg(feature = "spanned")]
    _span: Span,
    val: bool,
}

pub struct NullLit {
    #[cfg(feature = "spanned")]
    _span: Span,
}

/// A compiled JSON path. Can be used to match against items any number of times, preventing
/// recompilation of the same pattern many times.
#[must_use = "A path does nothing on its own, call `find` or `find_str` to evaluate the path on a \
              value"]
pub struct Path {
    _dollar: token::Dollar,
    segments: Vec<Segment>,
    tilde: Option<token::Tilde>,
}

pub struct SubPath {
    kind: PathKind,
    segments: Vec<Segment>,
    tilde: Option<token::Tilde>,
}

pub enum PathKind {
    Root(token::Dollar),
    Relative(token::At),
}

pub enum Segment {
    Dot(token::Dot, DotIdent),
    Bracket(token::Bracket, BracketInner),
    Recursive(token::DotDot, Option<RecursiveOp>),
}

pub enum RecursiveOp {
    Raw(DotIdent),
    Bracket(token::Bracket, BracketInner),
}

pub enum DotIdent {
    Wildcard(token::Star),
    Parent(token::Caret),
    Name(Ident),
}

pub struct StepRange {
    start: Option<IntLit>,
    _colon1: token::Colon,
    end: Option<IntLit>,
    _colon2: token::Colon,
    step: Option<NonZeroIntLit>,
}

pub struct Range {
    start: Option<IntLit>,
    _colon: token::Colon,
    end: Option<IntLit>,
}

pub enum UnionComponent {
    StepRange(StepRange),
    Range(Range),
    Parent(token::Caret),
    Path(SubPath),
    Filter(Filter),
    Literal(BracketLit),
}

pub enum BracketInner {
    Union(Vec<UnionComponent>),
    StepRange(StepRange),
    Range(Range),
    Wildcard(token::Star),
    Parent(token::Caret),
    Path(SubPath),
    Filter(Filter),
    Literal(BracketLit),
}

pub enum BracketLit {
    Int(IntLit),
    String(StringLit),
}

pub struct Filter {
    _question: token::Question,
    _paren: token::Paren,
    inner: FilterExpr,
}

pub enum ExprLit {
    Int(IntLit),
    Str(StringLit),
    Bool(BoolLit),
    Null(NullLit),
}

pub enum FilterExpr {
    Unary(UnOp, Box<FilterExpr>),
    Binary(Box<FilterExpr>, BinOp, Box<FilterExpr>),
    Path(SubPath),
    Lit(ExprLit),
    Parens(token::Paren, Box<FilterExpr>),
}

pub enum UnOp {
    Neg(token::Dash),
    Not(token::Bang),
}

pub enum BinOp {
    And(token::DoubleAnd),
    Or(token::DoublePipe),

    Eq(token::EqEq),
    Le(token::LessEq),
    Lt(token::LessThan),
    Gt(token::GreaterThan),
    Ge(token::GreaterEq),

    Add(token::Plus),
    Sub(token::Dash),
    Mul(token::Star),
    Div(token::RightSlash),
    Rem(token::Percent),
}
