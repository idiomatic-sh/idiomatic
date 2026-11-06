//! The census of derived consumers. The owner's consumer-visible surface is read from source and
//! must equal the exhaustive `Entry` table; every acquiring entry has a positive and a negative
//! compile control; the production files outside the owner that call each entry, and those that
//! hold a leased view or handle, must equal the declared lists; every `DerivedKind` names its
//! route, lifecycle and identity controls. A new entry, consumer, holder or kind fails here.
use super::super::fixtures::{Compilation, Expectation, Probe};
use std::{collections::BTreeSet, path::Path};

mod controls;
mod entry;
mod holder;
mod kind;
mod scan;

use entry::{Authority, Class, Entry};
use holder::Holder;
use ryo_derived::domain::contract::types::DerivedKind;

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
                owner_type,
                path,
                ..
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
    assert_eq!(declared.len(), Entry::ALL.len(), "one row per owner function");
    assert_eq!(
        scan::owner_surface(&src, &production),
        declared,
        "the owner gained or lost a consumer-visible function: classify it in entry.rs"
    );

    // Every caller outside the owner is declared.
    for entry in Entry::ALL {
        let row = entry.row();
        match row.class {
            Class::Acquires(_) => {
                let found = scan::callers(&src, &production, row.call, row.with);
                let expected: BTreeSet<String> =
                    row.sites.iter().map(|site| (*site).to_owned()).collect();
                assert_eq!(found, expected, "{entry:?}: consumers of {}", row.name);
            }
            Class::Reads { .. } => assert!(row.sites.is_empty()),
            Class::Exempt(reason) => assert!(!reason.is_empty(), "{entry:?} states why"),
        }
    }

    // Every holder of a leased view or handle is declared with the authorities it names.
    let owned = |file: &str| {
        file == scan::OWNER_FILE || file.starts_with(&format!("{}/", scan::OWNER_TREE))
    };
    let holders: BTreeSet<&str> = Holder::ALL.iter().map(|holder| holder.path()).collect();
    assert_eq!(holders.len(), Holder::ALL.len(), "one row per holder");
    for file in production.iter().filter(|file| !owned(file)) {
        let text = std::fs::read_to_string(src.join(file)).unwrap();
        let named: BTreeSet<&str> = Authority::ALL
            .iter()
            .map(|authority| authority.control().0)
            .filter(|name| text.contains(name))
            .collect();
        let expected: BTreeSet<&str> = Holder::ALL
            .iter()
            .filter(|holder| holder.path() == file)
            .flat_map(|holder| holder.holds().iter().map(|authority| authority.control().0))
            .collect();
        assert_eq!(named, expected, "{file}: authorities it names");
    }

    // Every kind names a route through acquiring entries and existing controls, and every
    // acquiring entry belongs to some kind's route.
    let family = |name: &str| kind::family_file(name);
    let mut routed = BTreeSet::new();
    let mut ordinals = BTreeSet::new();
    for derived in DerivedKind::ALL {
        assert!(ordinals.insert(derived.ordinal()), "{derived:?} listed once");
        let route = kind::route(derived);
        assert!(!route.entries.is_empty(), "{derived:?} has a production route");
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
    let (raw_file, raw_cases) = kind::RAW_CASES;
    let raw = std::fs::read_to_string(src.join(raw_file)).unwrap();
    for case in raw_cases {
        assert!(raw.contains(case), "RawKeyOrder exit case {case} exists");
    }
    for entry in Entry::ALL {
        if matches!(entry.row().class, Class::Acquires(_)) {
            assert!(routed.contains(&entry), "{entry:?} belongs to a kind's route");
        }
    }
}
