//! The `names` service's cases, on values alone.

use super::service::{builtin_of, find, intern, text_of};
use super::types::{NameCode, NameTable};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo::languages::NameText;

/// The names interned, in order, and the name asked about.
type Input = (&'static [&'static str], &'static str);
/// The code the asked name has, its text read back, and whether the language's builtins have it.
type Value = Option<(NameCode, &'static str, bool)>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_name_met_twice_keeps_its_first_code"),
        word: Word::Succeeds,
        input: || (&["alpha", "beta", "alpha"], "alpha"),
        expect: Expect::Value(Some((NameCode(0), "alpha", false))),
    },
    Case {
        name: CaseName("a_builtin_name_says_so"),
        word: Word::Succeeds,
        input: || (&["alpha", "len"], "len"),
        expect: Expect::Value(Some((NameCode(1), "len", true))),
    },
    Case {
        name: CaseName("a_name_the_file_never_wrote_has_no_code"),
        word: Word::Succeeds,
        input: || (&["alpha"], "gamma"),
        expect: Expect::Value(None),
    },
    Case {
        name: CaseName("a_hundred_names_outgrow_the_first_slots_and_are_all_found"),
        word: Word::Succeeds,
        input: || (&MANY, "n99"),
        expect: Expect::Value(Some((NameCode(99), "n99", false))),
    },
];

/// A hundred distinct names.
const MANY: [&str; 100] = [
    "n0", "n1", "n2", "n3", "n4", "n5", "n6", "n7", "n8", "n9", "n10", "n11", "n12", "n13", "n14", "n15", "n16", "n17", "n18",
    "n19", "n20", "n21", "n22", "n23", "n24", "n25", "n26", "n27", "n28", "n29", "n30", "n31", "n32", "n33", "n34", "n35", "n36",
    "n37", "n38", "n39", "n40", "n41", "n42", "n43", "n44", "n45", "n46", "n47", "n48", "n49", "n50", "n51", "n52", "n53", "n54",
    "n55", "n56", "n57", "n58", "n59", "n60", "n61", "n62", "n63", "n64", "n65", "n66", "n67", "n68", "n69", "n70", "n71", "n72",
    "n73", "n74", "n75", "n76", "n77", "n78", "n79", "n80", "n81", "n82", "n83", "n84", "n85", "n86", "n87", "n88", "n89", "n90",
    "n91", "n92", "n93", "n94", "n95", "n96", "n97", "n98", "n99",
];

fn subject((names, asked): Input) -> Value {
    let mut table = NameTable::default();
    for name in names {
        let _: NameCode = intern(&mut table, NameText(name));
    }
    let code = find(&table, NameText(asked))?;
    let text = text_of(&table, code).filter(|text| text.0 == asked).map(|_| asked)?;
    Some((code, text, builtin_of(&table, code).is_some()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
