//! Move grammar results across one reset without retaining a corpus of input strings.



/// How many entries the memo holds.
#[cfg(test)]
#[must_use]
pub(crate) fn len<V>(memo: &GrammarMemo<V>) -> super::types::EntryCount {
    super::types::EntryCount(memo.entries.len())
}









