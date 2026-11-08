//! File-generation freshness under one immutable prepared request and service set.
use super::super::direct::test::{request, schema_column, simple};
use super::super::*;
use ryo_execution_store::application::file_dag::derived::owner::fixtures::resources::RecordedDerived;
#[test]
fn one_prepared_request_reopens_same_file_after_delivery_failure_empty_skip_and_changed_payload() {
    let left = schema_column(Relation::Entry, CellKind::Number, 0);
    let right = schema_column(Relation::SequenceElement, CellKind::Number, 0);
    let payload = schema_column(Relation::SequenceElement, CellKind::Number, 1);
    let recorded = Arc::new(RecordedDerived::default());
    let derived: Arc<dyn ryo_execution_store::outbound::derived::interface::DerivedPort> = recorded.clone();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            plans: vec![request(&[left], &[right], &[(Side::Right, payload)])],
            bindings: vec![vec![]],
            observations: vec![vec![]],
            schedule: RuleSchedule::Independent,
        },
        &[rule()],
        &[Digest([0; 16])],
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )
    .unwrap();
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(real_port())),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("same-prepared-file.py".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ));
    let mut symbols = Symbols::default();
    let mut total_builds = 0;
    for (
        generation,
        keys,
        values,
        refuse_after,
        expected_rows,
        expected_index_rows,
        expected_builds,
    ) in [
        (
            0,
            &[1][..],
            &[(1, 10), (1, 11)][..],
            Some(1),
            vec![10],
            2,
            1,
        ),
        (1, &[][..], &[(1, 99)][..], None, vec![], 0, 1),
        (2, &[1][..], &[(1, 99)][..], None, vec![99], 1, 2),
    ] {
        let builds_before = recorded.builds().len();
        let providers = simple(keys, values, RepresentationKind::Stored);
        let reads = [Counter::new(0), Counter::new(0)];
        let read = |relation| {
            let side = providers
                .iter()
                .position(|p| p.relation == relation)
                .ok_or(OperatingError::NotRead)?;
            reads[side].set(reads[side].get() + 1);
            Ok(providers[side].rows())
        };
        let mut file = open_file(
            &prepared,
            FileId(0),
            ScopePartials::File {
                evaluation: &[],
                publication: &[],
            },
            &[FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Forwarded,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::Disabled,
            }],
            Arc::clone(&services),
            ryo_values::domain::files::types::SourceView(b""),
            ReuseEligibility::Eligible,
            &mut symbols,
            None,
            None)
        .unwrap();
        let weak = Rc::downgrade(&file.owner);
        let mut prefix = vec![];
        let result = file.emit_rows(RuleIndex(0), &read, &mut symbols, &mut |row| {
            if refuse_after == Some(prefix.len()) {
                return Err(OperatingError::WrongBinding);
            }
            prefix.push(row.to_vec());
            Ok(())
        });
        if refuse_after.is_some() {
            assert!(matches!(result, Err(OperatingError::WrongBinding)));
        } else {
            result.expect("actual next generation");
        }
        assert_eq!(
            prefix,
            expected_rows
                .into_iter()
                .map(|n| vec![Cell::Number(Integer(n))])
                .collect::<Vec<_>>(),
            "generation{generation} must observe its own provider rows"
        );
        let work = file.route_observation(RuleIndex(0), StepId(0)).unwrap();
        assert_eq!(work.index_rows, expected_index_rows);
        assert_eq!(work.index_cells, 0);
        assert_eq!(work.index_capacity, 0);
        assert_eq!(
            work.index_representation,
            if keys.is_empty() {
                None
            } else {
                Some(RepresentationKind::Stored)
            }
        );
        let counts = file.owner.derived().counts();
        total_builds += counts.0.0;
        assert_eq!(
            total_builds, expected_builds,
            "real constructions accumulate1→1→2 under the same request/services"
        );
        assert_eq!(
            counts.1, 0,
            "a new file-generation owner cannot hit an old population's index"
        );
        if keys.is_empty() {
            assert!(reads[1].get() > 0, "the declared RHS is still admitted");
            assert_eq!(
                recorded.builds().len(),
                builds_before,
                "the admitted RHS needs no derived payload when the primary is empty"
            );
        }
        let owner = Rc::clone(&file.owner);
        file.finish().unwrap();
        assert_eq!(owner.derived().working_bytes(), 0);
        assert_eq!(owner.derived().owned_bytes(), 0);
        drop(owner);
        assert!(
            weak.upgrade().is_none(),
            "finished generation has no retained execution owner"
        );
    }
    assert_eq!(total_builds, 2);
    let calls = recorded.builds();
    assert_eq!(calls.len(), 2);
    assert!(calls.iter().all(|request| matches!(request,
        ryo_derived::domain::request::types::DerivedRequest::Equality { columns }
        if columns == &[ColumnIndex(right)])));
    let providers = simple(&[], &[(1, 99)], RepresentationKind::Stored);
    let right_refusals = Counter::new(0);
    let read = |relation| {
        if relation == providers[0].relation {
            Ok(providers[0].rows())
        } else {
            assert_eq!(relation, providers[1].relation);
            right_refusals.set(right_refusals.get() + 1);
            Err(OperatingError::NotRead)
        }
    };
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &[FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Forwarded,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }],
        Arc::clone(&services),
        ryo_values::domain::files::types::SourceView(b""),
        ReuseEligibility::Eligible,
        &mut symbols,
        None,
        None,
    )
    .unwrap();
    let mut rows = vec![];
    assert!(matches!(
        file.emit_rows(RuleIndex(0), &read, &mut symbols, &mut |row| {
            rows.push(row.to_vec());
            Ok(())
        }),
        Err(OperatingError::NotRead)
    ));
    assert_eq!(right_refusals.get(), 1);
    assert!(rows.is_empty());
    assert_eq!(file.owner.derived().counts().0, 0);
    assert_eq!(
        recorded.builds().len(),
        2,
        "refused supply cannot trigger a third generation payload"
    );
    file.finish().unwrap();
    // The required malformed-key declaration still refuses an empty primary,
    // before any equality payload can be acquired.
    let mut invalid = request(&[left], &[99], &[(Side::Right, payload)]);
    invalid.emit.step = StepId(0);
    let actual = execute(
        invalid,
        simple(&[], &[(1, 99)], RepresentationKind::Stored),
        &mut symbols,
        b"",
        false,
        real_port(),
        Faults::default(),
    )
    .unwrap();
    assert!(matches!(
        actual.result,
        Err(OperatingError::Table(
            ryo_values::domain::rows::errors::RowsError::NoSuchColumn { index: 99 }
        ))
    ));
    assert!(actual.derived_builds.is_empty());
}
