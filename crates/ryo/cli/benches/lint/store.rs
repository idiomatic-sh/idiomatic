//! The store a run leaves in the cache directory of a target: the files and the megabytes after a cold run and a warm
//! one. Exact numbers, so one round is the whole sample.

use crate::bmf::Cost;

/// The files and the bytes of the store a run kept in the cache directory of a root.
fn store_of(root: &std::path::Path) -> (u128, u128) {
    let (mut files, mut bytes) = (0_u128, 0_u128);
    for entry in std::fs::read_dir(root.join(".ryo_cache"))
        .expect("the cache directory")
        .flatten()
    {
        files += 1;
        bytes += u128::from(entry.metadata().map_or(0, |each| each.len()));
    }
    (files, bytes)
}

/// The store of each target after a cold run and a warm one.
pub(crate) fn of_each_target() -> Vec<Cost> {
    let rules: Vec<_> = crate::counting::runnable()
        .into_iter()
        .map(|(_, rule)| rule)
        .collect();
    let mut metrics = Vec::new();
    let name = "ruff";
    {
        let (root, inside) = crate::base::snapshot::project(name);
        let copy = crate::ruff::copy_of(&root);
        let at = copy.path().join(inside);
        let kept = || crate::base::store_at(&at);
        let cold =
            crate::counting::lint_counting(&at, rules.clone(), kept()).expect("the rules run");
        assert!(cold.reads > 0, "the cold run reads the target's files");
        let (cold_files, cold_bytes) = store_of(&at);
        let warm =
            crate::counting::lint_counting(&at, rules.clone(), kept()).expect("the rules run");
        assert_eq!(
            warm.reads, 0,
            "the warm run reads no file, the store answers it"
        );
        let (files, bytes) = store_of(&at);
        eprintln!(
            "{name}: store after a cold run {cold_files} files {} MB, after a warm run {files} files {} MB",
            cold_bytes >> 20_i32,
            bytes >> 20_i32
        );
        metrics.push(Cost {
            target: name,
            metric: "store-megabytes",
            rounds: vec![bytes >> 20_i32],
        });
        metrics.push(Cost {
            target: name,
            metric: "store-files",
            rounds: vec![files],
        });
    }
    metrics
}
