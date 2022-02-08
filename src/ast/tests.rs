
use super::*;

#[test]
#[cfg(feature = "spanned")]
fn test_filter_span() {
    let path_str = "$[?(@ == true)]";
    let path = Path::compile(path_str)
        .unwrap();

    let filter = if let Segment::Bracket(_, BracketSelector::Filter(filter)) = &path.segments()[0] {
        filter
    } else {
        panic!("First segment wasn't a filter")
    };

    let filter_span = filter
        .span();
    assert_eq!(filter_span.slice(path_str), "?(@ == true)");

    let filter_expr_span = filter
        .expression()
        .span();
    assert_eq!(filter_expr_span.slice(path_str), "@ == true");
}
