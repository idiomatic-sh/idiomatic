//! Resolve relative import spellings within the containing package supplied by the project.

use super::{
    fact_catalog::RELATIVE_IMPORT_REL,
    finish::{Run, Table},
};

/// One leading dot selects the containing package, and each further dot selects its parent.
fn prefix(mut package: &[u8], level: usize) -> Option<&[u8]> {
    for _ in 1..level {
        package = &package[..package.iter().rposition(|&b| b == b'.')?];
    }
    Some(package)
}

fn absolute(module: &[u8], spelling: &[u8]) -> Option<Vec<u8>> {
    let level = spelling.iter().take_while(|&&byte| byte == b'.').count();
    let package = prefix(&module[..module.iter().rposition(|&b| b == b'.')?], level)?;
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
    let mut result = run.storage.table(names, tys, imports.rows.0);
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
    let mut relative = (0..imports.rows.0)
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
                result.rows.0 += 1;
            }
        }
    }
    Ok(result)
}

/// The column walk can visit just relative imports, without constructing all six static-import columns.
pub(super) fn walked(
    run: &mut Run,
    project: &mut super::provider_packaging::Project,
    source: &ryo_python::fixtures::columns::ColumnFile<'_, '_>,
    file: u32,
) -> Result<Table, String> {
    let (_, names, tys) = run.catalog(RELATIVE_IMPORT_REL).ok_or("no relative-import-target")?;
    let mut result = run.storage.table(names, tys, 0);
    let mut imports = source.relative_imports().peekable();
    if imports.peek().is_none() {
        return Ok(result);
    }
    let file_path = std::path::Path::new(&run.paths[file as usize]);
    let Some(stem) = super::provider_packaging::module_stem(file_path) else { return Ok(result) };
    let Some(directory) = file_path.parent().and_then(|dir| project.package(dir)) else { return Ok(result) };
    // The parent of the dotted module identity can include dots from the file stem.
    let package = match stem.rsplit_once('.') {
        Some((head, _)) => std::borrow::Cow::Owned(format!("{}.{head}", directory.dotted)),
        None => std::borrow::Cow::Borrowed(directory.dotted.as_str()),
    };
    let mut path = Vec::new();
    for (node, level, module, member) in imports {
        let Some(base) = prefix(package.as_bytes(), level as usize) else { continue };
        path.clear();
        path.extend_from_slice(base);
        for component in [module, member].into_iter().flatten() {
            path.push(b'.');
            path.extend_from_slice(component.as_bytes());
        }
        result.cols[0].push(u64::from(node));
        result.cols[1].push(u64::from(run.sym(&path)));
        result.cols[2].push(u64::from(file));
        result.rows.0 += 1;
    }
    Ok(result)
}
