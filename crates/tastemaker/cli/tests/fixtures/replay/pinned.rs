//! The pin of a copied workspace to the built `tm`, which the replay tests and the `commands` bench share.

use std::path::Path;

/// Sets the `tastemaker` pin of the workspace file `file` to the built `tm`'s version. A workspace in the repository pins the
/// published `tm` the lint runs; its copy is run by the `tm` under test, which runs only a workspace that pins its own version.
pub(crate) fn pinned(file: &Path) -> std::io::Result<()> {
    let text = std::fs::read_to_string(file)?;
    let written: String = text
        .split_inclusive('\n')
        .map(|line| {
            if line.starts_with("tastemaker = ") {
                let ending = line.strip_prefix(line.trim_end()).unwrap_or_default();
                format!("tastemaker = \"{}\"{ending}", env!("CARGO_PKG_VERSION"))
            } else {
                line.to_owned()
            }
        })
        .collect();
    std::fs::write(file, written)
}
