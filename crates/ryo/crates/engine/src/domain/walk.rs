//! SPIKE (ryo-fmx3.1, the walk): the one environment switch of the walk prototype, `RYO_WALK`, read once. Unset or empty:
//! today's behaviour exactly. Otherwise a comma separated list of words, each `name` or `name=number`.

use std::sync::OnceLock;

/// The words of `RYO_WALK`, read once.
fn words() -> &'static [(String, Option<u64>)] {
    static WORDS: OnceLock<Vec<(String, Option<u64>)>> = OnceLock::new();
    WORDS.get_or_init(|| {
        let text = std::env::var("RYO_WALK").unwrap_or_default();
        text.split(',')
            .map(str::trim)
            .filter(|word| !word.is_empty())
            .map(|word| match word.split_once('=') {
                Some((name, value)) => (name.to_owned(), value.parse().ok()),
                None => (word.to_owned(), None),
            })
            .collect()
    })
}

/// Whether the word is in the switch.
#[must_use]
pub fn on(name: &str) -> bool {
    words().iter().any(|(word, _)| word == name)
}

/// The number the word is given, when it is in the switch with one.
#[must_use]
pub fn number(name: &str) -> Option<u64> {
    words().iter().find(|(word, _)| word == name).and_then(|(_, value)| *value)
}

/// `pass=<n>`: the asks of a memo site before its hits are looked at (today `PASS_AFTER`); none: today's.
#[must_use]
pub fn pass_after() -> Option<usize> {
    static VALUE: OnceLock<Option<usize>> = OnceLock::new();
    *VALUE.get_or_init(|| number("pass").and_then(|value| usize::try_from(value).ok()))
}

/// `anydepth`: a site may pass at any depth of a program's asks, not only at depth nought.
#[must_use]
pub fn any_depth() -> bool {
    static VALUE: OnceLock<bool> = OnceLock::new();
    *VALUE.get_or_init(|| on("anydepth"))
}

/// Rules lowered with a program of their own, and rules that walk a table of their rows (no program for some branch).
static RULES: [std::sync::atomic::AtomicUsize; 2] =
    [std::sync::atomic::AtomicUsize::new(0), std::sync::atomic::AtomicUsize::new(0)];

/// Counts one rule's lowering: walked = it walks a table of its rows.
pub fn counted(walked: bool) {
    let _before = RULES[usize::from(walked)].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

/// The two counts, (navigated, walked).
#[must_use]
pub fn rules_counted() -> (usize, usize) {
    (
        RULES[0].load(std::sync::atomic::Ordering::Relaxed),
        RULES[1].load(std::sync::atomic::Ordering::Relaxed),
    )
}

/// Rule branches by what their program starts from (`lowering::service::anchor_of`).
static ANCHORS: [std::sync::atomic::AtomicUsize; 5] = [
    std::sync::atomic::AtomicUsize::new(0),
    std::sync::atomic::AtomicUsize::new(0),
    std::sync::atomic::AtomicUsize::new(0),
    std::sync::atomic::AtomicUsize::new(0),
    std::sync::atomic::AtomicUsize::new(0),
];

/// Counts one branch's anchor.
pub fn anchored(class: usize) {
    if let Some(count) = ANCHORS.get(class) {
        let _before = count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// The branch counts by anchor.
#[must_use]
pub fn anchors() -> [usize; 5] {
    ANCHORS.each_ref().map(|count| count.load(std::sync::atomic::Ordering::Relaxed))
}
