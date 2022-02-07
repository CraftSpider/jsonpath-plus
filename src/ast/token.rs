use super::{Error, Input};
use chumsky::primitive::just;
use chumsky::Parser;

#[cfg(feature = "spanned")]
use super::Span;

// 'Wrapping' tokens

macro_rules! wrapping_tokens {
        ($($name:ident($start:literal, $end:literal));* $(;)?) => {
            $(
            #[cfg(feature = "spanned")]
            pub struct $name(Span, Span);
            #[cfg(not(feature = "spanned"))]
            pub struct $name(());

            impl $name {
                #[cfg(feature = "spanned")]
                pub(super) fn parser<T>(item: impl Parser<Input, T, Error = Error>) -> impl Parser<Input, (Self, T), Error = Error> {
                    item.delimited_by(just($start), just($end))
                        .map_with_span(|inner, span| {
                            let start = span.start..(span.start + 1);
                            let end = (span.end - 1)..span.end;

                            ($name(start.into(), end.into()), inner)
                        })
                }

                #[cfg(not(feature = "spanned"))]
                pub(super) fn parser<T>(item: impl Parser<Input, T, Error = Error>) -> impl Parser<Input, (Self, T), Error = Error> {
                    item.delimited_by(just($start), just($end))
                        .map(|inner| {
                            ($name(()), inner)
                        })
                }
            }

            #[cfg(feature = "spanned")]
            impl crate::ast::Spanned for $name {
                fn span(&self) -> crate::ast::Span {
                    self.0.join(self.1)
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
            #[cfg(feature = "spanned")]
            pub struct $name(Span);
            #[cfg(not(feature = "spanned"))]
            pub struct $name(());

            impl $name {
                #[cfg(feature = "spanned")]
                pub(super) fn parser() -> impl Parser<Input, Self, Error = Error> {
                    just::<_, _, Error>($just)
                        .map_with_span(|_, span| $name(span.into()))
                }

                #[cfg(not(feature = "spanned"))]
                pub(super) fn parser() -> impl Parser<Input, Self, Error = Error> {
                    just::<_, _, Error>($just).map(|_| $name(()))
                }
            }

            #[cfg(feature = "spanned")]
            impl crate::ast::Spanned for $name {
                fn span(&self) -> crate::ast::Span {
                    self.0
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
