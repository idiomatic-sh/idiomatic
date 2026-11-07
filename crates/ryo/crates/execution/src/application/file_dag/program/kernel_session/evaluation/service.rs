//! Registered-operator evaluation of a kernel session.
use crate::application::file_dag::program::runtime::service::KernelPermit;
use crate::application::file_dag::program::runtime::types::InvocationFailure;
use crate::application::file_dag::program::runtime::types::KernelInput;
use crate::application::file_dag::program::runtime::types::KernelScratch;
use crate::{
    application::file_dag::{invocation::InvocationResources, workspace::InvocationWorkspace},
    domain::{
        executable_program::types::Instruction,
        operating::{
            errors::OperatingError,
            types::{Emit, InputPlace, OperatorContext},
        },
        operator::contract::{
            types::{InputShape, PreparedInputs},
            views::BorrowedRows,
        },
        table_model::{
            service::{count_of, read_row},
            types::Rows,
        },
    },
};
use ryo_values::domain::rows::types::RowIndex;
use std::rc::Rc;

impl super::super::KernelSession<'_, '_> {
    pub(in crate::application::file_dag::program::kernel_session) fn evaluate_session(
        &self,
        instruction: &Instruction,
        permit: &KernelPermit<'_>,
        input: KernelInput<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
        output: &mut dyn Emit,
    ) -> Result<(), InvocationFailure> {
        self.walk
            .validate(permit)
            .map_err(InvocationFailure::Semantic)?;
        let mut rebound = OperatorContext {
            phase: context.phase,
            semantic: context.semantic,
            prepared_matchers: context.prepared_matchers,
            prepared_kinds: context.prepared_kinds,
            bindings: self
                .prepared
                .bindings(instruction.producer)
                .map_err(InvocationFailure::Semantic)?,
            symbols: ryo_language::domain::symbols::service::query_reborrow(&mut context.symbols),
            source: context.source,
            paths: context.paths,
            arenas: context.arenas,
            memo: &mut *context.memo,
            scratch: &mut *context.scratch,
        };
        let context = &mut rebound;
        let mut invocation_workspace = scratch.workspace.take().unwrap_or_else(|| {
            InvocationWorkspace::for_contract(
                Rc::clone(&self.owner),
                permit.descriptor().contract.as_ref(),
            )
        });
        let workspace = &mut invocation_workspace;
        let contract = instruction.contract.as_ref();
        let setup_failure = std::sync::OnceLock::new();
        let setup_semantics = crate::application::file_dag::execution::PrimitiveSemanticsBridge {
            port: self.kernels,
            permit: permit.semantics(),
            failure: &setup_failure,
            binding: crate::domain::executable_program::physical::PhysicalBinding {
                bindings: context.bindings,
                regex_identity: None,
                native_join: false,
                source_len: context.source.0.len(),
            },
        };
        let result = (|| {
            match input {
                KernelInput::Continuation(row) => {
                    workspace.work.found(0, 1);
                    let mut resources = crate::application::file_dag::invocation::service::continuation(contract)
                        .map_err(InvocationFailure::Semantic)?;
                    crate::application::file_dag::execution::lease::evaluate(
                        self.kernels,
                        contract,
                        permit,
                        &instruction.operation,
                        PreparedInputs::Row {
                            input: InputPlace(0),
                            sides: &[],
                            row,
                        },
                        &mut resources,
                        workspace,
                        context,
                        output,
                    )
                    .map_err(InvocationFailure::Semantic)
                }
                KernelInput::Admitted(admitted) => {
                    let publications = admitted
                        .iter()
                        .map(|input| input.publication(&self.owner).cloned())
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(InvocationFailure::Semantic)?;
                    let originals = admitted
                        .iter()
                        .map(|input| input.rows(&self.owner, self.read))
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(InvocationFailure::Semantic)?;
                    if let [source] = originals.as_slice() {
                        if let Some(shared) = self
                            .walk
                            .acquire(
                                self.prepared,
                                permit,
                                source,
                                crate::application::file_dag::program::native_walk::WalkDelivery::Rows,
                                context,
                                self.kernels,
                            )
                            .map_err(InvocationFailure::Semantic)?
                        {
                            use crate::domain::replay::types::SelectionColumns;
                            use ryo_values::domain::{
                                plan_parts::types::RepeatedRows, plans::types::Operation,
                            };
                            let kept = match &instruction.operation {
                                Operation::Select { kept, .. } if kept.is_empty() => {
                                    SelectionColumns::All
                                }
                                Operation::Select { kept, .. } => SelectionColumns::Columns(kept),
                                Operation::Project {
                                    columns,
                                    distinct: RepeatedRows::Kept,
                                    ..
                                } => SelectionColumns::Columns(columns),
                                _ => {
                                    return Err(InvocationFailure::Semantic(
                                        OperatingError::WrongOperation,
                                    ));
                                }
                            };
                            workspace.work.found(0, shared.visits().0);
                            return crate::domain::replay::service::selected(
                                shared.result(),
                                source,
                                kept,
                                context,
                                output,
                            )
                            .map_err(InvocationFailure::Semantic);
                        }
                    }
                    let views: Vec<_> = originals.iter().map(BorrowedRows::from).collect();
                    if contract.semantics.preflight
                        == crate::domain::operator::contract::types::PreflightOrder::SchemaFirst
                    {
                        contract
                            .output_kinds
                            .as_ref()
                            .map_err(|error| InvocationFailure::Semantic(error.clone()))?;
                    }

                    workspace.capture_admitted(publications.clone());
                    InvocationResources::validate_row_metadata(contract, &views)
                        .map_err(InvocationFailure::Semantic)?;
                    let meaning = &contract.semantics;
                    let mut resources = if matches!(meaning.shape, InputShape::Row { .. })
                    && meaning.acquisition
                        == crate::domain::operator::contract::types::AcquisitionBoundary::FirstRow
                {
                    None
                } else {
                    Some(
                        crate::application::file_dag::invocation::service::prepare(
                            contract, &views, context, &originals, workspace, &setup_semantics,
                        )
                        .map_err(InvocationFailure::Semantic)?,
                    )
                };
                    {
                        contract
                            .output_kinds
                            .as_ref()
                            .map_err(|error| InvocationFailure::Semantic(error.clone()))?;
                    }
                    if matches!(
                        permit.native_join_eligibility(),
                        crate::application::file_dag::program::runtime::types::NativeJoinEligibility::Native
                    ) && originals.first().is_some_and(|left| count_of(left).0 != 0)
                        && matches!(originals.get(1), Some(Rows::Tree(_)))
                    {
                        if resources.is_none() {
                            resources = Some(
                                crate::application::file_dag::invocation::service::prepare(
                                    contract,
                                    &views,
                                    context,
                                    &originals,
                                    workspace,
                                    &setup_semantics,
                                )
                                .map_err(InvocationFailure::Semantic)?,
                            );
                        }
                        if let Some(native) = resources
                            .as_ref()
                            .ok_or(InvocationFailure::Semantic(OperatingError::NotRun))?
                            .native_join_lease()
                            .map_err(InvocationFailure::Semantic)?
                        {
                            let delivery = std::sync::OnceLock::new();
                            let request =
                                crate::application::file_dag::program::native_join::lease::service::PermittedNativeJoin::new(
                                    permit,
                                    native,
                                    &originals[0],
                                    &self.owner,
                                    context,
                                    &workspace.work,
                                    output,
                                    &delivery,
                                )
                                .map_err(InvocationFailure::Semantic)?;
                            let result = self.kernels.execute(
                                crate::application::file_dag::executor::KernelCall::NativeJoin {
                                    request,
                                },
                            );
                            if let Some(error) = delivery.get() {
                                return Err(InvocationFailure::Delivery(error.clone()));
                            }
                            return match result {
                                Ok(
                                    crate::application::file_dag::executor::KernelAnswer::Complete,
                                ) => Ok(()),
                                Ok(_) => {
                                    Err(InvocationFailure::Semantic(OperatingError::WrongOperation))
                                }
                                Err(
                                    crate::application::file_dag::executor::KernelFault::Refused(
                                        error,
                                    ),
                                ) => Err(InvocationFailure::Semantic(error)),
                                Err(
                                    crate::application::file_dag::executor::KernelFault::Portable(
                                        error,
                                    ),
                                ) => Err(InvocationFailure::Semantic(
                                    crate::domain::operator::contract::types::portable_fault(error),
                                )),
                            };
                        }
                    }
                    let meaning = &contract.semantics;
                    match meaning.shape {
                        InputShape::Views => {
                            crate::application::file_dag::execution::lease::evaluate(
                                self.kernels,
                                contract,
                                permit,
                                &instruction.operation,
                                PreparedInputs::Views(&views),
                                resources
                                    .as_mut()
                                    .ok_or(InvocationFailure::Semantic(OperatingError::NotRun))?,
                                workspace,
                                context,
                                output,
                            )
                            .map_err(InvocationFailure::Semantic)
                        }
                        InputShape::RequestSource => {
                            let empty = ryo_values::domain::rows::types::Table::default();
                            let source = match &instruction.operation {
                                // Rows semantics consume the exact producer's parameter
                                // snapshot through the lease, not an external relation.
                                ryo_values::domain::plans::types::Operation::Rows { .. } => {
                                    BorrowedRows::Computed(&empty)
                                }
                                _ => views.first().copied().ok_or(InvocationFailure::Semantic(
                                    OperatingError::MissingInput { position: 1 },
                                ))?,
                            };
                            crate::application::file_dag::execution::lease::evaluate(
                                self.kernels,
                                contract,
                                permit,
                                &instruction.operation,
                                PreparedInputs::RequestSource(&source),
                                resources
                                    .as_mut()
                                    .ok_or(InvocationFailure::Semantic(OperatingError::NotRun))?,
                                workspace,
                                context,
                                output,
                            )
                            .map_err(InvocationFailure::Semantic)
                        }
                        InputShape::Row { primary } => {
                            // The primary input defines the row contract; the required mask
                            // declares every population that drives that same evaluator.
                            let row_inputs = meaning.row_inputs.0;
                            let primary_shift = u32::try_from(primary.0).map_err(|_| {
                                InvocationFailure::Semantic(OperatingError::WrongOperation)
                            })?;
                            let primary_bit = 1_u8.checked_shl(primary_shift).ok_or(
                                InvocationFailure::Semantic(OperatingError::WrongOperation),
                            )?;
                            if row_inputs == 0 || row_inputs & primary_bit == 0 {
                                return Err(InvocationFailure::Semantic(
                                    OperatingError::WrongOperation,
                                ));
                            }
                            for input in originals.len()..u8::BITS as usize {
                                if row_inputs & (1_u8 << input) != 0 {
                                    return Err(InvocationFailure::Semantic(
                                        OperatingError::MissingInput {
                                            position: input + 1,
                                        },
                                    ));
                                }
                            }
                            let mut cells = Vec::new();
                            for (input, source) in originals.iter().enumerate() {
                                if input >= u8::BITS as usize || row_inputs & (1_u8 << input) == 0 {
                                    continue;
                                }
                                for at in 0..count_of(source).0 {
                                    let row = read_row(source, RowIndex(at), &mut cells).ok_or(
                                        InvocationFailure::Semantic(OperatingError::ShapeMismatch),
                                    )?;
                                    workspace.work.found(input, 1);
                                    let texts = crate::domain::table_texts::types::Texts {
                                        symbols: context.symbols.as_ref(),
                                        source: context.source,
                                        paths: context.paths,
                                        arenas: context.arenas,
                                    };
                                    if !crate::domain::executable_program::kernel::row_admissible(
                                        contract, row, &texts,
                                    )
                                    .map_err(InvocationFailure::Semantic)?
                                    {
                                        continue;
                                    }
                                    if resources.is_none() {
                                        resources = Some(
                                            crate::application::file_dag::invocation::service::prepare(
                                                contract,
                                                &views,
                                                context,
                                                &originals,
                                                workspace,
                                                &setup_semantics,
                                            )
                                            .map_err(InvocationFailure::Semantic)?,
                                        );
                                    }
                                    crate::application::file_dag::execution::lease::evaluate(
                                        self.kernels,
                                        contract,
                                        permit,
                                        &instruction.operation,
                                        PreparedInputs::Row {
                                            input: InputPlace(input),
                                            sides: &views,
                                            row,
                                        },
                                        resources.as_mut().ok_or(InvocationFailure::Semantic(
                                            OperatingError::NotRun,
                                        ))?,
                                        workspace,
                                        context,
                                        output,
                                    )
                                    .map_err(InvocationFailure::Semantic)?;
                                }
                            }
                            Ok(())
                        }
                    }
                }
            }
        })();
        let result = match setup_failure.into_inner() {
            Some(error) => Err(InvocationFailure::Semantic(error)),
            None => result,
        };
        workspace.release_inputs();
        scratch.workspace = Some(invocation_workspace);
        result
    }
}
