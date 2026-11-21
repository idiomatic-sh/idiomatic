//! `held` over costs written by hand: a pack holds what its manifest declares when no site is above its ceiling and every site
//! above linear with sorts is on its list, a module's statement once for the pack and a statement of a rule's own query for
//! that rule alone. A rule that only reaches a listed module's statement holds; a second product does not; a listed site that
//! rose, that no rule reaches or that sits at linear with sorts breaks the list.

use super::super::service::held;
use super::super::types::{
    CostBreach, CostFinding, CostShape, ListedSite, PackCeiling, PlanCost, PlanCostClass, RuleCost,
    SiteHome, SiteOrigin, SiteOwner,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::query_tokens::types::{QuerySourceId, StepSpelling};
use ryo_values::domain::runs::types::RuleId;
use std::borrow::Cow;

/// A finding of a rule's cost: its document, its statement, which kind of document writes it, and its class.
#[derive(Clone, Copy)]
struct Found(&'static str, &'static str, SiteHome, PlanCostClass);

/// The product of the pairing module, which every rule that imports it reaches.
const PAIRS_OF_THE_MODULE: Found = Found(
    "pack/pairing",
    "pairs",
    SiteHome::Module,
    PlanCostClass::QuadraticInFile,
);
/// What a rule that only imports the module finds.
const IMPORTING: &[Found] = &[PAIRS_OF_THE_MODULE];
/// What a rule that imports the module and writes a second product of its own finds.
const IMPORTING_AND_PAIRING: &[Found] = &[
    PAIRS_OF_THE_MODULE,
    Found(
        "rules/C",
        "more",
        SiteHome::Query,
        PlanCostClass::QuadraticInFile,
    ),
];
/// What a rule with a product of its own, named `pairs`, finds when its query is compiled under the name every bare query has.
const OWN_PRODUCT: &[Found] = &[Found(
    "<query>",
    "pairs",
    SiteHome::Query,
    PlanCostClass::QuadraticInFile,
)];
/// What a rule whose one statement above linear sorts finds.
const SORTING: &[Found] = &[Found(
    "rules/S",
    "last",
    SiteHome::Query,
    PlanCostClass::LinearWithSorts,
)];

/// A rule of a pack: its id and what its query's cost finds.
#[derive(Clone, Copy)]
struct Rule(&'static str, &'static [Found]);

/// An entry of a pack's list: the rule that owns it, or none for the pack's, then its document, its statement and its class.
#[derive(Clone, Copy)]
struct Listed(
    Option<&'static str>,
    &'static str,
    &'static str,
    PlanCostClass,
);

/// What a row asks: whether these rules hold this ceiling and this list.
#[derive(Clone, Copy)]
struct Ask {
    /// The pack's rules.
    rules: &'static [Rule],
    /// The ceiling its manifest declares.
    ceiling: PlanCostClass,
    /// The list its manifest declares.
    listed: &'static [Listed],
}

type Input = Ask;
type Value = Cow<'static, str>;
type Failure = std::convert::Infallible;

/// The module's product, listed for the pack at the class it has.
const PAIRS: Listed = Listed(
    None,
    "pack/pairing",
    "pairs",
    PlanCostClass::QuadraticInFile,
);

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("rules_that_only_import_a_listed_module_hold"),
        word: Word::Succeeds,
        input: || Ask {
            rules: &[Rule("A", IMPORTING), Rule("B", IMPORTING)],
            ceiling: PlanCostClass::QuadraticInFile,
            listed: &[PAIRS],
        },
        expect: Expect::Value(Cow::Borrowed("held")),
    },
    Case {
        name: CaseName("a_second_product_in_a_rule_that_reaches_a_listed_one_is_unlisted"),
        word: Word::Succeeds,
        input: || Ask {
            rules: &[Rule("A", IMPORTING), Rule("C", IMPORTING_AND_PAIRING)],
            ceiling: PlanCostClass::QuadraticInFile,
            listed: &[PAIRS],
        },
        expect: Expect::Value(Cow::Borrowed("C reaches the unlisted rules/C::more")),
    },
    Case {
        name: CaseName("a_site_above_the_ceiling_breaks_it_though_it_is_listed"),
        word: Word::Succeeds,
        input: || Ask {
            rules: &[Rule("A", IMPORTING)],
            ceiling: PlanCostClass::QuadraticInGroup,
            listed: &[PAIRS],
        },
        expect: Expect::Value(Cow::Borrowed(
            "A is above the ceiling at pack/pairing::pairs",
        )),
    },
    Case {
        name: CaseName("a_listed_site_reached_above_the_class_it_is_listed_at_rose"),
        word: Word::Succeeds,
        input: || Ask {
            rules: &[Rule("A", IMPORTING)],
            ceiling: PlanCostClass::QuadraticInFile,
            listed: &[Listed(
                None,
                "pack/pairing",
                "pairs",
                PlanCostClass::QuadraticInGroup,
            )],
        },
        expect: Expect::Value(Cow::Borrowed(
            "the pack's pack/pairing::pairs at QuadraticInGroup rose",
        )),
    },
    Case {
        name: CaseName("a_listed_site_no_rule_reaches_is_stale"),
        word: Word::Succeeds,
        input: || Ask {
            rules: &[Rule("A", IMPORTING)],
            ceiling: PlanCostClass::QuadraticInFile,
            listed: &[
                PAIRS,
                Listed(None, "pack/pairing", "gone", PlanCostClass::QuadraticInFile),
            ],
        },
        expect: Expect::Value(Cow::Borrowed(
            "the pack's pack/pairing::gone at QuadraticInFile is stale",
        )),
    },
    Case {
        name: CaseName("a_site_at_linear_with_sorts_is_never_listed"),
        word: Word::Succeeds,
        input: || Ask {
            rules: &[Rule("S", SORTING)],
            ceiling: PlanCostClass::LinearWithSorts,
            listed: &[Listed(
                Some("S"),
                "rules/S",
                "last",
                PlanCostClass::LinearWithSorts,
            )],
        },
        expect: Expect::Value(Cow::Borrowed(
            "S's rules/S::last at LinearWithSorts is stale",
        )),
    },
    Case {
        name: CaseName("a_site_at_linear_with_sorts_asks_nothing"),
        word: Word::Succeeds,
        input: || Ask {
            rules: &[Rule("S", SORTING)],
            ceiling: PlanCostClass::LinearWithSorts,
            listed: &[],
        },
        expect: Expect::Value(Cow::Borrowed("held")),
    },
    Case {
        name: CaseName("two_rules_under_one_document_name_with_a_product_each_are_two_sites"),
        word: Word::Succeeds,
        input: || Ask {
            rules: &[Rule("A", OWN_PRODUCT), Rule("B", OWN_PRODUCT)],
            ceiling: PlanCostClass::QuadraticInFile,
            listed: &[Listed(
                Some("A"),
                "<query>",
                "pairs",
                PlanCostClass::QuadraticInFile,
            )],
        },
        expect: Expect::Value(Cow::Borrowed("B reaches the unlisted <query>::pairs")),
    },
    Case {
        name: CaseName("each_rules_own_product_listed_for_it_holds"),
        word: Word::Succeeds,
        input: || Ask {
            rules: &[Rule("A", OWN_PRODUCT), Rule("B", OWN_PRODUCT)],
            ceiling: PlanCostClass::QuadraticInFile,
            listed: &[
                Listed(
                    Some("A"),
                    "<query>",
                    "pairs",
                    PlanCostClass::QuadraticInFile,
                ),
                Listed(
                    Some("B"),
                    "<query>",
                    "pairs",
                    PlanCostClass::QuadraticInFile,
                ),
            ],
        },
        expect: Expect::Value(Cow::Borrowed("held")),
    },
    Case {
        name: CaseName("a_rules_own_product_is_not_held_by_a_pack_entry_of_its_name"),
        word: Word::Succeeds,
        input: || Ask {
            rules: &[Rule("A", OWN_PRODUCT)],
            ceiling: PlanCostClass::QuadraticInFile,
            listed: &[Listed(
                None,
                "<query>",
                "pairs",
                PlanCostClass::QuadraticInFile,
            )],
        },
        expect: Expect::Value(Cow::Borrowed(
            "A reaches the unlisted <query>::pairs; the pack's <query>::pairs at QuadraticInFile is stale",
        )),
    },
];

/// A site on one line: its document and its statement.
fn site(site: &SiteOrigin) -> String {
    format!("{}::{}", site.document.0, site.statement)
}

/// An entry of a list on one line: whose it is, its site and its class.
fn entry(listed: &ListedSite) -> String {
    let owner = match &listed.owner {
        SiteOwner::Pack => "the pack's".to_owned(),
        SiteOwner::Rule(rule) => format!("{}'s", rule.0),
    };
    format!("{owner} {} at {:?}", site(&listed.site), listed.class)
}

/// The greatest class of what a rule finds.
fn found_class(found: &[Found]) -> PlanCostClass {
    found
        .iter()
        .map(|found| found.3)
        .max()
        .unwrap_or(PlanCostClass::Linear)
}

fn subject(ask: Input) -> Result<Value, Failure> {
    let costs = ask
        .rules
        .iter()
        .map(|rule| {
            let findings = rule
                .1
                .iter()
                .map(|found| CostFinding {
                    site: SiteOrigin {
                        document: QuerySourceId(found.0.into()),
                        statement: StepSpelling(found.1.into()),
                    },
                    home: found.2,
                    shape: CostShape::Product,
                    class: found.3,
                })
                .collect::<Vec<_>>();
            RuleCost {
                rule: RuleId(rule.0.into()),
                cost: PlanCost {
                    class: found_class(rule.1),
                    findings,
                },
            }
        })
        .collect::<Vec<_>>();
    let pack = PackCeiling {
        ceiling: ask.ceiling,
        listed: ask
            .listed
            .iter()
            .map(|listed| ListedSite {
                site: SiteOrigin {
                    document: QuerySourceId(listed.1.into()),
                    statement: StepSpelling(listed.2.into()),
                },
                owner: listed
                    .0
                    .map_or(SiteOwner::Pack, |rule| SiteOwner::Rule(RuleId(rule.into()))),
                class: listed.3,
            })
            .collect(),
    };
    let breaches = held(&costs, &pack)
        .iter()
        .map(|breach| match breach {
            CostBreach::AboveCeiling(rule, at) => {
                format!("{} is above the ceiling at {}", rule.0, site(at))
            }
            CostBreach::Unlisted(rule, at) => {
                format!("{} reaches the unlisted {}", rule.0, site(at))
            }
            CostBreach::Rose(listed) => format!("{} rose", entry(listed)),
            CostBreach::Stale(listed) => format!("{} is stale", entry(listed)),
        })
        .collect::<Vec<_>>();
    Ok(Cow::Owned(if breaches.is_empty() {
        "held".to_owned()
    } else {
        breaches.join("; ")
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
