//! The decodes of one publication scope, shared by every root that admits the same words.
use super::super::types::{
    DecodeCount, DecodeShape, DecodedFrontier, FrontierInput, PackedFrontier,
};

/// The frontier decodes of one publication scope. Every rule of a cohort reads its own stored
/// bucket, but the buckets of one population hold the same words: the first root that admits them
/// decodes them, and every later root of the scope receives the same rows, relabelled with its own
/// input and coverage. Names are places in the symbol generation that decoded them, so a decode is
/// shared only within that generation.
pub(crate) struct FrontierDecodes {
    pub(in super::super) decoded: std::cell::RefCell<Vec<DecodedFrontier>>,
    pub(in super::super) count: std::cell::Cell<DecodeCount>,
    /// What names the content of the words: the request's digest port.
    pub(in super::super) digest: std::sync::Arc<dyn crate::outbound::digest::interface::DigestPort>,
}

impl FrontierDecodes {
    /// The decodes of one publication scope: `publication_owner` issues it into the scope, over the
    /// digest port the prepared program carries.
    pub(crate) fn new(
        digest: std::sync::Arc<dyn crate::outbound::digest::interface::DigestPort>,
    ) -> Self {
        Self {
            decoded: std::cell::RefCell::new(Vec::new()),
            count: std::cell::Cell::new(DecodeCount::default()),
            digest,
        }
    }
    #[cfg(test)]
    /// How many packed frontiers this scope decoded.
    pub(crate) fn count(&self) -> DecodeCount {
        self.count.get()
    }
    /// Admit one packed frontier: words this scope already decoded in the same shape, mode and
    /// symbol generation are not decoded again; the earlier rows are relabelled with this root's
    /// input and coverage, and an earlier refusal with this root's input. Only a miss calls `decode`.
    pub(in super::super) fn admit(
        &self,
        packed: PackedFrontier,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
        decode: impl FnOnce(
            PackedFrontier,
            &mut ryo_language::domain::symbols::types::Symbols,
        ) -> Result<
            FrontierInput,
            ryo_execution_domain::domain::frontiers::types::RefusedFrontier,
        >,
    ) -> Result<FrontierInput, ryo_execution_domain::domain::frontiers::types::RefusedFrontier>
    {
        use ryo_language::domain::symbols::{
            service::{same_transport_generation, transport_permit},
            types::Generation,
        };
        let permit = transport_permit(symbols);
        let shape = shape_of(packed.input);
        let content = self
            .digest
            .digest(crate::outbound::digest::types::DigestBytes(
                &packed.words.0.0,
            ));
        let mut decoded = self.decoded.borrow_mut();
        if let Some(found) = decoded.iter().find(|found| {
            found.shape == shape
                && found.mode == packed.mode
                && same_transport_generation(&found.permit, &permit) == Generation::Same
                && found.content == content
        }) {
            return relabel(&found.admitted, packed.input, packed.coverage);
        }
        let mode = packed.mode;
        let admitted = decode(packed, symbols);
        self.count.set(DecodeCount(self.count.get().0 + 1));
        decoded.push(DecodedFrontier {
            content,
            shape,
            mode,
            permit,
            admitted: admitted.clone(),
        });
        admitted
    }
}

/// What the words of a packed frontier of `input` decode to.
fn shape_of(input: ryo_values::domain::plan_parts::types::Input) -> DecodeShape {
    if matches!(input, ryo_values::domain::plan_parts::types::Input::Step(_)) {
        DecodeShape::Compact
    } else {
        DecodeShape::Decoded
    }
}

/// An earlier decode as this root admits it: the shared rows under this root's input and coverage.
fn relabel(
    admitted: &Result<
        FrontierInput,
        ryo_execution_domain::domain::frontiers::types::RefusedFrontier,
    >,
    input: ryo_values::domain::plan_parts::types::Input,
    coverage: ryo_execution_domain::domain::completion::types::CoverageState,
) -> Result<FrontierInput, ryo_execution_domain::domain::frontiers::types::RefusedFrontier> {
    match admitted.clone() {
        Ok(FrontierInput::Decoded { frontier, .. }) => Ok(FrontierInput::Decoded {
            frontier: ryo_execution_domain::domain::table_model::types::Frontier {
                input,
                table: frontier.table,
            },
            coverage,
        }),
        Ok(FrontierInput::Compact {
            table, spelling, ..
        }) => Ok(FrontierInput::Compact {
            input,
            table,
            spelling,
            coverage,
        }),
        Ok(FrontierInput::Packed(packed)) => Ok(FrontierInput::Packed(PackedFrontier {
            input,
            coverage,
            ..packed
        })),
        Err(refused) => Err(
            ryo_execution_domain::domain::frontiers::types::RefusedFrontier {
                input,
                error: refused.error,
            },
        ),
    }
}
