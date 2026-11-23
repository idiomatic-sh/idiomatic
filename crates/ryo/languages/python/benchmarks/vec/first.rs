//! Prepare the first half from the same typed steps as the executor. A rule's
//! root holds file prerequisites and a necessary witness among the file's nodes.
use super::Vocab;
use super::funnel::SpecRule;
use super::lower::ReadPack;
use super::pair_summary::Program;
use std::fmt::Write as _;

pub(super) const SYNTAX: [&str; 2] = ["semantic-syntax", "syntax-tree"];
pub(super) const CHILD: [&str; 2] = ["semantic-child", "syntax-child"];

/// Fact availability, spelling and lexical text are supplied by the language provider.
pub(super) struct Language {
    pub(super) facts: Vec<String>,
    pub(super) text_kinds: Vec<u16>,
    pub(super) file_sets: Vec<String>,
    pub(super) spelled: Vec<(String, String, Option<char>)>,
    pub(super) is_identifier: fn(&str) -> bool,
}

pub(super) struct Prepared {
    pub(super) pairs: Program,
    pub(super) skips: Vec<(String, bool, Vec<Vec<String>>)>,
    pub(super) rules: Vec<SpecRule>,
}

pub(super) fn prepare(read: &ReadPack, lang: Language, vocab: &Vocab) -> Prepared {
    assert!(
        std::env::var_os("VEC_FIRST_PLANS").is_none()
            && std::env::var_os("VEC_FIRST_SKIP_ONLY").is_none(),
        "this build uses typed pair summaries; use the archived binary to benchmark previous implementations"
    );
    let plan = read.typed.take().unwrap_or_else(|| read.lower_axes());
    assert!(
        plan.unplanned.is_empty(),
        "{} rules unplanned",
        plan.unplanned.len()
    );
    let mut pairs = super::pair_summary::prepare(&plan, vocab, &lang);
    println!(
        "pair summaries: steps {} node domains {} file tests {} rules {} capped {}",
        plan.steps.len(),
        pairs.domains.len(),
        pairs.needs.len(),
        pairs.roots.len(),
        pairs.capped
    );
    let mut skips = Vec::with_capacity(pairs.roots.len());
    let mut rules = Vec::with_capacity(pairs.roots.len());
    for ((name, _), (never, shape, words)) in
        pairs.roots.iter().zip(std::mem::take(&mut pairs.skips))
    {
        skips.push((name.clone(), never, shape));
        rules.push(SpecRule {
            name: name.clone(),
            node: false,
            plans: Vec::new(),
            word_cnfs: vec![words],
        });
    }
    Prepared {
        pairs,
        skips,
        rules,
    }
}

/// Standalone phase measurement starts with lowered steps, as in a combined run.
pub(super) fn run_first(args: &[&str]) {
    let option = |name: &str| args.iter().position(|a| *a == name).map(|i| args[i + 1]);
    let repeat: usize = option("--repeat").map_or(1, |s| s.parse().expect("repeat"));
    let read = super::lower::read_pack(
        std::path::Path::new(args[0]),
        super::request::rule_context(option("--request")),
    );
    let vocab = super::vocabulary(&ryo_python::walk_with_settings(super::target_settings()));
    for round in 0..repeat {
        read.typed.replace(Some(read.lower_axes()));
        let a0 = super::alloc_counts();
        let t0 = super::thread_cpu();
        let p = prepare(&read, super::python_language(&vocab), &vocab);
        let elapsed = super::thread_cpu() - t0;
        let a1 = super::alloc_counts();
        println!(
            "allocations {} bytes {} (peak so far {})",
            a1[0] - a0[0],
            a1[1] - a0[1],
            a1[3]
        );
        println!(
            "round {round} parse {:.4} analysis {elapsed:.4} s rules {}",
            read.parse,
            p.rules.len()
        );
        if round != 0 {
            continue;
        }
        if let Some(path) = option("--dump") {
            let mut w = String::new();
            for (r, s) in p.rules.iter().zip(&p.skips) {
                writeln!(w, "{s:?}\n{r:?}").unwrap();
            }
            for (i, d) in p.pairs.domains.iter().enumerate() {
                writeln!(w, "nodes {i} {d:?}").unwrap();
            }
            for (i, f) in p.pairs.needs.iter().enumerate() {
                writeln!(w, "file {i} {f:?}").unwrap();
            }
            for (rule, root) in &p.pairs.roots {
                writeln!(w, "root {rule} {root}").unwrap();
            }
            std::fs::write(path, w).expect("write summary dump");
        }
        if let Some(path) = option("--out") {
            let mut w = String::from("rule\tfile_test\tnever\tkind_or_field_clauses\n");
            for ((rule, root), (_, never, clauses)) in p.pairs.roots.iter().zip(&p.skips) {
                let clauses: Vec<_> = clauses.iter().map(|c| c.join(",")).collect();
                writeln!(w, "{rule}\t{root}\t{never}\t{}", clauses.join(";")).unwrap();
            }
            std::fs::write(path, w).expect("write rule roots");
        }
    }
}
