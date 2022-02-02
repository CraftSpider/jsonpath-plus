
use super::*;
use chumsky::prelude::*;

impl Ident {
    fn parser() -> impl Parser<Input, Ident, Error = Error> {
        filter::<_, _, Error>(|c: &char| c.is_alphanumeric() || *c == '-' || *c == '_')
            .repeated()
            .map_with_span(|val, span| Ident {
                span: span.into(),
                val: String::from_iter(val),
            })
    }
}

impl IntLit {
    fn parser() -> impl Parser<Input, IntLit, Error = Error> {
        just::<_, _, Error>('-')
            .or_not()
            .then(filter(|c: &char| c.is_numeric()).repeated())
            .map_with_span(|(neg, val), span| IntLit {
                span: span.into(),
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
                _span: il.span,
                val: il
                    .val
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
            .map_with_span(|content, span| StringContent {
                _span: span.into(),
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
                _start: start,
                content,
                _end: end,
            })
    }
}

impl DoubleStringLit {
    fn parser() -> impl Parser<Input, DoubleStringLit, Error = Error> {
        token::DoubleQuote::parser()
            .then(StringContent::parser('"'))
            .then(token::DoubleQuote::parser())
            .map(|((start, content), end)| DoubleStringLit {
                _start: start,
                content,
                _end: end,
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
    fn parser() -> impl Parser<Input, BoolLit, Error=Error> {
        just::<_, _, Error>("true").to(true)
            .or(just("false").to(false))
            .map_with_span(|val, span| BoolLit {
                _span: span.into(),
                val
            })
    }
}

impl NullLit {
    fn parser() -> impl Parser<Input, NullLit, Error=Error> {
        just::<_, _, Error>("null")
            .map_with_span(|_, span| NullLit { _span: span.into() })
    }
}

impl Path {
    pub(crate) fn parser() -> impl Parser<Input, Path, Error=Error> {
        token::Dollar::parser()
            .then(Operator::parser().repeated())
            .then_ignore(end())
            .map(|(dollar, children)| Path { _dollar: dollar, children })
    }
}

impl SubPath {
    fn parser(operator: impl Parser<Input, Operator, Error = Error>) -> impl Parser<Input, SubPath, Error = Error> {
        PathKind::parser()
            .then(operator.repeated())
            .map(|(kind, children)| SubPath { kind, children })
    }
}

impl PathKind {
    fn parser() -> impl Parser<Input, PathKind, Error = Error> {
        token::Dollar::parser()
            .map(PathKind::Root)
            .or(token::At::parser().map(PathKind::Relative))
    }
}

impl Operator {
    fn parser() -> impl Parser<Input, Operator, Error = Error> {
        recursive(|operator| {
            token::DotDot::parser()
                .then(RecursiveOp::parser(operator.clone()))
                .map(|(dotdot, inner)| Operator::Recursive(dotdot, inner))
                .or(token::Bracket::parser(BracketInner::parser(operator))
                    .map(|(brack, inner)| Operator::Bracket(brack, inner)))
                .or(token::Dot::parser()
                    .then(DotIdent::parser())
                    .map(|(dot, ident)| Operator::Dot(dot, ident)))
        })
    }
}

impl RecursiveOp {
    fn parser(operator: impl Parser<Input, Operator, Error = Error> + Clone + 'static) -> impl Parser<Input, RecursiveOp, Error=Error> {
        DotIdent::parser()
            .map(RecursiveOp::Raw)
            .or(token::Bracket::parser(BracketInner::parser(operator))
                .map(|(bracket, inner)| RecursiveOp::Bracket(bracket, inner)))
    }
}

impl DotIdent {
    fn parser() -> impl Parser<Input, DotIdent, Error=Error> {
        token::Star::parser()
            .map(DotIdent::Wildcard)
            .or(token::Caret::parser().map(DotIdent::Parent))
            .or(Ident::parser().map(DotIdent::Name))
    }
}

impl BracketInner {
    fn parser(operator: impl Parser<Input, Operator, Error = Error> + Clone + 'static) -> impl Parser<Input, BracketInner, Error = Error> {
        let steprange = IntLit::parser()
            .or_not()
            .then(token::Colon::parser())
            .then(IntLit::parser().or_not())
            .then(token::Colon::parser())
            .then(NonZeroIntLit::parser().or_not())
            .map(|((((start, colon1), end), colon2), step)| {
                BracketInner::StepRange(start, colon1, end, colon2, step)
            });

        let range = IntLit::parser()
            .or_not()
            .then(token::Colon::parser())
            .then(IntLit::parser().or_not())
            .map(|((start, colon1), end)| BracketInner::Range(start, colon1, end));

        steprange
            .or(range)
            .or(token::Star::parser().map(BracketInner::Wildcard))
            .or(token::Caret::parser().map(BracketInner::Parent))
            .or(SubPath::parser(operator.clone()).map(BracketInner::Path))
            .or(Filter::parser(operator).map(BracketInner::Filter))
            .or(BracketLit::parser().map(BracketInner::Literal))
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
    fn parser(operator: impl Parser<Input, Operator, Error = Error> + Clone + 'static) -> impl Parser<Input, Filter, Error = Error> {
        token::Question::parser()
            .then(token::Paren::parser(FilterExpr::parser(operator)))
            .map(|(question, (paren, inner))| Filter {
                _question: question,
                _paren: paren,
                inner,
            })
    }
}

impl ExprLit {
    fn parser() -> impl Parser<Input, ExprLit, Error = Error> {
        IntLit::parser()
            .map(ExprLit::Int)
            .or(StringLit::parser().map(ExprLit::Str))
            .or(BoolLit::parser().map(ExprLit::Bool))
            .or(NullLit::parser().map(ExprLit::Null))
    }
}

impl FilterExpr {
    fn parser(operator: impl Parser<Input, Operator, Error = Error> + Clone + 'static) -> impl Parser<Input, FilterExpr, Error = Error> {
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
    pub fn product_parser() -> impl Parser<Input, BinOp, Error = Error> {
        token::Star::parser()
            .map(BinOp::Mul)
            .or(token::RightSlash::parser().map(BinOp::Div))
            .or(token::Percent::parser().map(BinOp::Rem))
    }

    pub fn sum_parser() -> impl Parser<Input, BinOp, Error = Error> {
        token::Plus::parser()
            .map(BinOp::Add)
            .or(token::Dash::parser().map(BinOp::Sub))
    }

    pub fn cmp_parser() -> impl Parser<Input, BinOp, Error = Error> {
        token::EqEq::parser()
            .map(BinOp::Eq)
            .or(token::LessEq::parser().map(BinOp::Le))
            .or(token::GreaterEq::parser().map(BinOp::Ge))
            .or(token::LessThan::parser().map(BinOp::Lt))
            .or(token::GreaterThan::parser().map(BinOp::Gt))
    }

    pub fn and_parser() -> impl Parser<Input, BinOp, Error = Error> {
        token::DoubleAnd::parser().map(BinOp::And)
    }

    pub fn or_parser() -> impl Parser<Input, BinOp, Error = Error> {
        token::DoublePipe::parser().map(BinOp::Or)
    }
}
