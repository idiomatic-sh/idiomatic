//! The shared producer fixture crosses the real injected filesystem receipt boundary.
use super::interface::ValidateCaseReceiptsUseCase;
use ryo_benchmark_contracts::domain::receipts::types::CaseKey;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use ryo_benchmark_contracts::domain::requests::types::RuleLabel;
use ryo_benchmark_contracts::domain::workloads::types::{
    benchmark_case_name::value::BenchmarkCaseName, case_count::value::CaseCount,
};
#[derive(Clone, Copy)]
pub enum Scenario {
    Complete,
    Authority,
    Modules,
    Artifacts,
}

/// Validate the shared registration's receipts with one field of it removed, or none.
///
/// # Errors
/// The removed field's name when the use case refuses the registration for it, or what failed: a fixture write, or a
/// complete registration that is refused.
///
/// # Panics
/// When a complete registration verifies other than its one case, a refusal does not name the missing field, or the
/// use case fails with an I/O or TOML error.
pub fn exercise(
    use_case: &dyn ValidateCaseReceiptsUseCase,
    field: Scenario,
) -> Result<usize, &'static str> {
    let mut registration: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/registrations/receipt_only/registration.json"
    ))
    .map_err(|_| "shared registration fixture")?;
    let missing = match field {
        Scenario::Complete => None,
        Scenario::Authority => Some("authority"),
        Scenario::Modules => Some("modules"),
        Scenario::Artifacts => Some("artifacts"),
    };
    if let Some(key) = missing {
        let object = if key == "authority" {
            &mut registration
        } else {
            &mut registration["rules"][0]
        };
        object
            .as_object_mut()
            .ok_or("registration record")?
            .remove(key);
    }
    let temporary = tempfile::tempdir().map_err(|_| "receipt fixture directory")?;
    let registration_path = temporary.path().join("registration.json");
    let receipts = temporary.path().join("observations.jsonl");
    std::fs::write(
        &registration_path,
        serde_json::to_vec(&registration).map_err(|_| "wire bytes")?,
    )
    .map_err(|_| "write registration")?;
    std::fs::write(
        &receipts,
        include_str!("../../../../tests/fixtures/registrations/receipt_only/observations.jsonl"),
    )
    .map_err(|_| "write actual side observations")?;
    match use_case.validate_case_receipts(&registration_path, &receipts) {
        Ok(audit) => {
            assert!(audit.errors.is_empty(), "{:?}", audit.errors);
            assert_eq!(
                audit.verified,
                vec![CaseKey {
                    rule: RuleLabel("A001".into()),
                    name: BenchmarkCaseName("mode".into()),
                }]
            );
            assert_eq!(audit.raw_exact, CaseCount(1));
            assert_eq!(audit.metadata_verified.len(), 1);
            Ok(audit.case_count.0)
        }
        Err(error @ (ReportsError::Invalid(_) | ReportsError::Json(_))) => {
            let message = error.to_string();
            let key = missing.ok_or("complete fixture must decode and verify")?;
            assert!(
                message.contains(&format!("missing field `{key}`")),
                "{message}"
            );
            Err(key)
        }
        Err(error) => panic!("unexpected receipt refusal: {error}"),
    }
}
