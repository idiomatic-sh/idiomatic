//! Sparse static environment operations. Joins preserve both real alternatives and possible absence.











/// Preserve a known uncertainty without inventing a binding candidate.
#[cfg(test)]
pub(crate) fn uncertain(state: &mut BindingEnvironments, frame: BindingFrameId, key: BindingKey) {
    let value = lookup(state, frame, key)
        .cloned()
        .unwrap_or(BindingAlternatives {
            candidates: CandidateRange {
                start: CandidateIndex(state.candidates.len()),
                end: CandidateIndex(state.candidates.len()),
            },
            absence: BindingAbsence::Possible,
            knowledge: BindingKnowledge::Known,
        });
    store(
        state,
        frame,
        key,
        BindingAlternatives {
            knowledge: BindingKnowledge::Uncertain,
            ..value
        },
    );
}


