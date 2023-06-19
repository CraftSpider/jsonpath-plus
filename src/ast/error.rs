use super::Span;

use std::collections::BTreeSet;

/// The cause of a parse failure
#[derive(Debug, PartialEq)]
pub enum FailReason<I> {
    /// An unexpected token at a span
    Unexpected {
        /// The set of expected tokens
        expected: BTreeSet<Option<I>>,
        /// The found token
        found: Option<I>,
    },
    /// An unclosed delimiter was encountered
    Unclosed {
        /// The span of the unclosed starting token
        unclosed_span: Span,
        /// The expected tokens
        expected: BTreeSet<Option<I>>,
        /// The found token
        found: Option<I>,
        /// The expected ending delimiter
        delimiter: I,
    },
    /// A custom message and span
    Custom(String),
    /// Multiple reasons merged together
    MultiReason(Vec<FailReason<I>>),
}

impl<I> FailReason<I> {
    fn merge(self, other: FailReason<I>) -> FailReason<I>
    where
        I: Ord,
    {
        match (self, other) {
            (
                FailReason::Unexpected { expected: e1, found },
                FailReason::Unexpected { expected: e2, found: _ }
            ) => {
                FailReason::Unexpected { expected: e1.into_iter().chain(e2.into_iter()).collect(), found }
            },
            (
                FailReason::MultiReason(mut multi1),
                FailReason::MultiReason(multi2),
            ) => {
                multi1.extend(multi2);
                FailReason::MultiReason(multi1)
            },
            (
                FailReason::MultiReason(mut multi),
                other,
            ) => {
                multi.push(other);
                FailReason::MultiReason(multi)
            },
            (
                other,
                FailReason::MultiReason(mut multi),
            ) => {
                multi.push(other);
                FailReason::MultiReason(multi)
            },
            (this, other) => FailReason::MultiReason(vec![this, other]),
        }
    }
}

/// A single parse failure error
#[derive(Debug)]
pub struct ParseFail<I: Ord, L> {
    span: Span,
    reason: FailReason<I>,
    label: Option<L>,
}

impl<I: Ord, L> ParseFail<I, L> {
    /// Create a custom parse failure
    pub(crate) fn custom(span: Span, message: &str) -> ParseFail<I, L> {
        ParseFail {
            span,
            reason: FailReason::Custom(message.to_string()),
            label: None,
        }
    }

    /// Get the reason of this parse failure
    pub fn reason(&self) -> &FailReason<I> {
        &self.reason
    }
}

impl<I: Ord, L> chumsky::Error<I> for ParseFail<I, L> {
    type Span = Span;
    type Label = L;

    fn expected_input_found<Iter: IntoIterator<Item = Option<I>>>(
        span: Self::Span,
        expected: Iter,
        found: Option<I>,
    ) -> Self {
        ParseFail {
            span,
            reason: FailReason::Unexpected {
                expected: expected.into_iter().collect(),
                found,
            },
            label: None,
        }
    }

    fn unclosed_delimiter(
        unclosed_span: Self::Span,
        unclosed: I,
        span: Self::Span,
        expected: I,
        found: Option<I>,
    ) -> Self {
        ParseFail {
            span,
            reason: FailReason::Unclosed {
                delimiter: unclosed,
                expected: BTreeSet::from([Some(expected)]),
                found,
                unclosed_span,
            },
            label: None,
        }
    }

    fn with_label(mut self, label: Self::Label) -> Self {
        self.label = Some(label);
        self
    }

    fn merge(self, other: Self) -> Self {
        let reason = self.reason.merge(other.reason);
        ParseFail {
            span: self.span,
            reason,
            label: self.label.or(other.label),
        }
    }
}
