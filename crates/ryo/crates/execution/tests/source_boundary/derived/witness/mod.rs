//! Opaque backing observations never grant construction or ownership of storage.
use super::super::fixtures::{Expectation, Probe};

const IMPORT: &str = r#"
use crate::application::file_dag::derived::owner::witness::service::{BackingWitness as Witness, BackingLiveness, BackingIdentity};
"#;

pub(super) fn controls(owner: &'static str) -> Vec<Probe> {
    vec![
        Probe {
            owner,
            source: format!(
                r#"
mod witness_observation {{
    {IMPORT}
    fn observe(left: &Witness, right: &Witness) -> (BackingLiveness, BackingIdentity, Witness) {{
        (left.liveness(), left.compare(right), left.clone())
    }}
    const _: fn(&Witness, &Witness) -> (BackingLiveness, BackingIdentity, Witness) = observe;
}}
"#
            ),
            expected: Expectation::Accept,
        },
        Probe {
            owner,
            source: format!(
                r#"
mod witness_private_constructor {{
    {IMPORT}
    fn forge(backing: std::sync::Weak<dyn std::any::Any>) -> Witness {{ Witness::new(backing) }}
    const _: fn(std::sync::Weak<dyn std::any::Any>) -> Witness = forge;
}}
"#
            ),
            expected: Expectation::Refuse {
                code: "E0624",
                first: "new",
                second: "private",
            },
        },
        Probe {
            owner,
            source: format!(
                r#"
mod witness_alias_constructor {{
    {IMPORT}
    fn forge(backing: std::sync::Weak<dyn std::any::Any>) -> Witness {{ Witness {{ backing }} }}
    const _: fn(std::sync::Weak<dyn std::any::Any>) -> Witness = forge;
}}
"#
            ),
            expected: Expectation::Refuse {
                code: "E0451",
                first: "backing",
                second: "private",
            },
        },
        Probe {
            owner,
            source: format!(
                r#"
mod witness_self_retarget {{
    {IMPORT}
    impl Witness {{
        fn retarget(value: Self, backing: std::sync::Weak<dyn std::any::Any>) -> Self {{ Self {{ backing, ..value }} }}
    }}
    const _: fn(Witness, std::sync::Weak<dyn std::any::Any>) -> Witness = Witness::retarget;
}}
"#
            ),
            expected: Expectation::Refuse {
                code: "E0451",
                first: "backing",
                second: "private",
            },
        },
        Probe {
            owner,
            source: format!(
                r#"
mod witness_detach_backing {{
    {IMPORT}
    fn detach(value: Witness) -> std::sync::Weak<dyn std::any::Any> {{ value.backing }}
    const _: fn(Witness) -> std::sync::Weak<dyn std::any::Any> = detach;
}}
"#
            ),
            expected: Expectation::Refuse {
                code: "E0616",
                first: "backing",
                second: "private",
            },
        },
        Probe {
            owner,
            source: format!(
                r#"
mod witness_upgrade_storage {{
    {IMPORT}
    fn upgrade(value: &Witness) -> Option<std::sync::Arc<dyn std::any::Any>> {{ value.upgrade() }}
    const _: fn(&Witness) -> Option<std::sync::Arc<dyn std::any::Any>> = upgrade;
}}
"#
            ),
            expected: Expectation::Refuse {
                code: "E0599",
                first: "upgrade",
                second: "BackingWitness",
            },
        },
        Probe {
            owner,
            source: format!(
                r#"
mod witness_clone_storage {{
    {IMPORT}
    fn detach(value: &Witness) -> std::sync::Arc<dyn std::any::Any> {{ std::sync::Arc::<dyn std::any::Any>::clone(value) }}
    const _: fn(&Witness) -> std::sync::Arc<dyn std::any::Any> = detach;
}}
"#
            ),
            expected: Expectation::Refuse {
                code: "E0308",
                first: "Arc",
                second: "BackingWitness",
            },
        },
    ]
}
