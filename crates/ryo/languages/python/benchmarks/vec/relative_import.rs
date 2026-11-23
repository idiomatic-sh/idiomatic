//! Resolve relative import spellings within the containing package supplied by the project.

use super::{
    fact_catalog::RELATIVE_IMPORT_REL,
    finish::{Run, Table},
};

/// Every target takes its occurrence from a relative static-import row.
pub(super) const WITNESS: super::first::RowWitness = super::first::RowWitness {
    relation: RELATIVE_IMPORT_REL,
    projected_key: "occurrence",
    source: "static-import",
    symbol_column: "target",
    prefix: ".",
};

/// One leading dot selects the containing package, and each further dot selects its parent.
fn absolute(module: &[u8], spelling: &[u8]) -> Option<Vec<u8>> {
    let level = spelling.iter().take_while(|&&byte| byte == b'.').count();
    let mut package = module;
    for _ in 0..level {
        package = &package[..package.iter().rposition(|&b| b == b'.')?];
    }
    let mut path = package.to_vec();
    if !spelling[level..].is_empty() {
        path.push(b'.');
        path.extend_from_slice(&spelling[level..]);
    }
    Some(path)
}

/// The path is the imported spelling including its member; the member can be a module or an attribute.
pub(super) fn targets(
    run: &mut Run,
    imports: &Table,
    file: u32,
    identity: impl FnOnce(&mut Run) -> Result<Table, String>,
) -> Result<Table, String> {
    let (_, names, tys) = run
        .catalog(RELATIVE_IMPORT_REL)
        .ok_or("no relative-import-target")?;
    let mut result = Table {
        names,
        tys,
        cols: vec![Vec::new(); 3],
        n: 0,
    };
    let target = imports
        .names
        .iter()
        .position(|name| name == "target")
        .ok_or("static-import lacks target")?;
    let occurrence = imports
        .names
        .iter()
        .position(|name| name == "occurrence")
        .ok_or("static-import lacks occurrence")?;
    let mut relative = (0..imports.n)
        .filter(|&i| run.texts[imports.cols[target][i] as usize].starts_with(b"."))
        .peekable();
    if relative.peek().is_none() {
        return Ok(result);
    }
    let relative: Vec<_> = relative.collect();
    let identity = identity(run)?;
    let path = identity
        .names
        .iter()
        .position(|name| name == "path")
        .ok_or("module-identity lacks path")?;
    for &module in &identity.cols[path] {
        for &i in &relative {
            if let Some(absolute) = absolute(
                &run.texts[module as usize],
                &run.texts[imports.cols[target][i] as usize],
            ) {
                result.cols[0].push(imports.cols[occurrence][i]);
                result.cols[1].push(u64::from(run.sym(&absolute)));
                result.cols[2].push(u64::from(file));
                result.n += 1;
            }
        }
    }
    Ok(result)
}
