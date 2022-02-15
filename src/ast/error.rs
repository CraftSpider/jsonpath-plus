use super::Span;

use std::collections::BTreeSet;

/// The cause of a parse failure
#[derive(Debug, PartialEq)]
pub enum FailReason<I> {
    /// An unexpected token at a span
    Unexpected(Span),
    /// An unclosed delimiter was encountered
    Unclosed {
        /// The span of the found token
        found_span: Span,
        /// The span of the unclosed starting token
        unclosed_span: Span,
        /// The expected ending delimiter
        delimiter: I,
    },
    /// A custom message and span
    Custom(Span, String),
    /// Multiple reasons merged together
    MultiReason(Vec<FailReason<I>>),
}

impl<I> FailReason<I> {
    fn into_vec(self) -> Vec<FailReason<I>> {
        match self {
            FailReason::MultiReason(v) => v,
            _ => vec![self],
        }
    }
}

/// A single parse failure error
#[derive(Debug)]
pub struct ParseFail<I: Ord, L> {
    reason: FailReason<I>,
    expected: BTreeSet<Option<I>>,
    found: Option<I>,
    label: Option<L>,
}

impl<I: Ord, L> ParseFail<I, L> {
    /// Create a custom parse failure
    pub(crate) fn custom(span: Span, message: &str) -> ParseFail<I, L> {
        ParseFail {
            reason: FailReason::Custom(span, message.to_string()),
            expected: BTreeSet::new(),
            found: None,
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
            reason: FailReason::Unexpected(span),
            expected: expected.into_iter().collect(),
            found,
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
            reason: FailReason::Unclosed {
                delimiter: unclosed,
                found_span: span,
                unclosed_span,
            },
            expected: BTreeSet::from([Some(expected)]),
            found,
            label: None,
        }
    }

    fn with_label(mut self, label: Self::Label) -> Self {
        self.label = Some(label);
        self
    }

    fn merge(self, other: Self) -> Self {
        let mut reason = self.reason.into_vec();
        reason.extend(other.reason.into_vec());
        reason.dedup();
        let reason = if reason.len() == 1 {
            reason.remove(0)
        } else {
            FailReason::MultiReason(reason)
        };

        let mut expected = self.expected;
        expected.extend(other.expected);

        ParseFail {
            reason,
            expected,
            found: self.found.or(other.found),
            label: self.label.or(other.label),
        }
    }
}
