//! Clearing collected partial observations retains storage without retaining stale causes.
use super::Collect;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::{
    collections::relations::indent_region::item::IndentRegionRow,
    types::{FileId, Missing, Relation, ReportText, Rows},
};
#[derive(Clone, Copy, Debug)]
enum Operation {
    Partial,
    PartialAt,
}
type Input = Operation;
type Value = bool;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("partial causes reuse buffers and preserve fresh content"),
        word: Word::Succeeds,
        input: || Operation::Partial,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("file partials clear old identity and reuse buffers"),
        word: Word::Succeeds,
        input: || Operation::PartialAt,
        expect: Expect::Value(true),
    },
];
fn subject(operation: Input) -> Result<Value, Failure> {
    let mut output = Collect::default();
    match operation {
        Operation::Partial => {
            <Collect as Rows<IndentRegionRow>>::partial(
                &mut output,
                ReportText("old long cause"),
                Missing::Any,
            );
            let pointer = output.partials[0].1.as_ptr();
            output.clear();
            assert!(output.partials.is_empty());
            <Collect as Rows<IndentRegionRow>>::partial(
                &mut output,
                ReportText("new cause"),
                Missing::Any,
            );
            <Collect as Rows<IndentRegionRow>>::partial(
                &mut output,
                ReportText("new cause"),
                Missing::Any,
            );
            Ok(
                output.partials == [(Relation::IndentRegion, "new cause".into(), Missing::Any)]
                    && output.partials[0].1.as_ptr() == pointer,
            )
        }
        Operation::PartialAt => {
            <Collect as Rows<IndentRegionRow>>::partial_at(
                &mut output,
                FileId(3),
                ReportText("old long cause"),
                Missing::Any,
            );
            let pointer = output.partials_at[0].2.as_ptr();
            output.clear();
            assert!(output.partials_at.is_empty());
            <Collect as Rows<IndentRegionRow>>::partial_at(
                &mut output,
                FileId(5),
                ReportText("new cause"),
                Missing::Any,
            );
            Ok(
                output.partials_at == [(Relation::IndentRegion, FileId(5), "new cause".into())]
                    && output.partials_at[0].2.as_ptr() == pointer,
            )
        }
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
