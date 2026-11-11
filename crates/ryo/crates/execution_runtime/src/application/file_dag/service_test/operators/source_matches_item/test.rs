//! Physical occurrence spans and capture rows through actual compiled execution.
use super::super::fixtures::span;
use super::fixtures::{captured, evaluate, evaluate_lines};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::constants::NO_SPAN;
#[test]
fn nonzero_offset_utf8_and_repeated_source_text_keep_exact_order_and_bag() {
    assert_eq!(
        evaluate("Zéé!".as_bytes(), &[span(1, 5), span(1, 5)], "é"),
        Ok(vec![span(1, 3), span(3, 5), span(1, 3), span(3, 5)])
    );
    assert_eq!(evaluate(b"abc", &[span(0, 3)], "z"), Ok(vec![]));
}

#[test]
fn zero_width_and_absent_spans_are_explicit() {
    assert_eq!(
        evaluate("éx".as_bytes(), &[span(0, 3)], ""),
        Ok(vec![span(0, 0), span(2, 2), span(3, 3)])
    );
    assert_eq!(
        evaluate(b"abc", &[span(1, 1), NO_SPAN], ""),
        Ok(vec![span(1, 1)])
    );
}

#[test]
fn malformed_or_non_utf8_boundary_spans_refuse_before_their_row_emits() {
    for (source, invalid) in [
        (&b"abc"[..], span(2, 1)),
        (&b"abc"[..], span(0, 4)),
        ("Zé!".as_bytes(), span(2, 3)),
    ] {
        assert_eq!(
            evaluate(source, &[invalid], "é"),
            Err(OperatingError::InvalidSourceMatchSpan)
        );
    }
    assert_eq!(
        evaluate(b"a\xffb", &[span(0, 3)], "a"),
        Err(OperatingError::InvalidSourceMatchSpan),
        "a whole-file match must never invent offsets in lossy-decoded source",
    );
}

#[test]
fn optional_line_uses_match_start_and_counts_crlf_and_single_endings_once() {
    assert_eq!(
        evaluate_lines(b"a\r\nb\rc\nd", span(0, 8), 1, "\nb|b\\rc|\nd|c"),
        Ok(vec![(span(2, 4), 1), (span(5, 6), 3), (span(6, 8), 3)])
    );
    assert_eq!(
        evaluate_lines(b"a\r\nb", span(0, 4), 1, ""),
        Ok(vec![
            (span(0, 0), 1),
            (span(1, 1), 1),
            (span(2, 2), 1),
            (span(3, 3), 2),
            (span(4, 4), 2)
        ])
    );
}

#[test]
fn optional_line_rejects_zero_and_overflow() {
    assert_eq!(
        evaluate_lines(b"x", span(0, 1), 0, "x"),
        Err(OperatingError::InvalidSourceMatchSpan)
    );
    assert_eq!(
        evaluate_lines(b"a\nb", span(0, 3), i64::MAX, "b"),
        Err(OperatingError::InvalidSourceMatchSpan)
    );
}

#[test]
fn named_capture_preserves_offsets_order_bag_and_skips_unmatched_branches() {
    assert_eq!(
        captured(
            "é\\q\\z".as_bytes(),
            &[span(2, 6), span(2, 6)],
            r"(?P<piece>\\.)"
        ),
        Ok(vec![
            (span(2, 4), span(2, 4), "\\q".into()),
            (span(4, 6), span(4, 6), "\\z".into()),
            (span(2, 4), span(2, 4), "\\q".into()),
            (span(4, 6), span(4, 6), "\\z".into()),
        ])
    );
    assert_eq!(
        captured(b"\\qz", &[span(0, 3)], r"(?P<piece>\\q)|z"),
        Ok(vec![(span(0, 2), span(0, 2), "\\q".into())])
    );
    assert_eq!(
        captured("é".as_bytes(), &[span(0, 2)], r"(?P<piece>)"),
        Ok(vec![
            (span(0, 0), span(0, 0), "".into()),
            (span(2, 2), span(2, 2), "".into())
        ])
    );
}
