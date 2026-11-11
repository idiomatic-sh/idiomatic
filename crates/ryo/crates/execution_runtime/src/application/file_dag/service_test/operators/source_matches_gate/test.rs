//! Real compiled source/line gates precede the use of the matcher, which preparation compiled.
use super::super::fixtures::span;
use super::fixtures::run;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::constants::NO_SPAN;
/// The source and line gates refuse a malformed row before its matcher is used: its error is the gate's, not the
/// pattern's. The literal pattern is compiled once, eagerly, when the request is prepared (L-span L1), so every run
/// counts that one build, and a row that passes the gates meets the compiled refusal.
#[test]
fn malformed_source_and_line_refuse_before_the_prepared_regex() {
    for stream in [false, true] {
        assert_eq!(
            run(Some(span(0, 2)), 1, stream),
            (Err(OperatingError::InvalidSourceMatchSpan), 1)
        );
        assert_eq!(
            run(Some(span(0, 1)), 0, stream),
            (Err(OperatingError::InvalidSourceMatchSpan), 1)
        );
        assert_eq!(run(Some(NO_SPAN), 0, stream), (Ok(()), 1));
        assert_eq!(run(None, 1, stream), (Ok(()), 1));
        let (result, builds) = run(Some(span(0, 1)), 1, stream);
        assert!(matches!(result, Err(OperatingError::InvalidRegex { .. })));
        assert_eq!(builds, 1);
    }
}
