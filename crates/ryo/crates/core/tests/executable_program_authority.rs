//! Compile the production authority declarations in their actual module positions.
//! Negative clients are siblings in the same crate, so a pub(crate) hiding test cannot pass.
use std::{fs, path::Path, process::Command};

fn declaration(source: &str, marker: &str) -> String {
    let start = source.find(marker).expect("production declaration");
    let opening = start + source[start..].find('{').expect("declaration body");
    let mut depth = 0usize;
    for (offset, byte) in source[opening..].bytes().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return source[start..=opening + offset].to_owned();
                }
            }
            _ => {}
        }
    }
    panic!("unclosed production declaration: {marker}");
}

fn compile(directory: &Path, name: &str, source: &str) -> std::process::Output {
    let input = directory.join(format!("{name}.rs"));
    fs::write(&input, source).unwrap();
    Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .args(["--edition=2024", "--crate-type=lib", "--emit=metadata"])
        .arg(&input)
        .arg("-o")
        .arg(directory.join(format!("{name}.rmeta")))
        .output()
        .unwrap()
}

fn passes(directory: &Path, name: &str, source: &str) {
    let result = compile(directory, name, source);
    assert!(
        result.status.success(),
        "authorized {name} failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

fn refuses(directory: &Path, name: &str, source: &str, expected: &str) {
    let result = compile(directory, name, source);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(!result.status.success(), "unauthorized {name} compiled");
    assert!(
        stderr.contains(expected),
        "unauthorized {name} failed for a different reason: {stderr}"
    );
}

fn program_source(client: &str) -> String {
    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/domain/executable_program/types.rs"),
    )
    .unwrap();
    let data = declaration(&source, "pub(crate) struct ExecutableProgram");
    let accessors = declaration(&source, "impl ExecutableProgram");
    r#"
#![allow(dead_code, unused_imports)]
mod domain {
  pub mod executable_program {
    pub mod types {
      pub struct Region;
      pub struct RuleProgram;
      pub struct RegionId(pub usize);
      pub struct OperatingError;
      __DECLARATION__
      __ACCESSORS__
    }
    pub mod service {
      use super::types::*;
      pub fn compile() -> ExecutableProgram {
        ExecutableProgram {
          regions: Box::new([]), rules: Box::new([]),
          logical_regions: Box::new([]), observation_regions: Box::new([]),
          original_instruction_count: 0, fused_instruction_count: 0,
        }
      }
    }
  }
}
mod application {
  pub mod facade {
    use crate::domain::executable_program::types::ExecutableProgram;
    pub fn region_count(program: &ExecutableProgram) -> usize { program.regions().len() }
  }
  pub mod rogue {
    use crate::domain::executable_program::types::ExecutableProgram;
    __CLIENT__
  }
}
"#
    .replace("__DECLARATION__", &data)
    .replace("__ACCESSORS__", &accessors)
    .replace("__CLIENT__", client)
}

#[test]
fn compiled_program_state_is_constructed_only_by_its_domain_owner() {
    let directory = tempfile::tempdir().unwrap();
    passes(
        directory.path(),
        "compiled_program_owner",
        &program_source(""),
    );
    refuses(
        directory.path(),
        "sibling_constructs_program",
        &program_source(
            "pub fn forge() -> ExecutableProgram { ExecutableProgram {
              regions: Box::new([]), rules: Box::new([]),
              logical_regions: Box::new([]), observation_regions: Box::new([]),
              original_instruction_count: 0, fused_instruction_count: 0,
            } }",
        ),
        "E0451",
    );
    refuses(
        directory.path(),
        "sibling_mutates_program",
        &program_source(
            "pub fn forge(program: &mut ExecutableProgram) {
              program.regions = Box::new([]);
            }",
        ),
        "E0616",
    );
}

fn permit_source(client: &str) -> String {
    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/application/file_dag/program/admission.rs"),
    )
    .unwrap();
    let permit = declaration(&source, "pub(crate) struct KernelPermit");
    let physical = declaration(&source, "pub(crate) struct PermittedPhysical");
    let owner = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/application/file_dag/relations.rs"),
    )
    .unwrap();
    let lease = declaration(
        &owner,
        "pub(in crate::application::file_dag) struct GenerationLease",
    );
    let lease_methods = declaration(&owner, "impl GenerationLease");
    r#"
#![allow(dead_code, unused_imports)]
extern crate self as ryo_operators;
pub mod domain {
  pub mod executable_program {
    pub mod types {
      pub struct ExecutableProgram;
      pub struct RegionId(pub usize);
    }
    pub mod physical { pub struct PhysicalAuthorization; }
  }
  pub mod contract { pub mod types {
    pub struct OperatorCall<'a>(pub std::marker::PhantomData<&'a ()>);
  }}
}
pub mod application { pub mod file_dag {
  pub mod relations {
    __LEASE__
    __LEASE_METHODS__
    pub fn owner_revocation() -> bool {
      let lease = GenerationLease::new();
      let before = lease.is_live();
      lease.invalidate();
      before && !lease.is_live()
    }
  }
  pub mod program {
    pub mod admission {
      use std::sync::Arc;
      use crate::domain::executable_program::types::{ExecutableProgram, RegionId};
      __PERMIT__
      __PHYSICAL__
      pub fn issue<'a>(
        program: &'a ExecutableProgram, generation: &'a Arc<()>,
        live: &'a Arc<super::super::relations::GenerationLease>,
      ) -> KernelPermit<'a> {
        KernelPermit { program, region: RegionId(0), instruction: 0,
                       generation, live, ordered: false }
      }
      pub fn physical<'p, 'm, 's>(
        permit: &'p KernelPermit<'s>,
        meaning: crate::domain::contract::types::OperatorCall<'m>,
      ) -> PermittedPhysical<'p, 'm, 's> {
        PermittedPhysical {
          permit, meaning,
          authority: crate::domain::executable_program::physical::PhysicalAuthorization,
        }
      }
    }
    pub mod rogue {
      use std::sync::Arc;
      use crate::domain::executable_program::types::{ExecutableProgram, RegionId};
      use super::super::relations::GenerationLease;
      use super::admission::{KernelPermit, PermittedPhysical};
      __CLIENT__
    }
  }
}}
"#
    .replace("__LEASE__", &lease)
    .replace("__LEASE_METHODS__", &lease_methods)
    .replace("__PERMIT__", &permit)
    .replace("__PHYSICAL__", &physical)
    .replace("__CLIENT__", client)
}

#[test]
fn physical_permits_are_minted_only_by_admission() {
    let directory = tempfile::tempdir().unwrap();
    passes(
        directory.path(),
        "admission_mints_permit",
        &permit_source(""),
    );
    refuses(
        directory.path(),
        "sibling_mints_permit",
        &permit_source(
            "pub fn forge<'a>(
              program: &'a ExecutableProgram, generation: &'a Arc<()>,
              live: &'a Arc<GenerationLease>,
            ) -> KernelPermit<'a> {
              KernelPermit { program, region: RegionId(0), instruction: 0,
                             generation, live, ordered: false }
            }",
        ),
        "E0451",
    );
    refuses(
        directory.path(),
        "sibling_dispatches_physical",
        &permit_source(
            "pub fn forge<'p, 'm, 's>(
              permit: &'p KernelPermit<'s>,
              meaning: crate::domain::contract::types::OperatorCall<'m>,
            ) -> PermittedPhysical<'p, 'm, 's> {
              PermittedPhysical {
                permit, meaning,
                authority: crate::domain::executable_program::physical::PhysicalAuthorization,
              }
            }",
        ),
        "E0451",
    );
}

#[test]
fn generation_lease_revocation_is_owner_only() {
    let directory = tempfile::tempdir().unwrap();
    passes(
        directory.path(),
        "owner_revokes_generation",
        &permit_source(""),
    );
    refuses(
        directory.path(),
        "sibling_constructs_generation",
        &permit_source(
            "pub fn forge() -> GenerationLease {
              GenerationLease { live: std::sync::atomic::AtomicBool::new(true) }
            }",
        ),
        "E0451",
    );
    refuses(
        directory.path(),
        "sibling_calls_generation_factory",
        &permit_source("pub fn forge() -> GenerationLease { GenerationLease::new() }"),
        "E0624",
    );
    refuses(
        directory.path(),
        "sibling_revives_generation",
        &permit_source(
            "pub fn revive(lease: &GenerationLease) {
              lease.live.store(true, std::sync::atomic::Ordering::Release);
            }",
        ),
        "E0616",
    );
    refuses(
        directory.path(),
        "sibling_invalidates_generation",
        &permit_source("pub fn revoke(lease: &GenerationLease) { lease.invalidate(); }"),
        "E0624",
    );
}
