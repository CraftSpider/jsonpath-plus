use core::ops;
use std::fmt;

/// A source span in a path. Can be used to reference the source location of tokens or syntax
/// structures.
#[cfg_attr(docsrs, doc(cfg(feature = "spanned")))]
#[derive(Copy, Clone, PartialEq)]
pub struct Span {
    start: usize,
    end: usize,
}

impl Span {
    pub(crate) fn join(self, other: Span) -> Span {
        let start = usize::min(self.start, other.start);
        let end = usize::max(self.end, other.end);
        Span { start, end }
    }

    pub(crate) fn start(&self) -> usize {
        self.start
    }

    pub(crate) fn end(&self) -> usize {
        self.end
    }

    /// Get the string slice of this span on the source string. Note the provided string must be
    /// the whole source string for this method to be meaningful.
    #[must_use]
    pub fn get_span(self, source: &str) -> &str {
        let start = source.char_indices().nth(self.start);

        let end = source.char_indices().nth(self.end);

        let ((start, _), (end, _)) = start.zip(end).expect("Invalid source for span");

        &source[start..end]
    }
}

impl From<ops::Range<usize>> for Span {
    fn from(span: ops::Range<usize>) -> Self {
        Span {
            start: span.start,
            end: span.end,
        }
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Span")
            .field(&(self.start..self.end))
            .finish()
    }
}

impl chumsky::Span for Span {
    type Context = ();
    type Offset = usize;

    fn new(_context: Self::Context, range: ops::Range<Self::Offset>) -> Self {
        range.into()
    }

    fn context(&self) -> Self::Context {}

    fn start(&self) -> Self::Offset {
        self.start
    }

    fn end(&self) -> Self::Offset {
        self.end
    }
}

/// Trait for any item that has a retrievable source span
#[cfg(feature = "spanned")]
#[cfg_attr(docsrs, doc(cfg(feature = "spanned")))]
pub trait Spanned {
    /// Get the source span of this item
    fn span(&self) -> Span;
}

#[cfg(feature = "spanned")]
mod __impl {
    use super::*;
    use crate::ast::*;

    // Atoms

    impl Spanned for Ident {
        fn span(&self) -> Span {
            self.span
        }
    }

    impl Spanned for BoolLit {
        fn span(&self) -> Span {
            self.span
        }
    }

    impl Spanned for NullLit {
        fn span(&self) -> Span {
            self.span
        }
    }

    impl Spanned for IntLit {
        fn span(&self) -> Span {
            self.span
        }
    }

    impl Spanned for NonZeroIntLit {
        fn span(&self) -> Span {
            self.span
        }
    }

    impl Spanned for StringContent {
        fn span(&self) -> Span {
            self.span
        }
    }

    impl Spanned for SingleStringLit {
        fn span(&self) -> Span {
            self.start
                .span()
                .join(self.content.span())
                .join(self.end.span())
        }
    }

    impl Spanned for DoubleStringLit {
        fn span(&self) -> Span {
            self.start
                .span()
                .join(self.content.span())
                .join(self.end.span())
        }
    }

    impl Spanned for StringLit {
        fn span(&self) -> Span {
            match self {
                StringLit::Single(s) => s.span(),
                StringLit::Double(d) => d.span(),
            }
        }
    }

    // AST

    impl Spanned for Path {
        fn span(&self) -> Span {
            let mut out = self.dollar.span();

            for s in &self.segments {
                out = out.join(s.span());
            }

            if let Some(t) = &self.tilde {
                out = out.join(t.span());
            }

            out
        }
    }

    impl Spanned for SubPath {
        fn span(&self) -> Span {
            let mut out = self.kind.span();

            for s in &self.segments {
                out = out.join(s.span());
            }

            if let Some(t) = &self.tilde {
                out = out.join(t.span());
            }

            out
        }
    }

    impl Spanned for PathKind {
        fn span(&self) -> Span {
            match self {
                PathKind::Root(d) => d.span(),
                PathKind::Relative(a) => a.span(),
            }
        }
    }

    impl Spanned for Segment {
        fn span(&self) -> Span {
            match self {
                Segment::Bracket(b, i) => b.span().join(i.span()),
                Segment::Dot(d, i) => d.span().join(i.span()),
                Segment::Recursive(r, i) => i
                    .as_ref()
                    .map_or_else(|| r.span(), |i| r.span().join(i.span())),
            }
        }
    }

    impl Spanned for BracketSelector {
        fn span(&self) -> Span {
            match self {
                BracketSelector::Union(comps) => {
                    let mut out = comps[0].span();
                    for comp in &comps[1..] {
                        out = out.join(comp.span());
                    }
                    out
                }
                BracketSelector::StepRange(sr) => sr.span(),
                BracketSelector::Range(r) => r.span(),
                BracketSelector::Wildcard(s) => s.span(),
                BracketSelector::Parent(c) => c.span(),
                BracketSelector::Path(sp) => sp.span(),
                BracketSelector::Filter(f) => f.span(),
                BracketSelector::Literal(lit) => lit.span(),
            }
        }
    }

    impl Spanned for RawSelector {
        fn span(&self) -> Span {
            match self {
                RawSelector::Wildcard(s) => s.span(),
                RawSelector::Parent(c) => c.span(),
                RawSelector::Name(i) => i.span(),
            }
        }
    }

    impl Spanned for UnionComponent {
        fn span(&self) -> Span {
            match self {
                UnionComponent::StepRange(sr) => sr.span(),
                UnionComponent::Range(r) => r.span(),
                UnionComponent::Parent(c) => c.span(),
                UnionComponent::Path(sp) => sp.span(),
                UnionComponent::Filter(f) => f.span(),
                UnionComponent::Literal(lit) => lit.span(),
            }
        }
    }

    impl Spanned for StepRange {
        fn span(&self) -> Span {
            let mut out = self
                .start
                .as_ref()
                .map_or_else(|| self.colon1.span(), |s| s.span().join(self.colon1.span()));

            if let Some(end) = &self.end {
                out = out.join(end.span());
            }

            out = out.join(self.colon2.span());

            if let Some(step) = &self.step {
                out = out.join(step.span());
            }

            out
        }
    }

    impl Spanned for Range {
        fn span(&self) -> Span {
            let mut out = self
                .start
                .as_ref()
                .map_or_else(|| self.colon.span(), |s| s.span().join(self.colon.span()));

            if let Some(end) = &self.end {
                out = out.join(end.span());
            }

            out
        }
    }

    impl Spanned for BracketLit {
        fn span(&self) -> Span {
            match self {
                BracketLit::Int(i) => i.span(),
                BracketLit::String(s) => s.span(),
            }
        }
    }

    impl Spanned for Filter {
        fn span(&self) -> Span {
            self.question
                .span()
                .join(self.paren.span())
                .join(self.inner.span())
        }
    }

    impl Spanned for FilterExpr {
        fn span(&self) -> Span {
            match self {
                FilterExpr::Unary(op, expr) => op.span().join(expr.span()),
                FilterExpr::Binary(lhs, op, rhs) => lhs.span().join(op.span()).join(rhs.span()),
                FilterExpr::Path(sp) => sp.span(),
                FilterExpr::Lit(el) => el.span(),
                FilterExpr::Parens(p, expr) => p.span().join(expr.span()),
            }
        }
    }

    impl Spanned for ExprLit {
        fn span(&self) -> Span {
            match self {
                ExprLit::Int(i) => i.span(),
                ExprLit::String(s) => s.span(),
                ExprLit::Bool(b) => b.span(),
                ExprLit::Null(n) => n.span(),
            }
        }
    }

    impl Spanned for UnOp {
        fn span(&self) -> Span {
            match self {
                UnOp::Neg(d) => d.span(),
                UnOp::Not(b) => b.span(),
            }
        }
    }

    impl Spanned for BinOp {
        fn span(&self) -> Span {
            match self {
                BinOp::And(a) => a.span(),
                BinOp::Or(p) => p.span(),
                BinOp::Eq(e) => e.span(),
                BinOp::Le(l) => l.span(),
                BinOp::Lt(l) => l.span(),
                BinOp::Gt(g) => g.span(),
                BinOp::Ge(g) => g.span(),
                BinOp::Add(p) => p.span(),
                BinOp::Sub(d) => d.span(),
                BinOp::Mul(s) => s.span(),
                BinOp::Div(s) => s.span(),
                BinOp::Rem(p) => p.span(),
            }
        }
    }
}
