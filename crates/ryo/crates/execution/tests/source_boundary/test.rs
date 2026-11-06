//! Only probe insertion differs from production; no authority implementation is copied.
use super::derived::controls;
use super::fixtures::{Expectation, Probe};
use cargo_metadata::{DependencyKind, Message, MetadataCommand, diagnostic::DiagnosticLevel};
use std::{fs, io, path::Path, process::Command};

fn copy_sources(source: &Path, target: &Path) -> io::Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let destination = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_sources(&entry.path(), &destination)?;
        } else {
            fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}

/// Cargo's resolved dependency descriptions preserve workspace inheritance,
/// renames and feature selections without copying the workspace or token types.
fn manifest(source: &Path) -> toml::Table {
    let metadata = MetadataCommand::new()
        .manifest_path(source.join("Cargo.toml"))
        .no_deps()
        .other_options(vec!["--offline".into(), "--locked".into()])
        .exec()
        .unwrap();
    let package = metadata
        .packages
        .iter()
        .find(|package| package.manifest_path.as_std_path() == source.join("Cargo.toml"))
        .unwrap();
    let original: toml::Table = fs::read_to_string(source.join("Cargo.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let workspace: toml::Table = fs::read_to_string(metadata.workspace_root.join("Cargo.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let mut result = toml::Table::new();
    let mut identity = toml::Table::new();
    identity.insert("name".into(), package.name.to_string().into());
    identity.insert("version".into(), package.version.to_string().into());
    identity.insert("edition".into(), package.edition.to_string().into());
    identity.insert(
        "description".into(),
        package.description.clone().unwrap_or_default().into(),
    );
    identity.insert("autotests".into(), false.into());
    result.insert("package".into(), identity.into());
    result.insert("workspace".into(), toml::Table::new().into());
    result.insert("lib".into(), original["lib"].clone());
    result.insert("features".into(), original["features"].clone());
    result.insert("lints".into(), workspace["workspace"]["lints"].clone());
    let mut dependencies = toml::Table::new();
    for dependency in package
        .dependencies
        .iter()
        .filter(|dependency| dependency.kind == DependencyKind::Normal)
    {
        let mut entry = toml::Table::new();
        entry.insert("version".into(), dependency.req.to_string().into());
        entry.insert("package".into(), dependency.name.clone().into());
        entry.insert("optional".into(), dependency.optional.into());
        entry.insert(
            "default-features".into(),
            dependency.uses_default_features.into(),
        );
        entry.insert(
            "features".into(),
            toml::Value::Array(
                dependency
                    .features
                    .iter()
                    .cloned()
                    .map(Into::into)
                    .collect(),
            ),
        );
        if let Some(path) = &dependency.path {
            entry.insert("path".into(), path.to_string().into());
        }
        dependencies.insert(
            dependency
                .rename
                .as_ref()
                .unwrap_or(&dependency.name)
                .clone(),
            entry.into(),
        );
    }
    result.insert("dependencies".into(), dependencies.into());
    result
}

const ADMISSION: &str = "src/application/file_dag/program/admission.rs";
const PROGRAM: &str = "src/application/file_dag/program.rs";
const RUNTIME_MODULE: &str = "src/application/file_dag/program/runtime/mod.rs";
const RUNTIME_TYPES: &str = "src/application/file_dag/program/runtime/types.rs";
const RELATIONS: &str = "src/application/file_dag/relations.rs";
const RELATION_ISSUER: &str = "src/application/file_dag/relations/admission.rs";
const RUNTIME: &str = "src/application/file_dag/program/runtime/service.rs";
const AUTHORIZED: &str = r#"
mod boundary_probe {
    use super::*;
    fn authorized<'a>(execution: &'a ProgramExecution, region: RegionId, inputs: &'a [AdmittedInput], left: &'a AdmittedInput, right: &'a AdmittedInput) {
        let authority = || KernelAdmission { execution, region };
        let _ = crate::application::file_dag::program::runtime::service::KernelPermit::admitted(authority(), inputs);
        let _ = crate::application::file_dag::program::runtime::service::KernelPermit::ordered(authority(), left);
        let _ = crate::application::file_dag::program::runtime::service::KernelPermit::column(authority(), left, right);
    }
    const _: for<'a> fn(&'a ProgramExecution, RegionId, &'a [AdmittedInput], &'a AdmittedInput, &'a AdmittedInput) = authorized;
    fn assert_owned_result<T: Send + Sync + 'static>() {}
    fn owned_walk_result() { assert_owned_result::<crate::application::file_dag::program::native_walk::WalkResult>(); }
    const _: fn() = owned_walk_result;
}
"#;
const SIBLING: &str = r#"
mod boundary_probe {
    use super::admission::AdmittedInput;
    use crate::application::file_dag::program::runtime::service::KernelPermit;
    use crate::application::file_dag::program::runtime::service::KernelAdmission;
    use crate::application::file_dag::program::runtime::service::ProgramExecution;
    use crate::domain::executable_program::types::RegionId;
"#;

#[test]
fn actual_runtime_authority_is_private_to_its_owner() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"));
    let overlay = tempfile::tempdir().unwrap();
    copy_sources(&source.join("src"), &overlay.path().join("src")).unwrap();
    fs::copy(source.join("README.md"), overlay.path().join("README.md")).unwrap();
    let manifest = manifest(source);
    let target = overlay.path().join("target");
    fs::write(
        overlay.path().join("Cargo.toml"),
        toml::to_string(&manifest).unwrap(),
    )
    .unwrap();
    // Keep production's resolved versions while allowing Cargo to trim unused
    // workspace members from this temporary standalone manifest offline.
    fs::copy(
        source.join("../../Cargo.lock"),
        overlay.path().join("Cargo.lock"),
    )
    .unwrap();
    let program = fs::read_to_string(source.join(PROGRAM)).unwrap();
    let admission = fs::read_to_string(source.join(ADMISSION)).unwrap();
    let runtime = fs::read_to_string(source.join(RUNTIME)).unwrap();
    let runtime_module = fs::read_to_string(source.join(RUNTIME_MODULE)).unwrap();
    let runtime_types = fs::read_to_string(source.join(RUNTIME_TYPES)).unwrap();
    let relations = fs::read_to_string(source.join(RELATIONS)).unwrap();
    let relation_issuer = fs::read_to_string(source.join(RELATION_ISSUER)).unwrap();
    let derived_root = fs::read_to_string(source.join(super::derived::ROOT)).unwrap();
    let derived_module = fs::read_to_string(source.join(super::derived::MODULE)).unwrap();
    let derived_issuer = fs::read_to_string(source.join(super::derived::ISSUER)).unwrap();
    let mut probes = vec![
        Probe {
            owner: RUNTIME,
            source: AUTHORIZED.to_owned(),
            expected: Expectation::Accept,
        },
        Probe {
            owner: PROGRAM,
            source: format!(
                "{SIBLING} fn rebind(execution: &mut ProgramExecution, owner: std::rc::Rc<crate::application::file_dag::relations::RelationOwner>) {{ execution.owner = owner; }} const _: fn(&mut ProgramExecution, std::rc::Rc<crate::application::file_dag::relations::RelationOwner>) = rebind; }}"
            ),
            expected: Expectation::Refuse {
                code: "E0616",
                first: "owner",
                second: "private",
            },
        },
        Probe {
            owner: PROGRAM,
            source: format!(
                "{SIBLING} fn forge(execution: &ProgramExecution, region: RegionId) {{ let _ = KernelAdmission {{ execution, region }}; }} const _: fn(&ProgramExecution, RegionId) = forge; }}"
            ),
            expected: Expectation::Refuse {
                code: "E0451",
                first: "execution",
                second: "region",
            },
        },
        Probe {
            owner: PROGRAM,
            source: format!(
                "{SIBLING} fn admitted<'a>(execution: &'a ProgramExecution, inputs: &'a [AdmittedInput]) {{ let _ = KernelPermit::admitted(execution, inputs); }} const _: for<'a> fn(&'a ProgramExecution, &'a [AdmittedInput]) = admitted; }}"
            ),
            expected: Expectation::Refuse {
                code: "E0308",
                first: "KernelAdmission",
                second: "ProgramExecution",
            },
        },
        Probe {
            owner: PROGRAM,
            source: format!(
                "{SIBLING} fn ordered<'a>(execution: &'a ProgramExecution, input: &'a AdmittedInput) {{ let _ = KernelPermit::ordered(execution, input); }} const _: for<'a> fn(&'a ProgramExecution, &'a AdmittedInput) = ordered; }}"
            ),
            expected: Expectation::Refuse {
                code: "E0308",
                first: "KernelAdmission",
                second: "ProgramExecution",
            },
        },
        Probe {
            owner: PROGRAM,
            source: format!(
                "{SIBLING} fn column<'a>(execution: &'a ProgramExecution, input: &'a AdmittedInput) {{ let _ = KernelPermit::column(execution, input, input); }} const _: for<'a> fn(&'a ProgramExecution, &'a AdmittedInput) = column; }}"
            ),
            expected: Expectation::Refuse {
                code: "E0308",
                first: "KernelAdmission",
                second: "ProgramExecution",
            },
        },
        Probe {
            owner: PROGRAM,
            source: format!(
                "{SIBLING} fn retarget(permit: KernelPermit<'_>, region: RegionId) {{ let _ = KernelPermit {{ region, ..permit }}; }} const _: for<'a> fn(KernelPermit<'a>, RegionId) = retarget; }}"
            ),
            expected: Expectation::Refuse {
                code: "E0451",
                first: "region",
                second: "private",
            },
        },
        Probe {
            owner: PROGRAM,
            source: format!(
                "{SIBLING} use super::native_walk::PermittedWalk; fn requeue(request: PermittedWalk<'_, '_, '_>) {{ let _ = PermittedWalk {{ ready: &[], ..request }}; }} const _: for<'a, 'b, 'c> fn(PermittedWalk<'a, 'b, 'c>) = requeue; }}"
            ),
            expected: Expectation::Refuse {
                code: "E0451",
                first: "ready",
                second: "private",
            },
        },
        Probe {
            owner: PROGRAM,
            source: format!(
                "{SIBLING} fn walk_factory(permit: &KernelPermit<'_>) {{ let _ = permit.walk(); }} const _: for<'a, 'b> fn(&'a KernelPermit<'b>) = walk_factory; }}"
            ),
            expected: Expectation::Refuse {
                code: "E0599",
                first: "walk",
                second: "KernelPermit",
            },
        },
    ];
    probes.push(Probe { owner: RUNTIME_MODULE, source: "mod alias_kerneladmission { use crate::application::file_dag::program::runtime::service::KernelAdmission as Authority; fn forge<'a>(value: Authority<'a>, region: crate::domain::executable_program::types::RegionId) -> Authority<'a> { Authority { region, ..value } } }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "region", second: "private" } });
    probes.push(Probe { owner: RUNTIME_MODULE, source: "mod self_kerneladmission { use crate::application::file_dag::program::runtime::service::KernelAdmission as Authority; impl<'a> Authority<'a> { fn forbidden(value: Self, region: crate::domain::executable_program::types::RegionId) -> Self { Self { region, ..value } } } }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "region", second: "private" } });
    probes.push(Probe { owner: RUNTIME_MODULE, source: "mod retarget_kerneladmission { use crate::application::file_dag::program::runtime::service::KernelAdmission as Authority; fn retarget<'a>(value: &mut Authority<'a>, region: crate::domain::executable_program::types::RegionId) { value.region = region; } }".to_owned(), expected: Expectation::Refuse { code: "E0616", first: "region", second: "private" } });
    probes.push(Probe { owner: RUNTIME, source: "mod alias_kerneladmission { use crate::application::file_dag::program::runtime::service::KernelAdmission as Authority; fn forge<'a>(value: Authority<'a>, region: crate::domain::executable_program::types::RegionId) -> Authority<'a> { Authority { region, ..value } } }\nmod self_kerneladmission { use crate::application::file_dag::program::runtime::service::KernelAdmission as Authority; impl<'a> Authority<'a> { fn forbidden(value: Self, region: crate::domain::executable_program::types::RegionId) -> Self { Self { region, ..value } } } }\nmod retarget_kerneladmission { use crate::application::file_dag::program::runtime::service::KernelAdmission as Authority; fn retarget<'a>(value: &mut Authority<'a>, region: crate::domain::executable_program::types::RegionId) { value.region = region; } }".to_owned(), expected: Expectation::Accept });
    probes.push(Probe { owner: RUNTIME_MODULE, source: "mod alias_kernelpermit { use crate::application::file_dag::program::runtime::service::KernelPermit as Authority; fn forge<'a>(value: Authority<'a>, region: crate::domain::executable_program::types::RegionId) -> Authority<'a> { Authority { region, ..value } } }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "region", second: "private" } });
    probes.push(Probe { owner: RUNTIME_MODULE, source: "mod self_kernelpermit { use crate::application::file_dag::program::runtime::service::KernelPermit as Authority; impl<'a> Authority<'a> { fn forbidden(value: Self, region: crate::domain::executable_program::types::RegionId) -> Self { Self { region, ..value } } } }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "region", second: "private" } });
    probes.push(Probe { owner: RUNTIME_MODULE, source: "mod retarget_kernelpermit { use crate::application::file_dag::program::runtime::service::KernelPermit as Authority; fn retarget<'a>(value: &mut Authority<'a>, region: crate::domain::executable_program::types::RegionId) { value.region = region; } }".to_owned(), expected: Expectation::Refuse { code: "E0616", first: "region", second: "private" } });
    probes.push(Probe { owner: RUNTIME, source: "mod alias_kernelpermit { use crate::application::file_dag::program::runtime::service::KernelPermit as Authority; fn forge<'a>(value: Authority<'a>, region: crate::domain::executable_program::types::RegionId) -> Authority<'a> { Authority { region, ..value } } }\nmod self_kernelpermit { use crate::application::file_dag::program::runtime::service::KernelPermit as Authority; impl<'a> Authority<'a> { fn forbidden(value: Self, region: crate::domain::executable_program::types::RegionId) -> Self { Self { region, ..value } } } }\nmod retarget_kernelpermit { use crate::application::file_dag::program::runtime::service::KernelPermit as Authority; fn retarget<'a>(value: &mut Authority<'a>, region: crate::domain::executable_program::types::RegionId) { value.region = region; } }".to_owned(), expected: Expectation::Accept });
    probes.push(Probe { owner: RUNTIME_MODULE, source: "mod alias_readyselection { use crate::application::file_dag::program::runtime::service::ReadySelection as Authority; fn forge<'a, 'b>(value: Authority<'a, 'b>, permit: &'a crate::application::file_dag::program::runtime::service::KernelPermit<'b>) -> Authority<'a, 'b> { Authority { permit, ..value } } }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "permit", second: "private" } });
    probes.push(Probe { owner: RUNTIME_MODULE, source: "mod self_readyselection { use crate::application::file_dag::program::runtime::service::ReadySelection as Authority; impl<'a, 'b> Authority<'a, 'b> { fn forbidden(value: Self, permit: &'a crate::application::file_dag::program::runtime::service::KernelPermit<'b>) -> Self { Self { permit, ..value } } } }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "permit", second: "private" } });
    probes.push(Probe { owner: RUNTIME_MODULE, source: "mod retarget_readyselection { use crate::application::file_dag::program::runtime::service::ReadySelection as Authority; fn retarget<'a, 'b>(value: &mut Authority<'a, 'b>, permit: &'a crate::application::file_dag::program::runtime::service::KernelPermit<'b>) { value.permit = permit; } }".to_owned(), expected: Expectation::Refuse { code: "E0616", first: "permit", second: "private" } });
    probes.push(Probe { owner: RUNTIME, source: "mod alias_readyselection { use crate::application::file_dag::program::runtime::service::ReadySelection as Authority; fn forge<'a, 'b>(value: Authority<'a, 'b>, permit: &'a crate::application::file_dag::program::runtime::service::KernelPermit<'b>) -> Authority<'a, 'b> { Authority { permit, ..value } } }\nmod self_readyselection { use crate::application::file_dag::program::runtime::service::ReadySelection as Authority; impl<'a, 'b> Authority<'a, 'b> { fn forbidden(value: Self, permit: &'a crate::application::file_dag::program::runtime::service::KernelPermit<'b>) -> Self { Self { permit, ..value } } } }\nmod retarget_readyselection { use crate::application::file_dag::program::runtime::service::ReadySelection as Authority; fn retarget<'a, 'b>(value: &mut Authority<'a, 'b>, permit: &'a crate::application::file_dag::program::runtime::service::KernelPermit<'b>) { value.permit = permit; } }".to_owned(), expected: Expectation::Accept });
    probes.push(Probe { owner: RUNTIME_MODULE, source: "mod alias_semanticpermit { use crate::application::file_dag::program::runtime::service::SemanticPermit as Authority; fn forge<'a>(value: Authority<'a>, region: crate::domain::executable_program::types::RegionId) -> Authority<'a> { Authority { region, ..value } } }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "region", second: "private" } });
    probes.push(Probe { owner: RUNTIME_MODULE, source: "mod self_semanticpermit { use crate::application::file_dag::program::runtime::service::SemanticPermit as Authority; impl<'a> Authority<'a> { fn forbidden(value: Self, region: crate::domain::executable_program::types::RegionId) -> Self { Self { region, ..value } } } }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "region", second: "private" } });
    probes.push(Probe { owner: RUNTIME_MODULE, source: "mod retarget_semanticpermit { use crate::application::file_dag::program::runtime::service::SemanticPermit as Authority; fn retarget<'a>(value: &mut Authority<'a>, region: crate::domain::executable_program::types::RegionId) { value.region = region; } }".to_owned(), expected: Expectation::Refuse { code: "E0616", first: "region", second: "private" } });
    probes.push(Probe { owner: RUNTIME, source: "mod alias_semanticpermit { use crate::application::file_dag::program::runtime::service::SemanticPermit as Authority; fn forge<'a>(value: Authority<'a>, region: crate::domain::executable_program::types::RegionId) -> Authority<'a> { Authority { region, ..value } } }\nmod self_semanticpermit { use crate::application::file_dag::program::runtime::service::SemanticPermit as Authority; impl<'a> Authority<'a> { fn forbidden(value: Self, region: crate::domain::executable_program::types::RegionId) -> Self { Self { region, ..value } } } }\nmod retarget_semanticpermit { use crate::application::file_dag::program::runtime::service::SemanticPermit as Authority; fn retarget<'a>(value: &mut Authority<'a>, region: crate::domain::executable_program::types::RegionId) { value.region = region; } }".to_owned(), expected: Expectation::Accept });
    probes.push(Probe { owner: RUNTIME_MODULE, source: "mod alias_permittedphysical { use crate::application::file_dag::program::runtime::service::PermittedPhysical as Authority; fn forge<'a, 'b, 'c>(value: Authority<'a, 'b, 'c>, meaning: ryo_operators::inbound::use_cases::operators::interface::OperatorCall<'b>) -> Authority<'a, 'b, 'c> { Authority { meaning, ..value } } }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "meaning", second: "private" } });
    probes.push(Probe { owner: RUNTIME_MODULE, source: "mod self_permittedphysical { use crate::application::file_dag::program::runtime::service::PermittedPhysical as Authority; impl<'a, 'b, 'c> Authority<'a, 'b, 'c> { fn forbidden(value: Self, meaning: ryo_operators::inbound::use_cases::operators::interface::OperatorCall<'b>) -> Self { Self { meaning, ..value } } } }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "meaning", second: "private" } });
    probes.push(Probe { owner: RUNTIME_MODULE, source: "mod retarget_permittedphysical { use crate::application::file_dag::program::runtime::service::PermittedPhysical as Authority; fn retarget<'a, 'b, 'c>(value: &mut Authority<'a, 'b, 'c>, meaning: ryo_operators::inbound::use_cases::operators::interface::OperatorCall<'b>) { value.meaning = meaning; } }".to_owned(), expected: Expectation::Refuse { code: "E0616", first: "meaning", second: "private" } });
    probes.push(Probe { owner: RUNTIME, source: "mod alias_permittedphysical { use crate::application::file_dag::program::runtime::service::PermittedPhysical as Authority; fn forge<'a, 'b, 'c>(value: Authority<'a, 'b, 'c>, meaning: ryo_operators::inbound::use_cases::operators::interface::OperatorCall<'b>) -> Authority<'a, 'b, 'c> { Authority { meaning, ..value } } }\nmod self_permittedphysical { use crate::application::file_dag::program::runtime::service::PermittedPhysical as Authority; impl<'a, 'b, 'c> Authority<'a, 'b, 'c> { fn forbidden(value: Self, meaning: ryo_operators::inbound::use_cases::operators::interface::OperatorCall<'b>) -> Self { Self { meaning, ..value } } } }\nmod retarget_permittedphysical { use crate::application::file_dag::program::runtime::service::PermittedPhysical as Authority; fn retarget<'a, 'b, 'c>(value: &mut Authority<'a, 'b, 'c>, meaning: ryo_operators::inbound::use_cases::operators::interface::OperatorCall<'b>) { value.meaning = meaning; } }".to_owned(), expected: Expectation::Accept });
    probes.push(Probe { owner: RUNTIME_MODULE, source: "mod alias_programexecution { use crate::application::file_dag::program::runtime::service::ProgramExecution as Authority; fn forge(value: Authority, generation: std::sync::Arc<()>) -> Authority { Authority { generation, ..value } } }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "generation", second: "private" } });
    probes.push(Probe { owner: RUNTIME_MODULE, source: "mod self_programexecution { use crate::application::file_dag::program::runtime::service::ProgramExecution as Authority; impl Authority { fn forbidden(value: Self, generation: std::sync::Arc<()>) -> Self { Self { generation, ..value } } } }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "generation", second: "private" } });
    probes.push(Probe { owner: RUNTIME_MODULE, source: "mod retarget_programexecution { use crate::application::file_dag::program::runtime::service::ProgramExecution as Authority; fn retarget(value: &mut Authority, generation: std::sync::Arc<()>) { value.generation = generation; } }".to_owned(), expected: Expectation::Refuse { code: "E0616", first: "generation", second: "private" } });
    probes.push(Probe { owner: RUNTIME, source: "mod alias_programexecution { use crate::application::file_dag::program::runtime::service::ProgramExecution as Authority; fn forge(value: Authority, generation: std::sync::Arc<()>) -> Authority { Authority { generation, ..value } } }\nmod self_programexecution { use crate::application::file_dag::program::runtime::service::ProgramExecution as Authority; impl Authority { fn forbidden(value: Self, generation: std::sync::Arc<()>) -> Self { Self { generation, ..value } } } }\nmod retarget_programexecution { use crate::application::file_dag::program::runtime::service::ProgramExecution as Authority; fn retarget(value: &mut Authority, generation: std::sync::Arc<()>) { value.generation = generation; } }".to_owned(), expected: Expectation::Accept });
    probes.push(Probe { owner: RUNTIME_MODULE, source: "mod alias_runtimestate { use crate::application::file_dag::program::runtime::service::RuntimeState as Authority; fn forge(value: Authority, generation: std::sync::Arc<()>) -> Authority { Authority { generation, ..value } } }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "generation", second: "private" } });
    probes.push(Probe { owner: RUNTIME_MODULE, source: "mod self_runtimestate { use crate::application::file_dag::program::runtime::service::RuntimeState as Authority; impl Authority { fn forbidden(value: Self, generation: std::sync::Arc<()>) -> Self { Self { generation, ..value } } } }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "generation", second: "private" } });
    probes.push(Probe { owner: RUNTIME_MODULE, source: "mod retarget_runtimestate { use crate::application::file_dag::program::runtime::service::RuntimeState as Authority; fn retarget(value: &mut Authority, generation: std::sync::Arc<()>) { value.generation = generation; } }".to_owned(), expected: Expectation::Refuse { code: "E0616", first: "generation", second: "private" } });
    probes.push(Probe { owner: RUNTIME, source: "mod alias_runtimestate { use crate::application::file_dag::program::runtime::service::RuntimeState as Authority; fn forge(value: Authority, generation: std::sync::Arc<()>) -> Authority { Authority { generation, ..value } } }\nmod self_runtimestate { use crate::application::file_dag::program::runtime::service::RuntimeState as Authority; impl Authority { fn forbidden(value: Self, generation: std::sync::Arc<()>) -> Self { Self { generation, ..value } } } }\nmod retarget_runtimestate { use crate::application::file_dag::program::runtime::service::RuntimeState as Authority; fn retarget(value: &mut Authority, generation: std::sync::Arc<()>) { value.generation = generation; } }".to_owned(), expected: Expectation::Accept });
    probes.push(Probe { owner: RUNTIME_MODULE, source: "mod alias_programadmittedinput { use crate::application::file_dag::program::admission::AdmittedInput as Authority; fn forge(value: Authority, root: crate::application::file_dag::program::admission::ProducerRoot) -> Authority { Authority { root, ..value } } }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "root", second: "private" } });
    probes.push(Probe { owner: RUNTIME_MODULE, source: "mod self_programadmittedinput { use crate::application::file_dag::program::admission::AdmittedInput as Authority; impl Authority { fn forbidden(value: Self, root: crate::application::file_dag::program::admission::ProducerRoot) -> Self { Self { root, ..value } } } }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "root", second: "private" } });
    probes.push(Probe { owner: RUNTIME_MODULE, source: "mod retarget_programadmittedinput { use crate::application::file_dag::program::admission::AdmittedInput as Authority; fn retarget(value: &mut Authority, root: crate::application::file_dag::program::admission::ProducerRoot) { value.root = root; } }".to_owned(), expected: Expectation::Refuse { code: "E0616", first: "root", second: "private" } });
    probes.push(Probe { owner: RELATIONS, source: "mod alias_relationadmittedinput { use crate::application::file_dag::relations::AdmittedInput as Authority; fn forge<'a>(value: Authority<'a>, rows: crate::domain::operator::contract::views::BorrowedRows<'a>) -> Authority<'a> { Authority { rows, ..value } } }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "rows", second: "private" } });
    probes.push(Probe { owner: RELATIONS, source: "mod self_relationadmittedinput { use crate::application::file_dag::relations::AdmittedInput as Authority; impl<'a> Authority<'a> { fn forbidden(value: Self, rows: crate::domain::operator::contract::views::BorrowedRows<'a>) -> Self { Self { rows, ..value } } } }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "rows", second: "private" } });
    probes.push(Probe { owner: RELATIONS, source: "mod retarget_relationadmittedinput { use crate::application::file_dag::relations::AdmittedInput as Authority; fn retarget<'a>(value: &mut Authority<'a>, rows: crate::domain::operator::contract::views::BorrowedRows<'a>) { value.rows = rows; } }".to_owned(), expected: Expectation::Refuse { code: "E0616", first: "rows", second: "private" } });
    probes.push(Probe { owner: RUNTIME_TYPES, source: "mod alias_sibling { use super::super::service::KernelAdmission as Authority; fn forge<'a>(execution: &'a super::super::service::ProgramExecution, region: crate::domain::executable_program::types::RegionId) -> Authority<'a> { Authority { execution, region } } }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "execution", second: "region" } });
    probes.push(Probe { owner: ADMISSION, source: "mod private_program_proof { use super::AdmittedInput as Authority; use super::ProducerRoot; fn alias(value: Authority, root: ProducerRoot) -> Authority { Authority { root, ..value } } impl Authority { fn private_self(value: Self, root: ProducerRoot) -> Self { Self { root, ..value } } } fn retarget(value: &mut Authority, root: ProducerRoot) { value.root = root; } }".to_owned(), expected: Expectation::Accept });
    probes.push(Probe { owner: RELATION_ISSUER, source: "mod private_relation_proof { use super::AdmittedInput as Authority; use crate::domain::operator::contract::views::BorrowedRows; fn alias<'a>(value: Authority<'a>, rows: BorrowedRows<'a>) -> Authority<'a> { Authority { rows, ..value } } impl<'a> Authority<'a> { fn private_self(value: Self, rows: BorrowedRows<'a>) -> Self { Self { rows, ..value } } } fn retarget<'a>(value: &mut Authority<'a>, rows: BorrowedRows<'a>) { value.rows = rows; } }".to_owned(), expected: Expectation::Accept });
    probes.extend(controls());
    for Probe {
        owner,
        source: probe,
        expected,
    } in probes
    {
        fs::write(overlay.path().join(PROGRAM), &program).unwrap();
        fs::write(overlay.path().join(ADMISSION), &admission).unwrap();
        fs::write(overlay.path().join(RUNTIME), &runtime).unwrap();
        fs::write(overlay.path().join(RUNTIME_MODULE), &runtime_module).unwrap();
        fs::write(overlay.path().join(RUNTIME_TYPES), &runtime_types).unwrap();
        fs::write(overlay.path().join(RELATIONS), &relations).unwrap();
        fs::write(overlay.path().join(RELATION_ISSUER), &relation_issuer).unwrap();
        fs::write(overlay.path().join(super::derived::ROOT), &derived_root).unwrap();
        fs::write(overlay.path().join(super::derived::MODULE), &derived_module).unwrap();
        fs::write(overlay.path().join(super::derived::ISSUER), &derived_issuer).unwrap();
        let original = match owner {
            PROGRAM => &program,
            ADMISSION => &admission,
            RUNTIME => &runtime,
            RUNTIME_MODULE => &runtime_module,
            RUNTIME_TYPES => &runtime_types,
            RELATIONS => &relations,
            RELATION_ISSUER => &relation_issuer,
            super::derived::ROOT => &derived_root,
            super::derived::MODULE => &derived_module,
            super::derived::ISSUER => &derived_issuer,
            _ => panic!("unclassified production overlay"),
        };
        fs::write(overlay.path().join(owner), format!("{original}\n{probe}")).unwrap();
        let output = Command::new(env!("CARGO"))
            .args([
                "check",
                "--offline",
                "--lib",
                "--jobs",
                "2",
                "--message-format=json",
            ])
            .arg("--manifest-path")
            .arg(overlay.path().join("Cargo.toml"))
            .arg("--target-dir")
            .arg(&target)
            .output()
            .unwrap();
        let errors = Message::parse_stream(output.stdout.as_slice())
            .map(Result::unwrap)
            .filter_map(|message| match message {
                Message::CompilerMessage(message)
                    if message.message.level == DiagnosticLevel::Error =>
                {
                    Some(message.message)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        match expected {
            Expectation::Accept => {
                assert!(
                    output.status.success(),
                    "authorized production control failed; parsed errors: {errors:#?}; stderr: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert!(errors.is_empty(), "{errors:#?}");
            }
            Expectation::Refuse {
                code,
                first,
                second,
            } => {
                assert!(
                    !output.status.success(),
                    "forbidden sibling probe compiled: {probe}"
                );
                assert!(
                    !errors.is_empty(),
                    "no Rust diagnostic: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                for error in errors {
                    assert_eq!(
                        error.code.as_ref().map(|code| code.code.as_str()),
                        Some(code),
                        "{error:#?}"
                    );
                    let rendered = error.rendered.as_deref().unwrap_or(&error.message);
                    assert!(
                        rendered.contains(first) && rendered.contains(second),
                        "{rendered}"
                    );
                    assert!(
                        error.spans.iter().any(|span| span.is_primary
                            && span.file_name.ends_with(owner)
                            && span.line_start > original.lines().count()),
                        "{error:#?}"
                    );
                }
            }
        }
    }
}
