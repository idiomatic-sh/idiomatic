//! Every public trait of the submission surface is an entry with a control, or is named with its reason.
use super::super::*;

/// The files that declare the entry traits and the read-only traits: the two use case interfaces, the interface of each handle's
/// concept and the rule compilation service.
const INTERFACES: [&str; 8] = [
    "src/inbound/use_cases/prepare/interface.rs",
    "src/inbound/use_cases/execution/interface.rs",
    "src/application/file_dag/program/prepared_execution/interface.rs",
    "src/application/file_dag/program/request_execution/interface.rs",
    "src/application/file_dag/program/file_session/interface.rs",
    "src/application/file_dag/program/project_phase/interface.rs",
    "src/application/file_dag/program/observation_stage/interface.rs",
    "src/application/rule_compilation/service.rs",
];

/// The text of every file in `INTERFACES`, one after another.
fn interface_sources(root: &Path) -> String {
    INTERFACES
        .iter()
        .map(|file| std::fs::read_to_string(root.join(file)).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Methods of each entry trait in the interface files, by the prefix of their `Method` variants.
const TRAITS: [(&str, &str); 8] = [
    ("PrepareUseCase", "UseCasePrepare"),
    ("ExecutionUseCase", "UseCaseExecution"),
    ("ExecutionRequest", "Request"),
    ("FileExecution", "File"),
    ("ProjectRootExecution", "Roots"),
    ("ProjectStage", "Stage"),
    ("ObservationExecution", "Observed"),
    ("ObservationPublication", "Publication"),
];

/// The public traits of `interface.rs` that are no entry: each states why none of its methods admits or mutates a root.
const NON_ENTRY: [(&str, &str); 4] = [
    (
        "PreparedExecution",
        "read-only metadata of a prepared program: a rule outside it answers an error and nothing is held, admitted or changed",
    ),
    (
        "ExecutionCompilation",
        "compiles the rules a request declares into plans and identities before any root exists",
    ),
    (
        "PreparedQueries",
        "reads the plans and bindings a program was prepared from; it takes no rule, scope or intent",
    ),
    (
        "PreparedReporting",
        "reads the run metadata and the replay closure of a finished preparation; it takes no root",
    ),
];

#[test]
pub(super) fn every_public_trait_is_an_entry_with_variants_or_is_named_with_its_reason() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let interface = interface_sources(root);
    let declared: Vec<&str> = interface
        .lines()
        .filter_map(|line| line.strip_prefix("pub trait "))
        .map(|rest| {
            rest.split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .next()
                .unwrap_or_default()
        })
        .collect();
    assert!(!declared.is_empty(), "interface.rs declares public traits");
    for name in &declared {
        let entry = TRAITS.iter().any(|(each, _)| each == name);
        let reason = NON_ENTRY
            .iter()
            .find(|(each, _)| each == name)
            .map(|(_, reason)| *reason);
        assert!(
            entry != reason.is_some(),
            "{name} is in exactly one of TRAITS and NON_ENTRY: a new public trait is classified here before it passes"
        );
        if let Some(reason) = reason {
            assert!(!reason.is_empty(), "{name} states why it is no entry");
        }
    }
    for (name, _) in TRAITS.iter().chain(NON_ENTRY.iter()) {
        assert!(
            declared.contains(name),
            "{name} is listed but interface.rs declares no such public trait"
        );
    }
}

/// Every `.rs` file under `dir`, as paths relative to `base`.
fn sources_under(base: &Path, dir: &Path, found: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources_under(base, &path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(
                path.strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
}

/// A `PreparedFrontiers` value is made only by the `frontiers` module, where `FrontierCache::prepared` is the one route to it: a
/// literal in any other production file would make a handover that never read its sources once.
#[test]
pub(super) fn prepared_frontiers_are_built_only_by_the_frontiers_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    sources_under(root, &root.join("src"), &mut files);
    let mut builders = Vec::new();
    for file in files {
        let test_tree = file.split('/').any(|part| {
            part == "test" || part == "tests" || part == "fixtures" || part.ends_with("_test.rs") || part == "test.rs"
        });
        if test_tree {
            continue;
        }
        let text = std::fs::read_to_string(root.join(&file)).unwrap();
        let literals = text
            .lines()
            .map(str::trim_start)
            .filter(|line| line.contains("PreparedFrontiers {"))
            .filter(|line| !line.starts_with("pub struct ") && !line.starts_with("impl "))
            .count();
        if literals > 0 {
            builders.push((file, literals));
        }
    }
    assert!(!builders.is_empty(), "the frontiers module builds them");
    for (file, literals) in builders {
        assert!(
            file.starts_with("src/application/file_dag/frontiers/"),
            "{file} builds {literals} PreparedFrontiers outside the frontiers module"
        );
    }
}

#[test]
pub(super) fn every_trait_method_has_its_own_variant() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let interface = interface_sources(root);
    let witness =
        std::fs::read_to_string(root.join("tests/source_boundary/submission/witness/mod.rs"))
            .unwrap();
    for (name, prefix) in TRAITS {
        let start = interface
            .find(&format!("pub trait {name}"))
            .unwrap_or_else(|| panic!("{name} is declared in interface.rs"));
        let body = &interface[start..];
        let body = &body[..body.find("\n}\n").expect("a trait body ends")];
        let methods = body.lines().filter(|line| line.starts_with("    fn ")).count();
        let variants = ALL
            .iter()
            .filter(|method| format!("{method:?}").starts_with(prefix))
            .count();
        assert_eq!(
            methods, variants,
            "{name} has {methods} methods and {variants} {prefix}* variants: one variant per method"
        );
    }
    for &method in ALL {
        let call = format!("witness(Method::{method:?})");
        assert_eq!(
            witness.matches(&call).count(),
            1,
            "{method:?} is witnessed by exactly one trait method"
        );
    }
    assert_eq!(
        witness.matches("witness(Method::").count(),
        ALL.len(),
        "every witnessed trait method names its own variant"
    );
}

#[test]
pub(super) fn every_entry_method_refuses_an_incomplete_extra_or_undeclared_root() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    implements_every_entry_trait(&Witness);
    assert_eq!(
        ALL.iter().collect::<BTreeSet<_>>().len(),
        ALL.len(),
        "each entry method has exactly one control"
    );
    assert_inventory(root, PROTOCOL, "DiagnosticDelivery", covered(&DELIVERIES));
    assert_inventory(root, PROTOCOL, "ObservationDemand", covered(&DEMANDS));
    assert_inventory(root, PROTOCOL, "CaptureIntent", covered(&CAPTURES));
    assert_inventory(root, PROTOCOL, "ApplicabilityRefusal", covered(&REFUSALS));
    assert_inventory(root, PROTOCOL, "ApplicabilityReason", covered(&REASONS));
    assert_inventory(root, PROTOCOL, "FileRuleIntent", covered(&every_intent()));
    let (mut refused, mut exempt) = (0, 0);
    for &method in ALL {
        match control(method) {
            Control::Refuses(refuses) => {
                refuses();
                refused += 1;
            }
            Control::AcceptsNoRoot(reason) => {
                assert!(!reason.is_empty(), "{method:?} states why it takes no root");
                exempt += 1;
            }
        }
    }
    assert_eq!(
        refused + exempt,
        ALL.len(),
        "every entry method was classified"
    );
}
