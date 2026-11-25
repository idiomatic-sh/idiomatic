struct Snapshot<'a> {
    project: Option<&'a crate::check_session::project::Population>,
    has_inventory: bool,
    hosted: &'a [Hosted],
    number: &'a dyn Fn(&str) -> Option<u32>,
}

impl Provider for Snapshot<'_> {
    fn shared_fill(&mut self, run:&mut Run, name:&str, scope:Scope<'_>) -> Result<std::rc::Rc<Table>,String> {
        if matches!(scope,Scope::Files(_)) && !self.hosted.iter().any(|h| h.name == name) && let Some(t)=self.project.and_then(|p|p.tables.get(name)) {
            return Ok(t.clone());
        }
        self.fill(run,name,scope).map(std::rc::Rc::new)
    }
    fn fill(&mut self, run: &mut Run, name: &str, scope: Scope<'_>) -> Result<Table, String> {
        let Scope::Files(files) = scope else {
            return Err(format!("local ask in closed session for {name}"));
        };
        if let Some(table) = self.hosted.iter().find(|h| h.name == name) {
            return host_table(table, run, self.number);
        }
        if let Some(t) = self.project.and_then(|p| p.tables.get(name)) {
            return Ok(t.copy());
        }
        if name != "filesystem.file" {
            return Err(format!("missing complete project population for {name}"));
        }
        // The release can withhold a named producer when all of its source
        // partitions were skipped. That state cannot prove a negative lookup.
        if files.is_empty() && self.has_inventory {
            return Err("all file partitions excluded; no complete inventory contribution".into());
        }
        let (_, names, tys) = run.catalog(name).ok_or("no file inventory declaration")?;
        let mut t = Table::with_rows(names, tys, files.len());
        t.cols[0].extend(files.iter().copied().map(u64::from));
        t.n = files.len();
        Ok(t)
    }

    fn render(&mut self, _: u32, _: &str) -> Result<Option<Vec<u8>>, String> {
        Err("project rendering requires source origins".into())
    }
}

fn bind_tables(
    hosted: &[Hosted],
    run: &mut Run,
    file_id: impl Fn(&str) -> Option<u32>,
) -> Result<BTreeMap<String, Table>, String> {
    let mut tables = BTreeMap::new();
    for table in hosted {
        if tables.contains_key(&table.name) {
            return Err(format!("duplicate host relation {}", table.name));
        }
        declare_table(table, run)?;
        tables.insert(table.name.clone(), host_table(table, run, &file_id)?);
    }
    Ok(tables)
}

/// Check one caller table and declare its columns. Its rows are made only when a step reads it.
fn declare_table(table: &Hosted, run: &mut Run) -> Result<(), String> {
    if let Some(reason) = &table.partial {
        return Err(format!("partial {}: {reason}", table.name));
    }
    if ryo_relations::domain::relations::service::relation_named(
        ryo_relations::domain::labels::types::NameRef(&table.name),
    )
    .is_some()
    {
        return Err(format!("relation already declared: {}", table.name));
    }
    let mut columns = Vec::new();
    for (name, kind) in &table.columns {
        let ty = match kind.as_str() {
            "File" => Ty::File,
            "Text" => Ty::Sym,
            "Number" => Ty::Num,
            _ => return Err(format!("unsupported host type {kind}")),
        };
        if columns.iter().any(|(n, _)| n == &name.as_str()) {
            return Err(format!("duplicate column {name}"));
        }
        columns.push((name.as_str(), ty));
    }
    run.declare(&table.name, &columns, Supplier::Host);
    Ok(())
}

/// The rows of a declared caller table, in this run's numbering.
fn host_table(
    table: &Hosted,
    run: &mut Run,
    file_id: &dyn Fn(&str) -> Option<u32>,
) -> Result<Table, String> {
    let (_, names, tys) = run.catalog(&table.name).ok_or("missing host declaration")?;
    let mut t = Table::with_rows(names, tys.clone(), table.rows.len());
    let mut cells = Vec::with_capacity(tys.len());
    'rows: for row in table.rows.iter() {
        if row.len() != tys.len() {
            return Err(format!("host row width in {}", table.name));
        }
        cells.clear();
        for (cell, ty) in row.iter().zip(tys.iter()) {
            let word = match ty {
                Ty::File => {
                    let HostCell::File { file: path } = cell else {
                        return Err("expected file cell".into());
                    };
                    // table_of_query omits a row with no such file in the numbered request.
                    let Some(id) = file_id(path) else {
                        continue 'rows;
                    };
                    u64::from(id)
                }
                Ty::Sym => {
                    let HostCell::Text(text) = cell else {
                        return Err("expected text cell".into());
                    };
                    u64::from(run.sym(text.as_bytes()))
                }
                Ty::Num => {
                    let HostCell::Number(n) = cell else {
                        return Err("expected number cell".into());
                    };
                    *n as u64
                }
                _ => unreachable!(),
            };
            cells.push(word);
        }
        for (column, word) in t.cols.iter_mut().zip(&cells) {
            column.push(*word);
        }
        t.n += 1;
    }
    Ok(t)
}

fn execute_ids(
    exec: &mut finish::Exec<'_>,
    run: &mut Run,
    numbers: &[u32],
    carried: &finish::population::Inputs,
    inventory: &[String],
    files: &[String],
    hosted: &[Hosted],
    project: Option<&crate::check_session::project::Population>,
    incomplete: &BTreeMap<String, Vec<Option<u32>>>,
) -> Result<Vec<finish::answer::Row>, String> {
    let number = |p: &str| {
        inventory
            .binary_search_by(|s| s.as_str().cmp(p))
            .ok()
            .map(|i| numbers[i])
    };
    let admitted: Vec<_> = files
        .iter()
        .map(|p| number(p).ok_or_else(|| format!("undeclared path {p}")))
        .collect::<Result<_, _>>()?;
    let subject_files: BTreeSet<_> = admitted.iter().copied().collect();
    if subject_files.len() != admitted.len() {
        return Err("duplicate admitted path".into());
    }
    let binding = finish::operators::enter(usize::MAX - 13, 0);
    let mut seen = BTreeSet::new();
    let declared = hosted.iter().try_for_each(|table| {
        if !seen.insert(table.name.as_str()) {
            return Err(format!("duplicate host relation {}", table.name));
        }
        declare_table(table, run)
    });
    if let Some(start) = binding {
        finish::operators::leave(start, "ids-declare-tables", 0, true);
    }
    declared?;
    let mut snapshot = Snapshot {
        project,
        has_inventory: !inventory.is_empty(),
        hosted,
        number: &number,
    };
    finish::session::evaluate_covered(
        exec,
        run,
        &mut snapshot,
        &admitted,
        |file| subject_files.contains(&file),
        carried,
        incomplete,
    )
}
