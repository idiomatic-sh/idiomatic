//! Every field or constructor an in-crate sibling could use to forge owner state is private to its
//! owner, and each authorized route stays reachable: ledger and lease, request frontier cache, read
//! proof, compiled region and instruction, relation owner.
use super::fixtures::{Compilation, Expectation, Probe};
const PROGRAM: &str = "src/application/file_dag/program.rs";
const REQUEST: &str = "src/inbound/use_cases/execution/use_case.rs";
const LEDGER_TYPES: &str = "crate::application::file_dag::derived::ledger::types";
const ADMISSION: &str = "crate::application::file_dag::program::admission";
const COMPILED: &str = "ryo_execution_domain::domain::executable_program::types";

fn refuse(
    owner: &'static str,
    source: String,
    code: &'static str,
    first: &'static str,
    second: &'static str,
) -> Probe {
    Probe {
        compilation: Compilation::Library,
        owner,
        source,
        expected: Expectation::Refuse {
            code,
            first,
            second,
        },
    }
}
fn accept(owner: &'static str, source: String) -> Probe {
    Probe {
        compilation: Compilation::Library,
        owner,
        source,
        expected: Expectation::Accept,
    }
}
pub(super) fn controls() -> Vec<Probe> {
    vec![
        // Ledger and lease: only the ledger owner counts and issues.
        accept(
            super::derived::ROOT,
            format!(
                "mod ledger_issuer {{ use {LEDGER_TYPES}::{{Backing, LeaseClass, Ledger, ResourceLease}}; use ryo_execution_domain::domain::operating::errors::OperatingError; use std::{{cell::RefCell, rc::Rc}}; fn issue(ledger: &Rc<RefCell<Ledger>>, backings: &[Backing]) -> Result<ResourceLease, OperatingError> {{ Ledger::acquire(ledger, backings, LeaseClass::Required) }} const _: fn(&Rc<RefCell<Ledger>>, &[Backing]) -> Result<ResourceLease, OperatingError> = issue; }}"
            ),
        ),
        refuse(
            PROGRAM,
            format!(
                "mod forge_lease {{ use {LEDGER_TYPES}::{{ByteCount, LeaseClass, Ledger, ResourceLease}}; use std::{{cell::RefCell, rc::Rc}}; fn forge(ledger: Rc<RefCell<Ledger>>) {{ let _ = ResourceLease {{ ledger, keys: Vec::new(), class: LeaseClass::Required, metadata_bytes: ByteCount(0) }}; }} const _: fn(Rc<RefCell<Ledger>>) = forge; }}"
            ),
            "E0451",
            "ResourceLease",
            "private",
        ),
        refuse(
            PROGRAM,
            format!(
                "mod read_lease {{ use {LEDGER_TYPES}::ResourceLease; fn read(lease: &ResourceLease) {{ let _ = &lease.keys; }} const _: fn(&ResourceLease) = read; }}"
            ),
            "E0616",
            "keys",
            "private",
        ),
        refuse(
            PROGRAM,
            format!(
                "mod retarget_ledger {{ use {LEDGER_TYPES}::{{ByteCount, Ledger}}; fn retarget(ledger: &mut Ledger) {{ ledger.required_bytes = ByteCount(0); }} const _: fn(&mut Ledger) = retarget; }}"
            ),
            "E0616",
            "required_bytes",
            "private",
        ),
        refuse(
            PROGRAM,
            format!(
                "mod issue_lease {{ use {LEDGER_TYPES}::{{Backing, LeaseClass, Ledger}}; use std::{{cell::RefCell, rc::Rc}}; fn issue(ledger: &Rc<RefCell<Ledger>>, backings: &[Backing]) {{ let _ = Ledger::acquire(ledger, backings, LeaseClass::Required); }} const _: fn(&Rc<RefCell<Ledger>>, &[Backing]) = issue; }}"
            ),
            "E0624",
            "acquire",
            "private",
        ),
        // Request frontier cache: no Default and no tuple constructor; the request issues its one.
        accept(
            REQUEST,
            "mod issued_frontier_cache { const _: fn() -> crate::application::file_dag::frontiers::types::FrontierCache = crate::application::file_dag::frontiers::types::FrontierCache::new; }".into(),
        ),
        refuse(
            PROGRAM,
            "mod defaulted_frontier_cache { fn second() { let _ = crate::application::file_dag::frontiers::types::FrontierCache::default(); } const _: fn() = second; }".into(),
            "E0599",
            "default",
            "FrontierCache",
        ),
        refuse(
            PROGRAM,
            "mod built_frontier_cache { fn second() { let _ = crate::application::file_dag::frontiers::types::FrontierCache(std::sync::Mutex::default()); } const _: fn() = second; }".into(),
            "E0603",
            "FrontierCache",
            "private",
        ),
        // Read proof: readiness is admission's to know; callers pass the proof.
        accept(
            PROGRAM,
            format!(
                "mod read_proof_route {{ use {ADMISSION}::{{AdmittedInput, Partition, ProducerRoot, ReadProof}}; use crate::application::file_dag::relations::{{RelationOwner, admission::service::BorrowedInput, handle::service::PublishedInput}}; use ryo_execution_domain::domain::operating::errors::OperatingError; use std::rc::Rc; fn whole() -> ReadProof {{ ReadProof::complete(Partition::RequestInvariant) }} const _: fn() -> ReadProof = whole; fn admit(owner: Rc<RelationOwner>, publication: PublishedInput, admitted: &BorrowedInput<'_>, root: ProducerRoot, read: ReadProof) -> Result<AdmittedInput, OperatingError> {{ AdmittedInput::from_publication(owner, publication, admitted, root, read) }} const _: for<'a> fn(Rc<RelationOwner>, PublishedInput, &BorrowedInput<'a>, ProducerRoot, ReadProof) -> Result<AdmittedInput, OperatingError> = admit; }}"
            ),
        ),
        refuse(
            PROGRAM,
            format!("mod named_readiness {{ use {ADMISSION}::Readiness; }}"),
            "E0603",
            "Readiness",
            "private",
        ),
        refuse(
            PROGRAM,
            format!(
                "mod read_readiness {{ use {ADMISSION}::ReadProof; fn read(proof: &ReadProof) {{ let _ = &proof.readiness; }} const _: fn(&ReadProof) = read; }}"
            ),
            "E0616",
            "readiness",
            "private",
        ),
        refuse(
            PROGRAM,
            format!(
                "mod retarget_read_proof {{ use {ADMISSION}::{{Partition, ReadProof}}; fn retarget(value: ReadProof, partition: Partition) -> ReadProof {{ ReadProof {{ partition, ..value }} }} const _: fn(ReadProof, Partition) -> ReadProof = retarget; }}"
            ),
            "E0451",
            "partition",
            "private",
        ),
        // Compiled program: only `compile` builds a region or an instruction.
        accept(
            PROGRAM,
            format!(
                "mod compiled_accessors {{ use {COMPILED}::instruction::value::Instruction; use {COMPILED}::region::value::Region; fn count(region: &Region) -> usize {{ region.instructions().len() + region.instructions().iter().map(|instruction: &Instruction| instruction.logical().0).sum::<usize>() }} const _: fn(&Region) -> usize = count; }}"
            ),
        ),
        refuse(
            PROGRAM,
            format!(
                "mod read_region {{ use {COMPILED}::region::value::Region; fn read(region: &Region) {{ let _ = &region.instructions; }} const _: fn(&Region) = read; }}"
            ),
            "E0616",
            "instructions",
            "private",
        ),
        refuse(
            PROGRAM,
            format!(
                "mod retarget_region {{ use {COMPILED}::region::value::Region; use ryo_execution_domain::domain::coverage::types::RuleIndex; fn retarget(value: Region, producer: RuleIndex) -> Region {{ Region {{ producer, ..value }} }} const _: fn(Region, RuleIndex) -> Region = retarget; }}"
            ),
            "E0451",
            "producer",
            "private",
        ),
        refuse(
            PROGRAM,
            format!(
                "mod read_instruction {{ use {COMPILED}::instruction::value::Instruction; fn read(instruction: &Instruction) {{ let _ = &instruction.operation; }} const _: fn(&Instruction) = read; }}"
            ),
            "E0616",
            "operation",
            "private",
        ),
        refuse(
            PROGRAM,
            format!(
                "mod retarget_instruction {{ use {COMPILED}::instruction::value::Instruction; use ryo_execution_domain::domain::coverage::types::RuleIndex; fn retarget(instruction: &mut Instruction, producer: RuleIndex) {{ instruction.producer = producer; }} const _: fn(&mut Instruction, RuleIndex) = retarget; }}"
            ),
            "E0616",
            "producer",
            "private",
        ),
        // Relation owner: its derived owner is read through the accessor alone.
        accept(
            PROGRAM,
            "mod relation_owner_accessor { use crate::application::file_dag::relations::RelationOwner; fn derived(owner: &RelationOwner) -> &crate::application::file_dag::derived::owner::DerivedOwner { owner.derived() } const _: fn(&RelationOwner) -> &crate::application::file_dag::derived::owner::DerivedOwner = derived; }".into(),
        ),
        refuse(
            PROGRAM,
            "mod relation_owner_field { use crate::application::file_dag::relations::RelationOwner; fn read(owner: &RelationOwner) { let _ = &owner.derived; } const _: fn(&RelationOwner) = read; }".into(),
            "E0616",
            "derived",
            "private",
        ),
    ]
}
