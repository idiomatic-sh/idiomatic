//! Probe clients are inserted beside the actual owner; no authority type is copied.
pub(super) struct Probe {
    pub(super) name: &'static str,
    pub(super) client: &'static str,
    pub(super) expected: Option<(&'static str, &'static str)>,
}
pub(super) const OWNER: &str = "src/application/cross_file.rs";
pub(super) const PROBES: &[Probe] = &[
    Probe {
        name: "authorized_seal",
        expected: None,
        client: r#"
mod authorized_seal {
 use crate::application::cross_file::{preparation::{service::{Preparation, SealedBatch}, errors::BatchPreparationError}, errors::CrossFileError};
 use crate::domain::answering::types::Todo;
 fn authorized(todo: &[Todo]) -> Result<SealedBatch, BatchPreparationError> {
  Preparation::ordinary(todo).prepare_all(|_, _| Err(CrossFileError::from(ryo_execution::inbound::use_cases::execution::types::OperatingError::NotRead))).seal()
 }
 const _: fn(&[Todo]) -> Result<SealedBatch, BatchPreparationError> = authorized;
}
"#,
    },
    Probe {
        name: "authorized_service",
        expected: None,
        client: r#"
mod authorized_service {
 use crate::application::cross_file::{service::CrossFile, preparation::service::SealedBatch};
 use crate::application::cross_file::types::ProjectCrossing;
 use ryo_execution::inbound::use_cases::execution::types::ProjectPhase;
 use ryo_language::domain::symbols::types::Symbols;
 fn submit<'phase>(service: &CrossFile, crossing: &ProjectCrossing<'_>, batch: SealedBatch, stage: &ProjectPhase<'phase>, symbols: &mut Symbols) {
  let _ = service.evaluate_batch(crossing, batch, stage, symbols, |_| Ok(()));
 }
 const _: for<'phase> fn(&CrossFile, &ProjectCrossing<'_>, SealedBatch, &ProjectPhase<'phase>, &mut Symbols) = submit;
}
"#,
    },
    Probe {
        name: "forged_sealed",
        expected: Some(("E0451", "entries")),
        client: r#"
mod forged_sealed {
 use crate::application::cross_file::preparation::service::SealedBatch;
 fn forge() -> SealedBatch { SealedBatch { declared: std::sync::Arc::from([]), entries: Box::new([]) } }
 const _: fn() -> SealedBatch = forge;
}
"#,
    },
    Probe {
        name: "aliased_sealed",
        expected: Some(("E0451", "entries")),
        client: r#"
mod aliased_sealed {
 use crate::application::cross_file::preparation::service::SealedBatch as Authority;
 fn forge() -> Authority { Authority { declared: std::sync::Arc::from([]), entries: Box::new([]) } }
 const _: fn() -> Authority = forge;
}
"#,
    },
    Probe {
        name: "self_sealed",
        expected: Some(("E0451", "entries")),
        client: r#"
mod self_sealed {
 use crate::application::cross_file::preparation::service::SealedBatch;
 impl SealedBatch { fn forbidden() -> Self { Self { declared: std::sync::Arc::from([]), entries: Box::new([]) } } }
 const _: fn() -> SealedBatch = SealedBatch::forbidden;
}
"#,
    },
    Probe {
        name: "retarget_sealed",
        expected: Some(("E0616", "entries")),
        client: r#"
mod retarget_sealed {
 use crate::application::cross_file::preparation::service::SealedBatch;
 fn retarget(batch: &mut SealedBatch) { batch.entries = Box::new([]); }
 const _: fn(&mut SealedBatch) = retarget;
}
"#,
    },
    Probe {
        name: "raw_vector_submission",
        expected: Some(("E0308", "SealedBatch")),
        client: r#"
mod raw_vector_submission {
 use crate::application::cross_file::service::CrossFile;
 use crate::application::cross_file::types::ProjectCrossing;
 use ryo_execution::inbound::use_cases::execution::types::ProjectPhase;
 use ryo_language::domain::symbols::types::Symbols;
 fn submit<'phase>(service: &CrossFile, crossing: &ProjectCrossing<'_>, stage: &ProjectPhase<'phase>, symbols: &mut Symbols) {
  let _ = service.evaluate_batch(crossing, Vec::new(), stage, symbols, |_| Ok(()));
 }
 const _: for<'phase> fn(&CrossFile, &ProjectCrossing<'_>, &ProjectPhase<'phase>, &mut Symbols) = submit;
}
"#,
    },
    Probe {
        name: "public_sdk_singleton",
        expected: None,
        client: r#"
mod public_sdk_singleton {
 use ryo_execution::inbound::use_cases::execution::{interface::ProjectRootExecution, types::{ProjectPhase, ProjectTask}};
 use ryo_language::domain::symbols::types::Symbols;
 fn singleton<'phase>(stage: &ProjectPhase<'phase>, task: ProjectTask<'_>, symbols: &mut Symbols) {
  let _ = stage.evaluate(vec![task], symbols, |_| Ok::<(), std::convert::Infallible>(()));
 }
 const _: for<'phase> fn(&ProjectPhase<'phase>, ProjectTask<'_>, &mut Symbols) = singleton;
}
"#,
    },
];
