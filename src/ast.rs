//! Syntax tree that backs a path. If you just want to use paths, you shouldn't touch this.
//! This is exposed for users who want to provide things like syntax highlighting of paths
//! or similar.

#![cfg_attr(not(feature = "spanned"), allow(dead_code))]

use core::num::NonZeroI64;

mod eval;
mod parse;
#[cfg(feature = "spanned")]
mod span;
mod token;

#[cfg(feature = "spanned")]
pub use span::{Span, Spanned};

// Aliases

type Input = char;
type Error = chumsky::error::Simple<char>;

// Atoms

/// A raw identifier, the `foo` in `.foo`
pub struct Ident {
    #[cfg(feature = "spanned")]
    span: Span,
    val: String,
}

impl Ident {
    /// Get the string representation of this identifier
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.val
    }
}

/// A boolean literal, such as `true` or `false`
pub struct BoolLit {
    #[cfg(feature = "spanned")]
    span: Span,
    val: bool,
}

impl BoolLit {
    /// Get the boolean representation of this literal
    #[must_use]
    pub fn as_bool(&self) -> bool {
        self.val
    }
}

/// A null literal, the keyword `null`
pub struct NullLit {
    #[cfg(feature = "spanned")]
    span: Span,
}

/// An integer literal, such as `-3`
pub struct IntLit {
    #[cfg(feature = "spanned")]
    span: Span,
    val: i64,
}

impl IntLit {
    /// Get the integer representation of this literal
    #[must_use]
    pub fn as_int(&self) -> i64 {
        self.val
    }
}

/// A non-zero integer literal, any integer not `0`
pub struct NonZeroIntLit {
    #[cfg(feature = "spanned")]
    span: Span,
    val: NonZeroI64,
}

impl NonZeroIntLit {
    /// Get the integer representation of this literal
    #[must_use]
    pub fn as_int(&self) -> NonZeroI64 {
        self.val
    }
}

struct StringContent {
    #[cfg(feature = "spanned")]
    span: Span,
    val: String,
}

/// An apostrophe-delimited string
pub struct SingleStringLit {
    start: token::SingleQuote,
    content: StringContent,
    end: token::SingleQuote,
}

impl SingleStringLit {
    /// Get the content of this string literal
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.content.val
    }
}

/// A quote-delimite string
pub struct DoubleStringLit {
    start: token::DoubleQuote,
    content: StringContent,
    end: token::DoubleQuote,
}

impl DoubleStringLit {
    /// Get the content of this string literal
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.content.val
    }
}

/// Any string literal, whether single or double quote delimited
pub enum StringLit {
    /// A single-quoted string literal
    Single(SingleStringLit),
    /// A double-quoted string literal
    Double(DoubleStringLit),
}

impl StringLit {
    /// Get the content of this string literal
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            StringLit::Single(s) => &s.content.val,
            StringLit::Double(s) => &s.content.val,
        }
    }
}

// Main AST

/// A compiled JSON path. Can be used to match against items any number of times, preventing
/// recompilation of the same pattern many times.
#[must_use = "A path does nothing on its own, call `find` or `find_str` to evaluate the path on a \
              value"]
pub struct Path {
    dollar: token::Dollar,
    segments: Vec<Segment>,
    tilde: Option<token::Tilde>,
}

impl Path {
    /// A slice of the segments this path contains
    #[must_use]
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }
}

/// A sub-path, such as in a filter or as a bracket selector. Can be based off the root or the
/// current location
pub struct SubPath {
    kind: PathKind,
    segments: Vec<Segment>,
    tilde: Option<token::Tilde>,
}

impl SubPath {
    /// The kind of this sub-path
    #[must_use]
    pub fn kind(&self) -> &PathKind {
        &self.kind
    }

    /// A slice of the segments this path contains
    #[must_use]
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// Whether this path references the IDs of the matched items, or the items themselves
    #[must_use]
    pub fn is_id(&self) -> bool {
        self.tilde.is_some()
    }
}

/// The kind of a sub-path. Either root-based or relative
pub enum PathKind {
    /// A root-based path
    Root(token::Dollar),
    /// A relative path
    Relative(token::At),
}

impl PathKind {
    /// Whether this is an absolute root based path kind
    #[must_use]
    pub fn is_root(&self) -> bool {
        matches!(self, PathKind::Root(_))
    }

    /// Whether this is a relative path kind
    #[must_use]
    pub fn is_relative(&self) -> bool {
        matches!(self, PathKind::Relative(_))
    }
}

/// A single segement selector in a path
pub enum Segment {
    /// A dot followed by a simple selector, `.a`
    Dot(token::Dot, RawSelector),
    /// A bracket containing a complex selector, `[?(...)]`
    Bracket(token::Bracket, BracketSelector),
    /// A recursive selector optionally followed by a simple selector, `..foo`
    Recursive(token::DotDot, Option<RecursiveOp>),
}

/// The optional selector following a recursive selector
pub enum RecursiveOp {
    /// A simple selector, see [`RawSelector`]
    Raw(RawSelector),
    /// A complex selector, see [`BracketSelector`]
    Bracket(token::Bracket, BracketSelector),
}

/// The raw selector following a dot
pub enum RawSelector {
    /// A wildcard selector to get all children, `.*`
    Wildcard(token::Star),
    /// A parent selector to retrieve the parent of the matched item, `.^`
    Parent(token::Caret),
    /// A name ident selector to retrieve the matched name in an object, `.my_name`
    Name(Ident),
}

/// A range for selecting keys from an array from a start to an end key, with an extra parameter to
/// select every Nth key
pub struct StepRange {
    start: Option<IntLit>,
    colon1: token::Colon,
    end: Option<IntLit>,
    colon2: token::Colon,
    step: Option<NonZeroIntLit>,
}

impl StepRange {
    /// Get the start literal token for this range
    #[must_use]
    pub fn start_lit(&self) -> Option<&IntLit> {
        self.start.as_ref()
    }

    /// Get the end literal token for this range
    #[must_use]
    pub fn end_lit(&self) -> Option<&IntLit> {
        self.end.as_ref()
    }

    /// Get the step literal token for this range
    #[must_use]
    pub fn step_lit(&self) -> Option<&NonZeroIntLit> {
        self.step.as_ref()
    }

    /// Get the user-provided literal start for this range
    #[must_use]
    pub fn start(&self) -> Option<i64> {
        self.start.as_ref().map(|a| a.val)
    }

    /// Get the user-provided literal end for this range
    #[must_use]
    pub fn end(&self) -> Option<i64> {
        self.end.as_ref().map(|a| a.val)
    }

    /// Get the user-provided step value for this range
    #[must_use]
    pub fn step(&self) -> Option<NonZeroI64> {
        self.step.as_ref().map(|a| a.val)
    }
}

/// A range for selecting keys from an array from a start to an end key
pub struct Range {
    start: Option<IntLit>,
    colon: token::Colon,
    end: Option<IntLit>,
}

impl Range {
    /// Get the start literal token for this range
    #[must_use]
    pub fn start_lit(&self) -> Option<&IntLit> {
        self.start.as_ref()
    }

    /// Get the end literal token for this range
    #[must_use]
    pub fn end_lit(&self) -> Option<&IntLit> {
        self.end.as_ref()
    }

    /// Get the user-provided literal start for this range
    #[must_use]
    pub fn start(&self) -> Option<i64> {
        self.start.as_ref().map(|a| a.val)
    }

    /// Get the user-provided literal end for this range
    #[must_use]
    pub fn end(&self) -> Option<i64> {
        self.end.as_ref().map(|a| a.val)
    }
}

/// A component of a bracket union selector
pub enum UnionComponent {
    /// A range selector with explicit step
    StepRange(StepRange),
    /// A range selector with implicit step
    Range(Range),
    /// A parent selector to retrieve the parent of the matched item
    Parent(token::Caret),
    /// A sub-path selector to retrieve keys from a matched path
    Path(SubPath),
    /// A filter selector to retrieve items matching a predicate
    Filter(Filter),
    /// A literal selector to retrieve the mentioned keys
    Literal(BracketLit),
}

/// The inside of a bracket selector segment
pub enum BracketSelector {
    /// A union of multiple selectors, `[1, 3, 9]`
    Union(Vec<UnionComponent>),
    /// A range selector with explicit step, `[1:5:2]`
    StepRange(StepRange),
    /// A range selector with implicit step, `[2:8]`
    Range(Range),
    /// A wildcard selector to get all children, `[*]`
    Wildcard(token::Star),
    /// A parent selector to retrieve the parent of the matched item, `[^]`
    Parent(token::Caret),
    /// A sub-path selector to retrieve keys from a matched path, `[$.foo.bar]`
    Path(SubPath),
    /// A filter selector to retrieve items matching a predicate, `[?(expr)]`
    Filter(Filter),
    /// A literal selector to retrieve the mentioned keys, `[6]` or `['qux']`
    Literal(BracketLit),
}

/// A literal selector inside of brackets, `0` or `'a'`
pub enum BracketLit {
    /// An integer literal, see [`IntLit`]
    Int(IntLit),
    /// A string literal, see [`StringLit`]
    String(StringLit),
}

/// A filter selector inside of brackets, `?(...)`
pub struct Filter {
    question: token::Question,
    paren: token::Paren,
    inner: FilterExpr,
}

impl Filter {
    /// The inner expression of this filter
    #[must_use]
    pub fn expression(&self) -> &FilterExpr {
        &self.inner
    }
}

/// A literal inside an expression
pub enum ExprLit {
    /// An integer literal, see [`IntLit`]
    Int(IntLit),
    /// A string literal, see [`StringLit`]
    Str(StringLit),
    /// A boolean literal, see [`BoolLit`]
    Bool(BoolLit),
    /// A null literal, see [`NullLit`]
    Null(NullLit),
}

/// An expression inside a filter directive, or any sub-expression in that tree
pub enum FilterExpr {
    /// An expression with an unary operator before it, such as `!(true)`
    Unary(UnOp, Box<FilterExpr>),
    /// Two expressions with a binary operator joining them, such as `1 + 4`
    Binary(Box<FilterExpr>, BinOp, Box<FilterExpr>),
    /// A [`SubPath`] expression, such as the `@.a` in `@.a == 1`
    Path(SubPath),
    /// A literal value, such as `null` or `'bar'`
    Lit(ExprLit),
    /// An expression wrapped in parenthesis, such as the `(1 + 2)` in `(1 + 2) * 3`
    Parens(token::Paren, Box<FilterExpr>),
}

/// An unary operator in an expression
pub enum UnOp {
    /// `-`
    Neg(token::Dash),
    /// `!`
    Not(token::Bang),
}

/// A binary operator in an expression
pub enum BinOp {
    /// `&&`
    And(token::DoubleAnd),
    /// `||`
    Or(token::DoublePipe),

    /// `==`
    Eq(token::EqEq),
    /// `<=`
    Le(token::LessEq),
    /// `<`
    Lt(token::LessThan),
    /// `>`
    Gt(token::GreaterThan),
    /// `>=`
    Ge(token::GreaterEq),

    /// `+`
    Add(token::Plus),
    /// `-`
    Sub(token::Dash),
    /// `*`
    Mul(token::Star),
    /// `/`
    Div(token::RightSlash),
    /// `%`
    Rem(token::Percent),
}
