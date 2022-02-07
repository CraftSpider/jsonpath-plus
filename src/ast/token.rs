use super::{Error, Input, Span};
use chumsky::primitive::just;
use chumsky::Parser;

// 'Wrapping' tokens

macro_rules! wrapping_tokens {
        ($($name:ident($start:literal, $end:literal));* $(;)?) => {
            $(
            pub struct $name(Span, Span);

            impl $name {
                pub(super) fn parser<T>(item: impl Parser<Input, T, Error=Error>) -> impl Parser<Input, (Self, T), Error=Error> {
                    item.delimited_by($start, $end)
                        .map_with_span(|inner, span| {
                            let start = span.start..(span.start + 1);
                            let end = (span.end - 1)..span.end;

                            ($name(start.into(), end.into()), inner)
                        })
                }
            }
            )*
        }
    }

wrapping_tokens! {
    Bracket('[', ']');
    Paren('(', ')');
    // Brace('{', '}');
}

// Simple tokens

macro_rules! simple_tokens {
        ($($name:ident($just:literal));* $(;)?) => {
            $(
            pub struct $name(Span);

            impl $name {
                pub(super) fn parser() -> impl Parser<Input, Self, Error=Error> {
                    just::<_, _, Error>($just)
                        .map_with_span(|_, span| $name(span.into()))
                }
            }
            )*
        }
    }

simple_tokens! {
    At('@');
    Bang('!');
    Caret('^');
    Colon(':');
    Dash('-');
    Dollar('$');
    Dot('.');
    DotDot("..");
    DoubleAnd("&&");
    DoublePipe("||");
    DoubleQuote('"');
    EqEq("==");
    GreaterEq(">=");
    GreaterThan('>');
    // LeftSlash('\\');
    LessEq("<=");
    LessThan('<');
    Percent('%');
    Plus('+');
    Question('?');
    RightSlash('/');
    SingleQuote('\'');
    Star('*');
    Tilde('~');
}
