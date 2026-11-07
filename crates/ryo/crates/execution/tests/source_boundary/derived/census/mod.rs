//! The census of derived consumers. The owner's consumer-visible surface is read from source and
//! must equal the exhaustive `Entry` table; every acquiring entry has a positive and a negative
//! compile control; every `DerivedKind` names its route, lifecycle and identity controls. A new
//! entry or kind fails here. Which files outside the owner call an entry or hold a leased view or
//! handle is `idiomatic.ryo/derived-view-acquisition`'s `allowed` list.
use super::super::fixtures::{Compilation, Expectation, Probe};
use std::{collections::BTreeSet, path::Path};

mod controls;
mod entry;
mod kind;
mod scan;

use entry::{Authority, Class, Entry};
use ryo_derived::domain::request::types::DerivedKind;

fn refuse(owner: &'static str, negative: &controls::Negative) -> Probe {
    Probe {
        compilation: Compilation::Library,
        owner,
        source: negative.source.into(),
        expected: Expectation::Refuse {
            code: negative.code,
            first: negative.first,
            second: negative.second,
        },
    }
}
/// The compile controls: every authorized acquisition and read, every bypass, every forgery.
pub(super) fn controls(root: &'static str) -> Vec<Probe> {
    let mut positive = String::new();
    let mut probes = Vec::new();
    for entry in Entry::ALL {
        match entry.row().class {
            Class::Acquires(control) => {
                positive.push_str(control.positive);
                probes.push(refuse(root, &control.negative));
            }
            Class::Reads {
                owner_type, path, ..
            } => positive.push_str(&controls::read(path, owner_type, entry.row().name)),
            Class::Exempt(_) => {}
        }
    }
    for authority in Authority::ALL {
        probes.push(refuse(root, authority.control().1));
    }
    probes.insert(
        0,
        Probe {
            compilation: Compilation::Library,
            owner: root,
            source: positive,
            expected: Expectation::Accept,
        },
    );
    probes
}

#[test]
fn every_derived_consumer_is_censused_and_every_kind_names_its_controls() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let production = scan::production(&src);

    // The owner surface equals the exhaustive table.
    let declared: BTreeSet<(String, String)> = Entry::ALL
        .iter()
        .map(|entry| {
            let row = entry.row();
            (row.file.to_owned(), row.name.to_owned())
        })
        .collect();
    assert_eq!(
        declared.len(),
        Entry::ALL.len(),
        "one row per owner function"
    );
    let surface = scan::owner_surface(&src, &production);
    assert_eq!(
        (
            surface.difference(&declared).collect::<Vec<_>>(),
            declared.difference(&surface).collect::<Vec<_>>()
        ),
        (vec![], vec![]),
        "the owner gained (left) or lost (right) a consumer-visible function: classify it in entry.rs"
    );

    // Every read is controlled by its own authority, and every exemption states why.
    for entry in Entry::ALL {
        let row = entry.row();
        match row.class {
            Class::Acquires(_) => {}
            Class::Reads {
                authority,
                owner_type,
                ..
            } => {
                assert_eq!(
                    authority.control().0,
                    owner_type,
                    "{entry:?}: a read is controlled by the forgery control of its own authority"
                );
            }
            Class::Exempt(reason) => assert!(!reason.is_empty(), "{entry:?} states why"),
        }
    }

    // Every kind names a route through acquiring entries and existing controls, and every
    // acquiring entry belongs to some kind's route.
    let family = |name: &str| kind::family_file(name);
    let mut routed = BTreeSet::new();
    let mut ordinals = BTreeSet::new();
    for derived in DerivedKind::ALL.iter().copied() {
        assert!(
            ordinals.insert(derived.ordinal()),
            "{derived:?} listed once"
        );
        let route = kind::route(derived);
        assert!(
            !route.entries.is_empty(),
            "{derived:?} has a production route"
        );
        for entry in route.entries {
            assert!(
                matches!(entry.row().class, Class::Acquires(_)),
                "{derived:?}: {entry:?} acquires through the owner"
            );
            routed.insert(*entry);
        }
        for (file, test) in route.lifecycle.iter().chain([&route.identity]) {
            assert!(
                scan::declares(&src, &family(file), test),
                "{derived:?}: {test} is declared in {file}"
            );
        }
    }
    for entry in Entry::ALL {
        if matches!(entry.row().class, Class::Acquires(_)) {
            assert!(
                routed.contains(&entry),
                "{entry:?} belongs to a kind's route"
            );
        }
    }
}
