use super::*;

#[test]
#[cfg(feature = "spanned")]
fn test_span_multibyte_string() {
    let path_str = "$['ඞ']";
    let path = Path::compile(path_str).unwrap();

    let lit = if let Segment::Bracket(_, BracketSelector::Literal(BracketLit::String(lit))) =
        &path.segments()[0]
    {
        lit
    } else {
        panic!("First segment wasn't a literal")
    };

    let lit_span = lit.span();
    assert_eq!(lit_span.get_span(path_str), "'ඞ'");
}

#[test]
#[cfg(feature = "spanned")]
fn test_filter_span() {
    let path_str = "$[?(@ == true)]";
    let path = Path::compile(path_str).unwrap();

    let filter = if let Segment::Bracket(_, BracketSelector::Filter(filter)) = &path.segments()[0] {
        filter
    } else {
        panic!("First segment wasn't a filter")
    };

    let filter_span = filter.span();
    assert_eq!(filter_span.get_span(path_str), "?(@ == true)");

    let filter_expr_span = filter.expression().span();
    assert_eq!(filter_expr_span.get_span(path_str), "@ == true");
}
