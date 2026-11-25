//! Closed programs can borrow the same immutable inputs after the last file.
use super::*;

pub(super) enum Inputs {
    Withheld(session::Reported),
    Ready {
        carried: finish::population::Inputs,
        incomplete: BTreeMap<String, Vec<Option<u32>>>,
        unknowns: Vec<(String, String)>,
        skipped: Vec<String>,
    },
}

pub(super) fn accepted_files<'a>(files: &'a [String], rejected: &[String]) -> std::borrow::Cow<'a, [String]> {
    if rejected.is_empty() { return std::borrow::Cow::Borrowed(files); }
    std::borrow::Cow::Owned(files.iter().filter(|p| rejected.binary_search(p).is_err()).cloned().collect())
}

pub(super) fn inputs(
    run: &finish::Run,
    contract: &session::completeness::Contract,
    unavailable: &HashMap<usize, BTreeSet<(String, u32)>>,
    pairs: &[usize],
    inventory: &[String],
    files: &[String],
    cut: Option<&finish::partition::Partition>,
    p: Option<&Population>,
    answers: &ReportSlots,
    frontier: impl FnOnce(&[usize]) -> Result<finish::population::Inputs, String>,
) -> Result<Inputs, String> {
    let mut carried = BTreeMap::new();
    let mut incomplete = BTreeMap::<String, Vec<Option<u32>>>::new();
    let mut unknowns = Vec::new();
    let mut skipped = Vec::new();
    if let Some(p) = p {
        use ryo_execution_domain::domain::{
            coverage::{
                service::{repository, unknown_message},
                types::{CoverageScope, PartialPlace},
            },
            trust::{service::matters, types::Bearing},
        };
        for partial in &p.partials {
            if !contract
                .needs
                .reads
                .iter()
                .any(|r| matters(r, partial) == Bearing::Matters)
            {
                continue;
            }
            let file = match partial.at {
                PartialPlace::Repository => None,
                PartialPlace::File(f) => Some(f.0),
            };
            let path = file
                .map(|f| l::FilePath(run.paths[f as usize].as_str().into()));
            let scope = path
                .as_ref()
                .map_or(CoverageScope::Repository, CoverageScope::File);
            let reason = unknown_message(partial, scope).0.into_string();
            unknowns
                .push((path.unwrap_or_else(repository).0.into_string(), reason));
            incomplete
                .entry(relations::service::name(partial.relation).0.to_owned())
                .or_default()
                .push(file);
        }
    }
    if files.is_empty() && !inventory.is_empty() && contract.frontier {
        let mut withheld = session::Reported::skipped(inventory);
        withheld.extend_unknowns(unknowns);
        return Ok(Inputs::Withheld(withheld));
    }
    if let Some(cut) = cut {
        for &slot in pairs {
            if let Some(partials) = unavailable.get(&slot) {
                for (relation, file) in partials {
                    incomplete
                        .entry(relation.clone())
                        .or_default()
                        .push(Some(*file));
                }
            }
            let answer = answers[slot].as_ref().map_err(Clone::clone)?;
            unknowns.extend(answer.unknowns.iter().cloned());
            skipped.extend(answer.rejected_syntax.iter().cloned());
        }
        skipped.sort_unstable();
        skipped.dedup();
        if !files.is_empty() && files.iter().all(|p| skipped.binary_search(p).is_ok()) {
            unknowns.extend(session::Reported::skipped(&skipped).unknowns.iter().cloned());
            return Ok(Inputs::Withheld(session::Reported::new(Vec::new(), unknowns, skipped)));
        }
        carried = frontier(&cut.inputs)?;
        if carried.len() != cut.inputs.len() {
            return Err("source frontier has not been closed".into());
        }
    }
    Ok(Inputs::Ready { carried, incomplete, unknowns, skipped })
}




