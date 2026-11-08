//! The `placement` service: free functions named by verb over the domain's types, with no IO and no
//! ports. The scopes of the rules are compiled once; each file then costs one match against every
//! glob at once, and a few bit operations.

use std::collections::HashMap;

use super::types::{
    Bit, BitMask, GlobKey, GlobOwner, GlobSide, Governing, Mask, PlacedScopes, RuleScope,
    ScopePlacement, WordIndex,
};
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::Glob;
use ryo_values::domain::files::types::{Hits, MatchCandidate};
use ryo_values::domain::runs::types::RuleNumber;

/// The word and the bit of a rule in a mask.
fn bit_of(rule: RuleNumber) -> Bit {
    Bit {
        word: WordIndex(usize::try_from(rule.0 / 64).unwrap_or(usize::MAX)),
        mask: BitMask(1 << (rule.0 % 64)),
    }
}

/// Sets a rule's bit.
fn set(mask: &mut Mask, rule: RuleNumber) {
    let bit = bit_of(rule);
    if let Some(held) = mask.0.get_mut(bit.word.0) {
        *held |= bit.mask.0;
    }
}

/// Clears a rule's bit.
fn clear(mask: &mut Mask, rule: RuleNumber) {
    let bit = bit_of(rule);
    if let Some(held) = mask.0.get_mut(bit.word.0) {
        *held &= !bit.mask.0;
    }
}

/// Clears one rule from a file's temporary applicability mask without changing its path scope.
pub(crate) fn unplace(mask: &mut Mask, rule: RuleNumber) {
    clear(mask, rule);
}

/// Decides the scopes of the rules, the first scope being rule zero: the globs in one list, whose
/// each is whose, and the rules that include nothing.
#[must_use]
pub(crate) fn decide(scopes: &[RuleScope]) -> ScopePlacement {
    let mut globs: Vec<Glob> = Vec::new();
    let mut owners: Vec<Vec<GlobOwner>> = Vec::new();
    let mut found: HashMap<GlobKey, usize> = HashMap::new();
    let mut always = Mask(vec![0; scopes.len() / 64 + 1]);
    for (place, scope) in scopes.iter().enumerate() {
        let rule = RuleNumber(u32::try_from(place).unwrap_or(u32::MAX));
        if scope.include.is_empty() {
            set(&mut always, rule);
        }
        for (side, listed) in [
            (GlobSide::Include, &scope.include),
            (GlobSide::Exclude, &scope.exclude),
        ] {
            for glob in listed {
                // A glob is made ready once, whoever writes it and however many times.
                let at = *found.entry(GlobKey(glob.0.clone())).or_insert_with(|| {
                    globs.push(glob.clone());
                    owners.push(Vec::new());
                    globs.len() - 1
                });
                let each = GlobOwner { rule, side };
                if let Some(held) = owners.get_mut(at)
                    && !held.contains(&each)
                {
                    held.push(each);
                }
            }
        }
    }
    ScopePlacement {
        globs,
        owners,
        always,
    }
}

/// Says which rules govern the file at `path`: the rules that include nothing, and the rules whose
/// included globs match, less the rules whose excluded globs match. `mask` and `hits` are the
/// worker's own and keep their room.
pub(crate) fn place(placed: &PlacedScopes, path: &FilePath, mask: &mut Mask, hits: &mut Hits) {
    mask.0.clone_from(&placed.placement.always.0);
    placed.matcher.matches.find(MatchCandidate(&path.0), hits);
    for owner in hits
        .0
        .iter()
        .filter_map(|glob| placed.placement.owners.get(*glob))
        .flatten()
        .filter(|owner| owner.side == GlobSide::Include)
    {
        set(mask, owner.rule);
    }
    for owner in hits
        .0
        .iter()
        .filter_map(|glob| placed.placement.owners.get(*glob))
        .flatten()
        .filter(|owner| owner.side == GlobSide::Exclude)
    {
        clear(mask, owner.rule);
    }
}

/// Whether the rule governs the file the mask was made for.
#[must_use]
pub(crate) fn governs(mask: &Mask, rule: RuleNumber) -> Governing {
    let bit = bit_of(rule);
    if mask
        .0
        .get(bit.word.0)
        .is_some_and(|held| held & bit.mask.0 != 0)
    {
        Governing::Governs
    } else {
        Governing::DoesNotGovern
    }
}

/// Whether any rule governs the file the mask was made for.
#[must_use]
pub(crate) fn governed(mask: &Mask) -> Governing {
    if mask.0.iter().any(|word| *word != 0) {
        Governing::Governs
    } else {
        Governing::DoesNotGovern
    }
}
