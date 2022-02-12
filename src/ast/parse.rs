use super::*;
use chumsky::prelude::*;

impl Ident {
    fn parser() -> impl Parser<Input, Ident, Error = Error> {
        filter::<_, _, Error>(|c: &char| c.is_alphanumeric() || *c == '-' || *c == '_')
            .repeated()
            .at_least(1)
            .map_with_span(|val, _span| Ident {
                #[cfg(feature = "spanned")]
                span: _span.into(),
                val: String::from_iter(val),
            })
    }
}

impl IntLit {
    fn parser() -> impl Parser<Input, IntLit, Error = Error> {
        just::<_, _, Error>('-')
            .or_not()
            .then(filter(|c: &char| c.is_numeric()).repeated().at_least(1))
            .map_with_span(|(neg, val), _span| IntLit {
                #[cfg(feature = "spanned")]
                span: _span.into(),
                val: match (String::from_iter(val).parse::<i64>().unwrap(), neg) {
                    (val, Some(_)) => -val,
                    (val, None) => val,
                },
            })
    }
}

impl NonZeroIntLit {
    fn parser() -> impl Parser<Input, NonZeroIntLit, Error = Error> {
        IntLit::parser().try_map(|il, span| {
            Ok(NonZeroIntLit {
                #[cfg(feature = "spanned")]
                span: il.span(),
                val: il
                    .as_int()
                    .try_into()
                    .map_err(|_| Simple::custom(span, "Expected a non-zero integer literal"))?,
            })
        })
    }
}

impl StringContent {
    fn parser(delimiter: char) -> impl Parser<Input, StringContent, Error = Error> {
        none_of::<_, _, Error>([delimiter])
            .or(just(format!("\\{}", delimiter)).to(delimiter))
            .repeated()
            .map_with_span(|content, _span| StringContent {
                #[cfg(feature = "spanned")]
                span: _span.into(),
                val: String::from_iter(content),
            })
    }
}

impl SingleStringLit {
    fn parser() -> impl Parser<Input, SingleStringLit, Error = Error> {
        token::SingleQuote::parser()
            .then(StringContent::parser('\''))
            .then(token::SingleQuote::parser())
            .map(|((start, content), end)| SingleStringLit {
                start,
                content,
                end,
            })
    }
}

impl DoubleStringLit {
    fn parser() -> impl Parser<Input, DoubleStringLit, Error = Error> {
        token::DoubleQuote::parser()
            .then(StringContent::parser('"'))
            .then(token::DoubleQuote::parser())
            .map(|((start, content), end)| DoubleStringLit {
                start,
                content,
                end,
            })
    }
}

impl StringLit {
    fn parser() -> impl Parser<Input, StringLit, Error = Error> {
        SingleStringLit::parser()
            .map(StringLit::Single)
            .or(DoubleStringLit::parser().map(StringLit::Double))
    }
}

impl BoolLit {
    fn parser() -> impl Parser<Input, BoolLit, Error = Error> {
        just::<_, _, Error>("true")
            .to(true)
            .or(just("false").to(false))
            .map_with_span(|val, _span| BoolLit {
                #[cfg(feature = "spanned")]
                span: _span.into(),
                val,
            })
    }
}

impl NullLit {
    fn parser() -> impl Parser<Input, NullLit, Error = Error> {
        just::<_, _, Error>("null").map_with_span(|_, _span| NullLit {
            #[cfg(feature = "spanned")]
            span: _span.into(),
        })
    }
}

impl Path {
    pub(crate) fn parser() -> impl Parser<Input, Path, Error = Error> {
        token::Dollar::parser()
            .then(Segment::parser().repeated())
            .then(token::Tilde::parser().or_not())
            .then_ignore(end())
            .map(|((dollar, segments), tilde)| Path {
                dollar,
                segments,
                tilde,
            })
    }
}

impl SubPath {
    fn parser(
        operator: impl Parser<Input, Segment, Error = Error>,
    ) -> impl Parser<Input, SubPath, Error = Error> {
        PathKind::parser()
            .then(operator.repeated())
            .then(token::Tilde::parser().or_not())
            .map(|((kind, segments), tilde)| SubPath {
                kind,
                segments,
                tilde,
            })
    }
}

impl PathKind {
    fn parser() -> impl Parser<Input, PathKind, Error = Error> {
        token::Dollar::parser()
            .map(PathKind::Root)
            .or(token::At::parser().map(PathKind::Relative))
    }
}

impl Segment {
    fn parser() -> impl Parser<Input, Segment, Error = Error> {
        recursive(|operator| {
            token::DotDot::parser()
                .then(RawSelector::parser().or_not())
                .map(|(dotdot, op)| Segment::Recursive(dotdot, op))
                .or(token::Bracket::parser(BracketSelector::parser(operator))
                    .map(|(brack, inner)| Segment::Bracket(brack, inner)))
                .or(token::Dot::parser()
                    .then(RawSelector::parser())
                    .map(|(dot, ident)| Segment::Dot(dot, ident)))
        })
    }
}

impl RawSelector {
    fn parser() -> impl Parser<Input, RawSelector, Error = Error> {
        token::Star::parser()
            .map(RawSelector::Wildcard)
            .or(token::Caret::parser().map(RawSelector::Parent))
            .or(Ident::parser().map(RawSelector::Name))
    }
}

impl StepRange {
    fn parser() -> impl Parser<Input, StepRange, Error = Error> {
        IntLit::parser()
            .or_not()
            .then(token::Colon::parser())
            .then(IntLit::parser().or_not())
            .then(token::Colon::parser())
            .then(NonZeroIntLit::parser().or_not())
            .map(|((((start, colon1), end), colon2), step)| StepRange {
                start,
                colon1,
                end,
                colon2,
                step,
            })
    }
}

impl Range {
    fn parser() -> impl Parser<Input, Range, Error = Error> {
        IntLit::parser()
            .or_not()
            .then(token::Colon::parser())
            .then(IntLit::parser().or_not())
            .map(|((start, colon), end)| Range { start, colon, end })
    }
}

impl UnionComponent {
    fn parser(
        operator: impl Parser<Input, Segment, Error = Error> + Clone + 'static,
    ) -> impl Parser<Input, UnionComponent, Error = Error> {
        StepRange::parser()
            .map(UnionComponent::StepRange)
            .or(Range::parser().map(UnionComponent::Range))
            .or(token::Caret::parser().map(UnionComponent::Parent))
            .or(SubPath::parser(operator.clone()).map(UnionComponent::Path))
            .or(Filter::parser(operator).map(UnionComponent::Filter))
            .or(BracketLit::parser().map(UnionComponent::Literal))
            .padded()
    }
}

impl BracketSelector {
    fn parser(
        operator: impl Parser<Input, Segment, Error = Error> + Clone + 'static,
    ) -> impl Parser<Input, BracketSelector, Error = Error> {
        StepRange::parser().map(BracketSelector::StepRange)
            .or(Range::parser().map(BracketSelector::Range))
            .or(token::Star::parser().map(BracketSelector::Wildcard))
            .or(token::Caret::parser().map(BracketSelector::Parent))
            .or(SubPath::parser(operator.clone()).map(BracketSelector::Path))
            .or(Filter::parser(operator.clone()).map(BracketSelector::Filter))
            .or(BracketLit::parser().map(BracketSelector::Literal))
            .padded()
            // Handle unions last to avoid constant backtracking
            .then(just(',').ignore_then(UnionComponent::parser(operator)).repeated().at_least(1).or_not())
            .try_map(|(select, union), _span| {
                Ok(match union {
                    Some(mut union) => {
                        #[cfg(feature = "spanned")]
                        let select_span = select.span().as_range();
                        #[cfg(not(feature = "spanned"))]
                        let select_span = _span;
                        union.insert(
                            0,
                            select
                                .try_into()
                                .map_err(|_| Simple::custom(select_span, "Union operator doesn't support wildcard"))?
                        );
                        BracketSelector::Union(union)
                    },
                    None => select,
                })
            })
    }
}

impl BracketLit {
    fn parser() -> impl Parser<Input, BracketLit, Error = Error> {
        IntLit::parser()
            .map(BracketLit::Int)
            .or(StringLit::parser().map(BracketLit::String))
    }
}

impl Filter {
    fn parser(
        operator: impl Parser<Input, Segment, Error = Error> + Clone + 'static,
    ) -> impl Parser<Input, Filter, Error = Error> {
        token::Question::parser()
            .then(token::Paren::parser(FilterExpr::parser(operator)))
            .map(|(question, (paren, inner))| Filter {
                question,
                paren,
                inner,
            })
    }
}

impl ExprLit {
    fn parser() -> impl Parser<Input, ExprLit, Error = Error> {
        IntLit::parser()
            .map(ExprLit::Int)
            .or(StringLit::parser().map(ExprLit::String))
            .or(BoolLit::parser().map(ExprLit::Bool))
            .or(NullLit::parser().map(ExprLit::Null))
    }
}

impl FilterExpr {
    fn parser(
        operator: impl Parser<Input, Segment, Error = Error> + Clone + 'static,
    ) -> impl Parser<Input, FilterExpr, Error = Error> {
        recursive(|filt_expr| {
            let atom = SubPath::parser(operator)
                .map(FilterExpr::Path)
                .or(ExprLit::parser().map(FilterExpr::Lit))
                .or(token::Paren::parser(filt_expr)
                    .map(|(p, expr)| FilterExpr::Parens(p, Box::new(expr))));

            let unary = UnOp::parser()
                .repeated()
                .then(atom)
                .foldr(|op, rhs| FilterExpr::Unary(op, Box::new(rhs)));

            let precedence = [
                BinOp::product_parser().boxed(),
                BinOp::sum_parser().boxed(),
                BinOp::cmp_parser().boxed(),
                BinOp::and_parser().boxed(),
                BinOp::or_parser().boxed(),
            ];

            let mut last = unary.boxed();

            for ops in precedence {
                last = last
                    .clone()
                    .then(ops.padded().then(last).repeated())
                    .foldl(|lhs, (op, rhs)| FilterExpr::Binary(Box::new(lhs), op, Box::new(rhs)))
                    .boxed();
            }

            last
        })
    }
}

impl UnOp {
    fn parser() -> impl Parser<Input, UnOp, Error = Error> {
        token::Dash::parser()
            .map(UnOp::Neg)
            .or(token::Bang::parser().map(UnOp::Not))
    }
}

impl BinOp {
    fn product_parser() -> impl Parser<Input, BinOp, Error = Error> {
        token::Star::parser()
            .map(BinOp::Mul)
            .or(token::RightSlash::parser().map(BinOp::Div))
            .or(token::Percent::parser().map(BinOp::Rem))
    }

    fn sum_parser() -> impl Parser<Input, BinOp, Error = Error> {
        token::Plus::parser()
            .map(BinOp::Add)
            .or(token::Dash::parser().map(BinOp::Sub))
    }

    fn cmp_parser() -> impl Parser<Input, BinOp, Error = Error> {
        token::EqEq::parser()
            .map(BinOp::Eq)
            .or(token::LessEq::parser().map(BinOp::Le))
            .or(token::GreaterEq::parser().map(BinOp::Ge))
            .or(token::LessThan::parser().map(BinOp::Lt))
            .or(token::GreaterThan::parser().map(BinOp::Gt))
    }

    fn and_parser() -> impl Parser<Input, BinOp, Error = Error> {
        token::DoubleAnd::parser().map(BinOp::And)
    }

    fn or_parser() -> impl Parser<Input, BinOp, Error = Error> {
        token::DoublePipe::parser().map(BinOp::Or)
    }
}
