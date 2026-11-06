//! The `ast.dump` pages pinned below `standard`, with the files they were printed for.

/// The `ast.dump` outputs pinned below `standard`, each with the file it was printed for, both by path below `fixtures`, sorted: for the
/// corpus and the suite at the same path, and for each relation's input beside the page `standard/<relation>.ast`.
pub(crate) fn pinned(
    standard: &std::path::Path,
    fixtures: &std::path::Path,
) -> Vec<(std::path::PathBuf, std::path::PathBuf)> {
    let mut found = Vec::new();
    let mut folders = vec![standard.join("corpus"), standard.join("suite")];
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(&folder)
            .expect("a folder of the fixtures is read")
            .map(|entry| entry.expect("an entry of a folder of the fixtures is read"))
        {
            let path = entry.path();
            if path.is_dir() {
                folders.push(path);
                continue;
            }
            if path.extension().is_some_and(|each| each == "ast")
                && let Ok(below) = path.strip_prefix(standard)
                && fixtures.join(below).with_extension("py").exists()
            {
                found.push((below.to_path_buf(), below.with_extension("py")));
            }
        }
    }
    for entry in std::fs::read_dir(standard)
        .expect("a folder of the fixtures is read")
        .map(|entry| entry.expect("an entry of a folder of the fixtures is read"))
    {
        let path = entry.path();
        let input = std::path::Path::new("relations")
            .join(path.file_stem().unwrap_or_default())
            .join("input.py");
        if path.extension().is_some_and(|each| each == "ast") && fixtures.join(&input).exists() {
            found.push((
                path.strip_prefix(standard).unwrap_or(&path).to_path_buf(),
                input,
            ));
        }
    }
    found.sort();
    found
}
