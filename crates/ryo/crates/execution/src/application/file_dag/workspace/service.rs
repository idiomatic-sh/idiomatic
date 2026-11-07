//! The invocation workspace admits an operator's inputs and acquires the derived view they demand.
use super::constants::{EXPECT_ADMITTED_INPUT, EXPECT_FIRST_INPUT};
use super::{InputAdmissions, InvocationWorkspace, ViewInputs};
use crate::application::file_dag::derived::owner::acquisition::population::types::StoredColumns;
use crate::application::file_dag::relations::admission::service::BorrowedInput;
use crate::domain::derived_views::types::contract::value::PreparedViewUse;
use crate::domain::operating::errors::OperatingError;

use crate::domain::table_model::types::RepresentationInput as BorrowedRows;
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::rows::types::Table;

impl InvocationWorkspace {
    pub(in crate::application::file_dag) fn inputs<'a, 'f>(
        &'a self,
        inputs: &'a [BorrowedRows<'f>],
    ) -> InputAdmissions<'a, 'f> {
        InputAdmissions {
            workspace: self,
            inputs,
            proofs: std::array::from_fn(|_| std::cell::OnceCell::new()),
        }
    }
    pub(super) fn admit<'a>(
        &self,
        place: InputNumber,
        rows: BorrowedRows<'a>,
    ) -> Result<BorrowedInput<'a>, OperatingError> {
        let owner = self.owner();
        if let Some(record) = self.publications.get(place.0).and_then(Option::as_ref) {
            return owner.admit(record.as_ref(), rows);
        }
        if place.0 >= self.publications.len() {
            Err(OperatingError::MissingInput {
                position: place.0 + 1,
            })
        } else {
            Err(OperatingError::NotRun)
        }
    }

    pub(super) fn acquire_admitted(
        &self,
        use_: &PreparedViewUse,
        inputs: &InputAdmissions<'_, '_>,
        identity: crate::application::file_dag::derived::owner::ViewContext,
        supplied: ViewInputs<'_, '_>,
    ) -> Result<crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView, OperatingError>
    {
        use crate::domain::{
            derived_views::types::{DerivedInputs, MembershipSpace},
            operating::errors::OperatingError,
        };
        use ryo_derived::domain::request::types::DerivedKind;
        let count = usize::from(use_.len.0);
        if !(1..=2).contains(&count) {
            return Err(OperatingError::WrongOperation);
        }
        if count < 2
            && matches!(
                &supplied,
                ViewInputs::MembershipNative | ViewInputs::Dfs { .. }
            )
        {
            return Err(OperatingError::MissingInput { position: 2 });
        }
        // The prepared Membership slot also declares the native tree space.
        // Keep admission/error order for all demanded operands, while the value
        // view's identity and row proof contain only its actual semantic source.
        let semantic_sources = if matches!(&supplied, ViewInputs::MembershipValues) {
            1
        } else {
            count
        };
        let mut proofs: [Option<&BorrowedInput<'_>>; 2] = [None, None];
        let mut relations = [None; 2];
        for at in 0..count {
            let proof = inputs.proof(use_.inputs[at])?;
            let relation = proof.identity(self.owner())?.relation;
            relations[at] = relation;
            proofs[at] = Some(proof);
        }
        let rows = [
            proofs[0].as_ref().expect(EXPECT_FIRST_INPUT).rows(),
            proofs[1]
                .as_ref()
                .map_or(BorrowedRows::Computed(&EMPTY_DERIVED_INPUT), |proof| {
                    proof.rows()
                }),
        ];
        let supplied = match supplied {
            ViewInputs::Rows => DerivedInputs::Rows(&rows[..count]),
            ViewInputs::Text(texts) => DerivedInputs::Text {
                rows: &rows[..count],
                texts,
            },
            ViewInputs::MembershipValues => DerivedInputs::Membership {
                rows: rows[0],
                space: MembershipSpace::Values,
            },
            ViewInputs::MembershipNative => {
                let BorrowedRows::Tree(tree) = rows[1] else {
                    return Err(OperatingError::WrongOperation);
                };
                DerivedInputs::Membership {
                    rows: rows[0],
                    space: MembershipSpace::Native(tree),
                }
            }
            ViewInputs::Dfs {
                symbols,
                source,
                paths,
                arenas,
            } => DerivedInputs::Dfs {
                roots: rows[0],
                edges: rows[1],
                symbols,
                source,
                paths,
                arenas,
            },
        };
        let stored = if use_.plan.kind == DerivedKind::Equality && count == 1 {
            match rows[0] {
                BorrowedRows::Stored(source) => relations[0]
                    .or(use_.relations[0])
                    .map(|relation| StoredColumns {
                        relation,
                    }),
                _ => None,
            }
        } else {
            None
        };
        let owner = self.owner().derived();
        let sources = proofs[..semantic_sources]
            .iter()
            .map(|proof| proof.expect(EXPECT_ADMITTED_INPUT))
            .collect::<Vec<_>>();
        let admitted = owner.admit_view(&use_.plan, self.owner(), &sources)?;
        owner.acquire_stored(
            &use_.plan,
            &admitted,
            identity,
            supplied,
            &mut self.derived_scratch.borrow_mut(),
            stored,
        )
    }
}
static EMPTY_DERIVED_INPUT: Table = Table {
    kinds: Vec::new(),
    cells: Vec::new(),
    count: ryo_values::domain::rows::types::RowCount(0),
};
