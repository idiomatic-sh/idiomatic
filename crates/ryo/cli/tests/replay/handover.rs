//! What a file hands the steps after the files is bounded by its own tree (`handovers`), over a tree written here: files of
//! functions copied across files, nested and commented, so the rules that read across files hand their rows.

use crate::cache::{function, write};
use crate::handovers::{handovers_over, within_the_bound};

#[test]
fn no_file_hands_a_rule_more_than_four_rows_for_each_row_of_its_tree() {
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("a directory");
    write(
        root.path(),
        &[
            ("one/lib.rs", function("copied", 40, "1")),
            ("two/lib.rs", function("copied", 40, "1")),
            ("three/lib.rs", function("alone", 40, "2")),
            ("four/lib.rs", function("other", 80, "3")),
        ],
    );
    let handovers = handovers_over(root.path());
    // A run that counted nothing would hold every bound: the run is held to have counted a handover.
    assert!(
        !handovers.is_empty(),
        "the counting run reports a handover for each file that handed something, not {}",
        handovers.len()
    );
    within_the_bound(handovers);
}
