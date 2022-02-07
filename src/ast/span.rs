use std::ops;

pub struct Span {
    _source: ops::Range<usize>,
}

impl From<ops::Range<usize>> for Span {
    fn from(span: ops::Range<usize>) -> Self {
        Span { _source: span }
    }
}
