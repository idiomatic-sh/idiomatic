//! Only probe insertion differs from production; no authority implementation is copied.
use super::derived::controls;
use super::fixtures::{Compilation, Expectation, Probe, copy_sources, manifest, ADMISSION, PROGRAM, RUNTIME_MODULE, RUNTIME_TYPES, RELATIONS, REQUEST, RELATION_ISSUER, RUNTIME, AUTHORIZED, SIBLING};
use cargo_metadata::Message;
use cargo_metadata::diagnostic::DiagnosticLevel;
use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn actual_runtime_authority_is_private_to_its_owner() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"));
    let overlay = tempfile::tempdir().unwrap();
    copy_sources(&source.join("src"), &overlay.path().join("src")).unwrap();
    fs::copy(source.join("README.md"), overlay.path().join("README.md")).unwrap();
    let manifest = manifest(source);
    let target = overlay.path().join("target");
    let library_name = manifest["lib"]["name"].as_str().unwrap();
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
    let request = fs::read_to_string(source.join(REQUEST)).unwrap();
    let graph_issuer = fs::read_to_string(source.join(super::derived::GRAPH_ISSUER)).unwrap();
    let derived_root = fs::read_to_string(source.join(super::derived::ROOT)).unwrap();
    let derived_module = fs::read_to_string(source.join(super::derived::MODULE)).unwrap();
    let derived_issuer = fs::read_to_string(source.join(super::derived::ISSUER)).unwrap();
    let kernel_service = fs::read_to_string(source.join(super::derived::KERNEL)).unwrap();
    let cache_types = fs::read_to_string(source.join(super::derived::CACHE)).unwrap();
    let access_types = fs::read_to_string(source.join(super::derived::TYPES)).unwrap();
    let inbound_use_case = fs::read_to_string(source.join(super::derived::INBOUND)).unwrap();
    let activation_setup = fs::read_to_string(source.join(super::activation::SETUP)).unwrap();
    let mut probes = vec![
        Probe {
            compilation: Compilation::Library,
            owner: RUNTIME,
            source: AUTHORIZED.to_owned(),
            expected: Expectation::Accept,
        },
        Probe {
            compilation: Compilation::Library,
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
            compilation: Compilation::Library,
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
            compilation: Compilation::Library,
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
            compilation: Compilation::Library,
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
            compilation: Compilation::Library,
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
            compilation: Compilation::Library,
            owner: PROGRAM,
            source: format!(
                "{SIBLING} use super::native_walk::lease::service::PermittedWalk; fn requeue(request: PermittedWalk<'_, '_, '_>) {{ let _ = PermittedWalk {{ ready: &[], ..request }}; }} const _: for<'a, 'b, 'c> fn(PermittedWalk<'a, 'b, 'c>) = requeue; }}"
            ),
            expected: Expectation::Refuse {
                code: "E0451",
                first: "ready",
                second: "private",
            },
        },
        Probe {
            compilation: Compilation::Library,
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
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME_MODULE, source: "mod alias_kerneladmission { use crate::application::file_dag::program::runtime::service::KernelAdmission as Authority; fn forge<'a>(value: Authority<'a>, region: crate::domain::executable_program::types::RegionId) -> Authority<'a> { Authority { region, ..value } }  const _: () = { let _ = forge; }; }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "region", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME_MODULE, source: "mod self_kerneladmission { use crate::application::file_dag::program::runtime::service::KernelAdmission as Authority; impl<'a> Authority<'a> { fn forbidden(value: Self, region: crate::domain::executable_program::types::RegionId) -> Self { Self { region, ..value } } }  const _: () = { let _ = Authority::forbidden; }; }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "region", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME_MODULE, source: "mod retarget_kerneladmission { use crate::application::file_dag::program::runtime::service::KernelAdmission as Authority; fn retarget<'a>(value: &mut Authority<'a>, region: crate::domain::executable_program::types::RegionId) { value.region = region; }  const _: () = { let _ = retarget; }; }".to_owned(), expected: Expectation::Refuse { code: "E0616", first: "region", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME, source: "mod alias_kerneladmission { use crate::application::file_dag::program::runtime::service::KernelAdmission as Authority; fn forge<'a>(value: Authority<'a>, region: crate::domain::executable_program::types::RegionId) -> Authority<'a> { Authority { region, ..value } }  const _: () = { let _ = forge; }; }\nmod self_kerneladmission { use crate::application::file_dag::program::runtime::service::KernelAdmission as Authority; impl<'a> Authority<'a> { fn forbidden(value: Self, region: crate::domain::executable_program::types::RegionId) -> Self { Self { region, ..value } } }  const _: () = { let _ = Authority::forbidden; }; }\nmod retarget_kerneladmission { use crate::application::file_dag::program::runtime::service::KernelAdmission as Authority; fn retarget<'a>(value: &mut Authority<'a>, region: crate::domain::executable_program::types::RegionId) { value.region = region; }  const _: () = { let _ = retarget; }; }".to_owned(), expected: Expectation::Accept });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME_MODULE, source: "mod alias_kernelpermit { use crate::application::file_dag::program::runtime::service::KernelPermit as Authority; fn forge<'a>(value: Authority<'a>, region: crate::domain::executable_program::types::RegionId) -> Authority<'a> { Authority { region, ..value } }  const _: () = { let _ = forge; }; }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "region", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME_MODULE, source: "mod self_kernelpermit { use crate::application::file_dag::program::runtime::service::KernelPermit as Authority; impl<'a> Authority<'a> { fn forbidden(value: Self, region: crate::domain::executable_program::types::RegionId) -> Self { Self { region, ..value } } }  const _: () = { let _ = Authority::forbidden; }; }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "region", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME_MODULE, source: "mod retarget_kernelpermit { use crate::application::file_dag::program::runtime::service::KernelPermit as Authority; fn retarget<'a>(value: &mut Authority<'a>, region: crate::domain::executable_program::types::RegionId) { value.region = region; }  const _: () = { let _ = retarget; }; }".to_owned(), expected: Expectation::Refuse { code: "E0616", first: "region", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME, source: "mod alias_kernelpermit { use crate::application::file_dag::program::runtime::service::KernelPermit as Authority; fn forge<'a>(value: Authority<'a>, region: crate::domain::executable_program::types::RegionId) -> Authority<'a> { Authority { region, ..value } }  const _: () = { let _ = forge; }; }\nmod self_kernelpermit { use crate::application::file_dag::program::runtime::service::KernelPermit as Authority; impl<'a> Authority<'a> { fn forbidden(value: Self, region: crate::domain::executable_program::types::RegionId) -> Self { Self { region, ..value } } }  const _: () = { let _ = Authority::forbidden; }; }\nmod retarget_kernelpermit { use crate::application::file_dag::program::runtime::service::KernelPermit as Authority; fn retarget<'a>(value: &mut Authority<'a>, region: crate::domain::executable_program::types::RegionId) { value.region = region; }  const _: () = { let _ = retarget; }; }".to_owned(), expected: Expectation::Accept });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME_MODULE, source: "mod alias_readyselection { use crate::application::file_dag::program::runtime::service::ReadySelection as Authority; fn forge<'a, 'b>(value: Authority<'a, 'b>, permit: &'a crate::application::file_dag::program::runtime::service::KernelPermit<'b>) -> Authority<'a, 'b> { Authority { permit, ..value } }  const _: () = { let _ = forge; }; }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "permit", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME_MODULE, source: "mod self_readyselection { use crate::application::file_dag::program::runtime::service::ReadySelection as Authority; impl<'a, 'b> Authority<'a, 'b> { fn forbidden(value: Self, permit: &'a crate::application::file_dag::program::runtime::service::KernelPermit<'b>) -> Self { Self { permit, ..value } } }  const _: () = { let _ = Authority::forbidden; }; }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "permit", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME_MODULE, source: "mod retarget_readyselection { use crate::application::file_dag::program::runtime::service::ReadySelection as Authority; fn retarget<'a, 'b>(value: &mut Authority<'a, 'b>, permit: &'a crate::application::file_dag::program::runtime::service::KernelPermit<'b>) { value.permit = permit; }  const _: () = { let _ = retarget; }; }".to_owned(), expected: Expectation::Refuse { code: "E0616", first: "permit", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME, source: "mod alias_readyselection { use crate::application::file_dag::program::runtime::service::ReadySelection as Authority; fn forge<'a, 'b>(value: Authority<'a, 'b>, permit: &'a crate::application::file_dag::program::runtime::service::KernelPermit<'b>) -> Authority<'a, 'b> { Authority { permit, ..value } }  const _: () = { let _ = forge; }; }\nmod self_readyselection { use crate::application::file_dag::program::runtime::service::ReadySelection as Authority; impl<'a, 'b> Authority<'a, 'b> { fn forbidden(value: Self, permit: &'a crate::application::file_dag::program::runtime::service::KernelPermit<'b>) -> Self { Self { permit, ..value } } }  const _: () = { let _ = Authority::forbidden; }; }\nmod retarget_readyselection { use crate::application::file_dag::program::runtime::service::ReadySelection as Authority; fn retarget<'a, 'b>(value: &mut Authority<'a, 'b>, permit: &'a crate::application::file_dag::program::runtime::service::KernelPermit<'b>) { value.permit = permit; }  const _: () = { let _ = retarget; }; }".to_owned(), expected: Expectation::Accept });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME_MODULE, source: "mod alias_semanticpermit { use crate::application::file_dag::program::runtime::service::SemanticPermit as Authority; fn forge<'a>(value: Authority<'a>, region: crate::domain::executable_program::types::RegionId) -> Authority<'a> { Authority { region, ..value } }  const _: () = { let _ = forge; }; }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "region", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME_MODULE, source: "mod self_semanticpermit { use crate::application::file_dag::program::runtime::service::SemanticPermit as Authority; impl<'a> Authority<'a> { fn forbidden(value: Self, region: crate::domain::executable_program::types::RegionId) -> Self { Self { region, ..value } } }  const _: () = { let _ = Authority::forbidden; }; }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "region", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME_MODULE, source: "mod retarget_semanticpermit { use crate::application::file_dag::program::runtime::service::SemanticPermit as Authority; fn retarget<'a>(value: &mut Authority<'a>, region: crate::domain::executable_program::types::RegionId) { value.region = region; }  const _: () = { let _ = retarget; }; }".to_owned(), expected: Expectation::Refuse { code: "E0616", first: "region", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME, source: "mod alias_semanticpermit { use crate::application::file_dag::program::runtime::service::SemanticPermit as Authority; fn forge<'a>(value: Authority<'a>, region: crate::domain::executable_program::types::RegionId) -> Authority<'a> { Authority { region, ..value } }  const _: () = { let _ = forge; }; }\nmod self_semanticpermit { use crate::application::file_dag::program::runtime::service::SemanticPermit as Authority; impl<'a> Authority<'a> { fn forbidden(value: Self, region: crate::domain::executable_program::types::RegionId) -> Self { Self { region, ..value } } }  const _: () = { let _ = Authority::forbidden; }; }\nmod retarget_semanticpermit { use crate::application::file_dag::program::runtime::service::SemanticPermit as Authority; fn retarget<'a>(value: &mut Authority<'a>, region: crate::domain::executable_program::types::RegionId) { value.region = region; }  const _: () = { let _ = retarget; }; }".to_owned(), expected: Expectation::Accept });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME_MODULE, source: "mod alias_permittedphysical { use crate::application::file_dag::program::runtime::service::PermittedPhysical as Authority; fn forge<'a, 'b, 'c>(value: Authority<'a, 'b, 'c>, meaning: ryo_operators::inbound::use_cases::operators::types::OperatorCall<'b>) -> Authority<'a, 'b, 'c> { Authority { meaning, ..value } }  const _: () = { let _ = forge; }; }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "meaning", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME_MODULE, source: "mod self_permittedphysical { use crate::application::file_dag::program::runtime::service::PermittedPhysical as Authority; impl<'a, 'b, 'c> Authority<'a, 'b, 'c> { fn forbidden(value: Self, meaning: ryo_operators::inbound::use_cases::operators::types::OperatorCall<'b>) -> Self { Self { meaning, ..value } } }  const _: () = { let _ = Authority::forbidden; }; }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "meaning", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME_MODULE, source: "mod retarget_permittedphysical { use crate::application::file_dag::program::runtime::service::PermittedPhysical as Authority; fn retarget<'a, 'b, 'c>(value: &mut Authority<'a, 'b, 'c>, meaning: ryo_operators::inbound::use_cases::operators::types::OperatorCall<'b>) { value.meaning = meaning; }  const _: () = { let _ = retarget; }; }".to_owned(), expected: Expectation::Refuse { code: "E0616", first: "meaning", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME, source: "mod alias_permittedphysical { use crate::application::file_dag::program::runtime::service::PermittedPhysical as Authority; fn forge<'a, 'b, 'c>(value: Authority<'a, 'b, 'c>, meaning: ryo_operators::inbound::use_cases::operators::types::OperatorCall<'b>) -> Authority<'a, 'b, 'c> { Authority { meaning, ..value } }  const _: () = { let _ = forge; }; }\nmod self_permittedphysical { use crate::application::file_dag::program::runtime::service::PermittedPhysical as Authority; impl<'a, 'b, 'c> Authority<'a, 'b, 'c> { fn forbidden(value: Self, meaning: ryo_operators::inbound::use_cases::operators::types::OperatorCall<'b>) -> Self { Self { meaning, ..value } } }  const _: () = { let _ = Authority::forbidden; }; }\nmod retarget_permittedphysical { use crate::application::file_dag::program::runtime::service::PermittedPhysical as Authority; fn retarget<'a, 'b, 'c>(value: &mut Authority<'a, 'b, 'c>, meaning: ryo_operators::inbound::use_cases::operators::types::OperatorCall<'b>) { value.meaning = meaning; }  const _: () = { let _ = retarget; }; }".to_owned(), expected: Expectation::Accept });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME_MODULE, source: "mod alias_programexecution { use crate::application::file_dag::program::runtime::service::ProgramExecution as Authority; fn forge(value: Authority, generation: std::sync::Arc<()>) -> Authority { Authority { generation, ..value } }  const _: () = { let _ = forge; }; }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "generation", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME_MODULE, source: "mod self_programexecution { use crate::application::file_dag::program::runtime::service::ProgramExecution as Authority; impl Authority { fn forbidden(value: Self, generation: std::sync::Arc<()>) -> Self { Self { generation, ..value } } }  const _: () = { let _ = Authority::forbidden; }; }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "generation", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME_MODULE, source: "mod retarget_programexecution { use crate::application::file_dag::program::runtime::service::ProgramExecution as Authority; fn retarget(value: &mut Authority, generation: std::sync::Arc<()>) { value.generation = generation; }  const _: () = { let _ = retarget; }; }".to_owned(), expected: Expectation::Refuse { code: "E0616", first: "generation", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME, source: "mod alias_programexecution { use crate::application::file_dag::program::runtime::service::ProgramExecution as Authority; fn forge(value: Authority, generation: std::sync::Arc<()>) -> Authority { Authority { generation, ..value } }  const _: () = { let _ = forge; }; }\nmod self_programexecution { use crate::application::file_dag::program::runtime::service::ProgramExecution as Authority; impl Authority { fn forbidden(value: Self, generation: std::sync::Arc<()>) -> Self { Self { generation, ..value } } }  const _: () = { let _ = Authority::forbidden; }; }\nmod retarget_programexecution { use crate::application::file_dag::program::runtime::service::ProgramExecution as Authority; fn retarget(value: &mut Authority, generation: std::sync::Arc<()>) { value.generation = generation; }  const _: () = { let _ = retarget; }; }".to_owned(), expected: Expectation::Accept });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME_MODULE, source: "mod alias_runtimestate { use crate::application::file_dag::program::runtime::service::RuntimeState as Authority; fn forge(value: Authority, generation: std::sync::Arc<()>) -> Authority { Authority { generation, ..value } }  const _: () = { let _ = forge; }; }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "generation", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME_MODULE, source: "mod self_runtimestate { use crate::application::file_dag::program::runtime::service::RuntimeState as Authority; impl Authority { fn forbidden(value: Self, generation: std::sync::Arc<()>) -> Self { Self { generation, ..value } } }  const _: () = { let _ = Authority::forbidden; }; }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "generation", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME_MODULE, source: "mod retarget_runtimestate { use crate::application::file_dag::program::runtime::service::RuntimeState as Authority; fn retarget(value: &mut Authority, generation: std::sync::Arc<()>) { value.generation = generation; }  const _: () = { let _ = retarget; }; }".to_owned(), expected: Expectation::Refuse { code: "E0616", first: "generation", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME, source: "mod alias_runtimestate { use crate::application::file_dag::program::runtime::service::RuntimeState as Authority; fn forge(value: Authority, generation: std::sync::Arc<()>) -> Authority { Authority { generation, ..value } }  const _: () = { let _ = forge; }; }\nmod self_runtimestate { use crate::application::file_dag::program::runtime::service::RuntimeState as Authority; impl Authority { fn forbidden(value: Self, generation: std::sync::Arc<()>) -> Self { Self { generation, ..value } } }  const _: () = { let _ = Authority::forbidden; }; }\nmod retarget_runtimestate { use crate::application::file_dag::program::runtime::service::RuntimeState as Authority; fn retarget(value: &mut Authority, generation: std::sync::Arc<()>) { value.generation = generation; }  const _: () = { let _ = retarget; }; }".to_owned(), expected: Expectation::Accept });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME_MODULE, source: "mod alias_programadmittedinput { use crate::application::file_dag::program::admission::AdmittedInput as Authority; fn forge(value: Authority, root: crate::application::file_dag::program::admission::ProducerRoot) -> Authority { Authority { root, ..value } }  const _: () = { let _ = forge; }; }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "root", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME_MODULE, source: "mod self_programadmittedinput { use crate::application::file_dag::program::admission::AdmittedInput as Authority; impl Authority { fn forbidden(value: Self, root: crate::application::file_dag::program::admission::ProducerRoot) -> Self { Self { root, ..value } } }  const _: () = { let _ = Authority::forbidden; }; }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "root", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME_MODULE, source: "mod retarget_programadmittedinput { use crate::application::file_dag::program::admission::AdmittedInput as Authority; fn retarget(value: &mut Authority, root: crate::application::file_dag::program::admission::ProducerRoot) { value.root = root; }  const _: () = { let _ = retarget; }; }".to_owned(), expected: Expectation::Refuse { code: "E0616", first: "root", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RELATIONS, source: "mod alias_relationadmittedinput { use crate::application::file_dag::relations::admission::service::BorrowedInput as Authority; fn forge<'a>(value: Authority<'a>, rows: crate::domain::table_model::types::RepresentationInput<'a>) -> Authority<'a> { Authority { rows, ..value } }  const _: () = { let _ = forge; }; }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "rows", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RELATIONS, source: "mod self_relationadmittedinput { use crate::application::file_dag::relations::admission::service::BorrowedInput as Authority; impl<'a> Authority<'a> { fn forbidden(value: Self, rows: crate::domain::table_model::types::RepresentationInput<'a>) -> Self { Self { rows, ..value } } }  const _: () = { let _ = Authority::forbidden; }; }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "rows", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RELATIONS, source: "mod retarget_relationadmittedinput { use crate::application::file_dag::relations::admission::service::BorrowedInput as Authority; fn retarget<'a>(value: &mut Authority<'a>, rows: crate::domain::table_model::types::RepresentationInput<'a>) { value.rows = rows; }  const _: () = { let _ = retarget; }; }".to_owned(), expected: Expectation::Refuse { code: "E0616", first: "rows", second: "private" } });
    probes.push(Probe { compilation: Compilation::Library, owner: RUNTIME_TYPES, source: "mod alias_sibling { use super::super::service::KernelAdmission as Authority; fn forge<'a>(execution: &'a super::super::service::ProgramExecution, region: crate::domain::executable_program::types::RegionId) -> Authority<'a> { Authority { execution, region } }  const _: () = { let _ = forge; }; }".to_owned(), expected: Expectation::Refuse { code: "E0451", first: "execution", second: "region" } });
    probes.push(Probe { compilation: Compilation::Library, owner: ADMISSION, source: "mod private_program_proof { use super::AdmittedInput as Authority; use super::ProducerRoot; fn alias(value: Authority, root: ProducerRoot) -> Authority { Authority { root, ..value } } impl Authority { fn private_self(value: Self, root: ProducerRoot) -> Self { Self { root, ..value } } } fn retarget(value: &mut Authority, root: ProducerRoot) { value.root = root; }  const _: () = { let _ = alias; let _ = Authority::private_self; let _ = retarget; }; }".to_owned(), expected: Expectation::Accept });
    probes.push(Probe { compilation: Compilation::Library, owner: RELATION_ISSUER, source: "mod private_relation_proof { use super::BorrowedInput as Authority; use crate::domain::table_model::types::RepresentationInput; fn alias<'a>(value: Authority<'a>, rows: RepresentationInput<'a>) -> Authority<'a> { Authority { rows, ..value } } impl<'a> Authority<'a> { fn private_self(value: Self, rows: RepresentationInput<'a>) -> Self { Self { rows, ..value } } } fn retarget<'a>(value: &mut Authority<'a>, rows: RepresentationInput<'a>) { value.rows = rows; }  const _: () = { let _ = alias; let _ = Authority::private_self; let _ = retarget; }; }".to_owned(), expected: Expectation::Accept });
    // These clients are appended to the actual program sibling. They cannot
    // name raw catalog implementations or retarget a registered classification.
    probes.extend([
        Probe { compilation: Compilation::Library,
            owner: PROGRAM,
            source: "mod unregistered_representation { fn bypass(_: &dyn crate::domain::table_model::collections::representations::interface::RepresentationsItem) {} const _: fn(&dyn crate::domain::table_model::collections::representations::interface::RepresentationsItem) = bypass; }".into(),
            expected: Expectation::Refuse { code: "E0603", first: "RepresentationsItem", second: "private" },
        },
        Probe { compilation: Compilation::Library,
            owner: PROGRAM,
            source: "mod retarget_registered_operator { use crate::domain::operator::contract::registration::RegisteredOperator; use ryo_values::domain::query_operators::types::OperationKind; fn bypass(route: &mut RegisteredOperator) { route.kind = OperationKind::Project; } const _: fn(&mut RegisteredOperator) = bypass; }".into(),
            expected: Expectation::Refuse { code: "E0616", first: "kind", second: "private" },
        },
        Probe { compilation: Compilation::Library,
            owner: PROGRAM,
            source: "mod retarget_registered_representation { use crate::domain::table_model::collections::representations::types::RepresentationKind; use crate::domain::table_model::types::registered_representation::value::RegisteredRepresentation; fn bypass(route: &mut RegisteredRepresentation) { route.kind = RepresentationKind::Computed; } const _: fn(&mut RegisteredRepresentation) = bypass; }".into(),
            expected: Expectation::Refuse { code: "E0616", first: "kind", second: "private" },
        },
        Probe { compilation: Compilation::Library,
            owner: PROGRAM,
            source: "mod registered_operator_preparation { use crate::domain::{operator::contract::types::OperatorContract, operating::errors::OperatingError}; use ryo_values::domain::plans::types::Operation; fn prepare(operation: &Operation) -> Result<OperatorContract, OperatingError> { crate::domain::operator::service::prepare_operator(ryo_values::domain::plans::service::kind_of(operation), operation, &[], &[]) } const _: fn(&Operation) -> Result<OperatorContract, OperatingError> = prepare; }".into(),
            expected: Expectation::Accept,
        },
        Probe { compilation: Compilation::Library,
            owner: PROGRAM,
            source: "mod registered_representation_read { use crate::domain::table_model::collections::representations::types::RelationContract; use crate::domain::table_model::types::RepresentationInput; use crate::domain::operating::errors::OperatingError; fn prepare<'a>(input: RepresentationInput<'a>) -> Result<RelationContract<'a>, OperatingError> { input.contract() } const _: for<'a> fn(RepresentationInput<'a>) -> Result<RelationContract<'a>, OperatingError> = prepare; }".into(),
            expected: Expectation::Accept,
        },
    ]);
    probes.extend(controls());
    probes.extend(super::activation::controls());
    probes.extend(super::publication::controls());
    probes.extend(super::facade::controls());
    probes.extend(super::submission::controls());
    probes.extend(super::hardening::controls());
    for Probe {
        compilation,
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
        fs::write(overlay.path().join(REQUEST), &request).unwrap();
        fs::write(
            overlay.path().join(super::derived::GRAPH_ISSUER),
            &graph_issuer,
        )
        .unwrap();
        fs::write(overlay.path().join(super::derived::ROOT), &derived_root).unwrap();
        fs::write(overlay.path().join(super::derived::MODULE), &derived_module).unwrap();
        fs::write(overlay.path().join(super::derived::ISSUER), &derived_issuer).unwrap();
        fs::write(overlay.path().join(super::derived::KERNEL), &kernel_service).unwrap();
        fs::write(
            overlay.path().join(super::derived::INBOUND),
            &inbound_use_case,
        )
        .unwrap();
        fs::write(overlay.path().join(super::derived::CACHE), &cache_types).unwrap();
        fs::write(overlay.path().join(super::derived::TYPES), &access_types).unwrap();
        fs::write(
            overlay.path().join(super::activation::SETUP),
            &activation_setup,
        )
        .unwrap();
        let original = match owner {
            super::activation::SETUP => &activation_setup,
            PROGRAM => &program,
            ADMISSION => &admission,
            RUNTIME => &runtime,
            RUNTIME_MODULE => &runtime_module,
            RUNTIME_TYPES => &runtime_types,
            RELATIONS => &relations,
            RELATION_ISSUER => &relation_issuer,
            REQUEST => &request,
            super::derived::GRAPH_ISSUER => &graph_issuer,
            super::derived::ROOT => &derived_root,
            super::derived::MODULE => &derived_module,
            super::derived::ISSUER => &derived_issuer,
            super::derived::KERNEL => &kernel_service,
            super::derived::INBOUND => &inbound_use_case,
            super::derived::CACHE => &cache_types,
            super::derived::TYPES => &access_types,
            _ => panic!("unclassified production overlay"),
        };
        let probe = match compilation {
            Compilation::Library | Compilation::WantedMutation => probe,
            Compilation::LibraryTests => format!("#[cfg(test)]\n{probe}"),
        };
        let (postimage, first_line, last_line) = if let Compilation::WantedMutation = compilation {
            let anchor = super::activation::ANCHOR;
            assert_eq!(original.matches(anchor).count(), 1);
            let at = original.find(anchor).unwrap();
            let line = original[..at].bytes().filter(|byte| *byte == b'\n').count() + 1;
            (
                original.replacen(anchor, &format!("{probe}\n{anchor}"), 1),
                line,
                line + probe.lines().count() - 1,
            )
        } else {
            (
                format!("{original}\n{probe}"),
                original.lines().count() + 1,
                usize::MAX,
            )
        };
        fs::write(overlay.path().join(owner), postimage).unwrap();
        let mut command = Command::new(env!("CARGO"));
        if let Compilation::LibraryTests = compilation {
            command.arg("check").arg("--tests");
        } else {
            command.arg("check");
        }
        let output = command
            .args(["--offline", "--lib", "--jobs", "2", "--message-format=json"])
            .arg("--manifest-path")
            .arg(overlay.path().join("Cargo.toml"))
            .arg("--target-dir")
            .arg(&target)
            .output()
            .unwrap();
        let messages = Message::parse_stream(output.stdout.as_slice())
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        let checked_library_tests = messages.iter().any(|message| {
            matches!(message, Message::CompilerArtifact(artifact)
                if artifact.target.name == library_name && artifact.profile.test)
        });
        let errors = messages
            .into_iter()
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
                    "authorized {compilation:?} control failed; parsed errors: {errors:#?}; stderr: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert!(errors.is_empty(), "{errors:#?}");
                if let Compilation::LibraryTests = compilation {
                    assert!(
                        checked_library_tests,
                        "authorized test-only probe did not compile the actual library-test target"
                    );
                }
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
                            && span.line_start >= first_line
                            && span.line_start <= last_line),
                        "{error:#?}"
                    );
                }
            }
        }
    }
}
