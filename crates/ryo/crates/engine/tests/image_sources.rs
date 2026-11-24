//! Cargo's linked-image dependency files include the actual inputs of all local crates.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const IMAGES: [&str; 4] = ["tm", "vec", "vec_ts", "vec_rust"];

fn inputs(depfile: &str) -> Result<BTreeSet<PathBuf>, String> {
    let joined = depfile.replace("\\\n", "");
    let (_, dependencies) = joined
        .lines()
        .next()
        .and_then(|line| line.split_once(": "))
        .ok_or("missing make dependency rule")?;
    let mut words = BTreeSet::new();
    let mut word = String::new();
    let mut escaped = false;
    for ch in dependencies.chars().chain(std::iter::once(' ')) {
        if escaped {
            word.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch.is_whitespace() {
            if !word.is_empty() {
                words.insert(PathBuf::from(std::mem::take(&mut word)));
            }
        } else {
            word.push(ch);
        }
    }
    Ok(words)
}

fn engine_source(path: &Path) -> bool {
    if path.extension().is_none_or(|e| e != "rs") {
        return false;
    }
    let path = path.to_string_lossy();
    // Keep revisions distinct: a pinned SDK and the current SDK do not define
    // interchangeable types. This classifier only selects, never renames, inputs.
    if path.contains("/crates/ryo/") || path.contains("/idiomatic-sh-ryo-") {
        return !path.contains("/benches/lint_image/main.rs")
            && !path.contains("/benches/lint_image/request.rs");
    }
    if let Some((_, relative)) = path.rsplit_once("/crates/tastemaker/benchmarks/project/") {
        // These two files compose and instrument the measured binary.
        return !matches!(relative, "live/allocated.rs" | "live/round-host.rs");
    }
    false
}

fn differences(images: &[BTreeSet<PathBuf>; 4]) -> BTreeMap<PathBuf, Vec<&'static str>> {
    let mut files = BTreeMap::<PathBuf, Vec<&str>>::new();
    for (name, sources) in IMAGES.into_iter().zip(images) {
        for path in sources.iter().filter(|p| engine_source(p)) {
            files.entry(path.clone()).or_default().push(name);
        }
    }
    files.retain(|_, names| names.len() != IMAGES.len());
    files
}

#[test]
fn catches_a_private_engine_module_in_each_image() {
    let path = PathBuf::from("/repo/crates/ryo/crates/engine/benches/vec/check_session/private.rs");
    let common = PathBuf::from("/repo/crates/ryo/crates/engine/benches/vec/finish.rs");
    for (index, name) in IMAGES.iter().enumerate() {
        let mut sets = std::array::from_fn(|_| BTreeSet::from([common.clone()]));
        assert!(differences(&sets).is_empty());
        sets[index].insert(path.clone());
        assert_eq!(
            differences(&sets),
            BTreeMap::from([(path.clone(), vec![*name])])
        );
    }
    assert_eq!(
        inputs("image: /a\\ b.rs /c.rs\n").unwrap(),
        BTreeSet::from([PathBuf::from("/a b.rs"), PathBuf::from("/c.rs")])
    );
}

#[test]
#[ignore = "requires the four linked benchmark images in the same Cargo target directory"]
fn four_images_compile_the_same_engine_sources() {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target"));
    let sets = IMAGES.map(|name| {
        let file = target.join("release").join(format!("{name}.d"));
        let source =
            std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        let paths: BTreeSet<_> = inputs(&source)
            .unwrap()
            .into_iter()
            .filter(|p| p.extension().is_some_and(|ext| ext == "rs"))
            .map(|p| {
                p.canonicalize()
                    .unwrap_or_else(|e| panic!("{}: {e}", p.display()))
            })
            .collect();
        assert!(
            paths
                .iter()
                .any(|p| p.ends_with("engine/benches/vec/check_session.rs")),
            "{name} does not compile the session driver"
        );
        paths
    });
    let private = differences(&sets);
    assert!(
        private.is_empty(),
        "engine sources compiled by only some images: {private:#?}"
    );
}
