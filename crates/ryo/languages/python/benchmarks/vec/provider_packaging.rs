//! redo-general (THROWAWAY, bead ryo-fmx3.1): the PROVIDER's packaging and import resolution (Python's), answered for a
//! project (`finish::Scope::Files`): which entries under the host's search roots a dotted import path names (a module file,
//! a package directory with an initializer, a namespace directory), and each file's module identity inside its packages.
//! Moved here from the executor (`finish.rs` `namespace_membership`, `classified`, `module_identity`; leak review 1.1,
//! 1.2, 6.4): the executor knows no file extension, package initializer or relative-import level. The search roots are the
//! host's (the request's `project_search_roots`); with none the host declared no scope, so no path is proven present or
//! absent (every row `unknown`, as the relation's summary says of an undeclared scope).

use super::HashMap;
use super::finish;
use ryo::languages::{NodeTable, Position, node_table};

/// The relations this provider answers for a project.
pub(super) const MEMBERSHIP_REL: &str = "project-namespace-membership";
pub(super) const MODULE_IDENTITY_REL: &str = "module-identity";

/// The provider's project: the host's root and search roots, the entries observed so far (each dotted path once a run), the
/// kinds of the import syntax, and the rows of each file's module identity once asked.
#[derive(Default)]
pub(super) struct Project {
    root_dir: String,
    search_roots: Option<Vec<String>>,
    classified: HashMap<String, PathObservation>,
    pub(super) import_prefixes: rustc_hash::FxHashMap<u32, super::import_syntax::ImportPrefix>,
    pub(super) head_modules: rustc_hash::FxHashMap<((i64,i64),u32), (u64, super::import_syntax::ModuleContext)>,
    pub(super) import_catalog: rustc_hash::FxHashMap<((i64,i64),u64), u64>,
    kinds: Kinds,
    packages: HashMap<std::path::PathBuf, Option<Package>>,
}

/// A child path can exist only at a root where its parent is a directory.
struct PathObservation {
    kinds: Vec<&'static str>,
    directories: Vec<usize>,
}

/// Directory spellings from the project layout. A package component can itself contain a dot.
pub(super) struct Package {
    pub(super) dotted: String,
    pub(super) root_name: String,
}

/// The filename component used in this project's dotted module identity.
pub(super) fn module_stem(path: &std::path::Path) -> Option<&str> {
    let name = path.file_name()?.to_str()?;
    name.strip_suffix(".pyi").or_else(|| name.strip_suffix(".py"))
}

/// The node kinds of an import (`alias`, `import-from`, the relative-level tokens, the dotted name's parts).
#[derive(Default, Clone, Copy)]
struct Kinds {
    alias: Option<u16>,
    from: Option<u16>,
    dot: Option<u16>,
    ellipsis: Option<u16>,
    ident: Option<u16>,
    name: Option<u16>,
}

impl Project {
    /// The project at `root_dir` with the host's search roots (None: the host declared none); `kind` gives the provider's
    /// code of a kind name.
    pub(super) fn new(root_dir: &str, search_roots: Option<Vec<String>>, kind: &dyn Fn(&str) -> Option<u16>) -> Self {
        let kinds = Kinds { alias: kind("alias"), from: kind("import-from"), dot: kind("dot"), ellipsis: kind("ellipsis"), ident: kind("identifier"), name: kind("name-token") };
        Self { root_dir: root_dir.to_owned(), search_roots, classified: HashMap::new(), import_prefixes: Default::default(), import_catalog: Default::default(), head_modules: Default::default(), kinds, packages: HashMap::new() }
    }

    /// The host's search roots from a request file (`project_search_roots`); None without a request or without the key.
    pub(super) fn search_roots_of(request: Option<&str>) -> Option<Vec<String>> {
        let text = std::fs::read_to_string(request?).ok()?;
        let j: serde_json::Value = serde_json::from_str(&text).ok()?;
        Self::search_roots(&j)
    }

    pub(super) fn search_roots(j: &serde_json::Value) -> Option<Vec<String>> {
        Some(j.get("project_search_roots")?.as_array()?.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect())
    }

    /// A project-scope answer: `rel`'s rows for the run's files `files`. `holds` is the file whose content this provider has
    /// read (`table`, `src`): a relation whose rows depend on a file's own content is answered for that file only.
    pub(super) fn fill(&mut self, run: &mut finish::Run, rel: &str, mask: u64, files: &[u32], holds: u32, table: &NodeTable, src: &[u8]) -> Result<finish::Table, String> {
        let result = self.rows(run, rel, mask, files, holds, table, src)?;
        if finish::fill_prof::dumping() && result.cols.iter().any(|c| c.len() < result.n) {
            let reference = self.rows(run, rel, u64::MAX, files, holds, table, src)?;
            assert_eq!(result.n, reference.n, "{rel} project rows");
            for (i, column) in result.cols.iter().enumerate() {
                if finish::request::wants(mask, i) { assert_eq!(column, &reference.cols[i], "{rel} project column {i}"); }
            }
        }
        Ok(result)
    }

    fn rows(&mut self, run: &mut finish::Run, rel: &str, mask: u64, files: &[u32], holds: u32, table: &NodeTable, src: &[u8]) -> Result<finish::Table, String> {
        match rel {
            MODULE_IDENTITY_REL => self.module_identity(run, mask, files),
            MEMBERSHIP_REL => {
                if files != [holds] {
                    return Err(format!("{rel}: answered from each file's imports as the provider reads the file; asked for {} files while holding file {holds}", files.len()));
                }
                self.namespace_membership(run, mask, holds, table, src)
            }
            _ => Err(format!("{rel}: not a project relation of this provider")),
        }
    }

    /// `project-namespace-membership` of `file`: each dotted prefix of each absolute import path it writes, classified by the
    /// entries under the project's search roots (module file, package directory with an initializer, namespace directory),
    /// a present child of a parent not present made unknown.
    fn namespace_membership(&mut self, run: &mut finish::Run, mask: u64, file: u32, table: &NodeTable, src: &[u8]) -> Result<finish::Table, String> {
        let (_, names, tys) = run.catalog(MEMBERSHIP_REL).ok_or("no project-namespace-membership")?;
        let k = self.kinds;
        let kind = |x: u32| Some(node_table::kind_of(table, Position(x)).0);
        let span = |x: u32| {
            let s = node_table::span_of(table, Position(x));
            (s.start.0 as usize, s.end.0 as usize)
        };
        let names_of = |at: u32, out: &mut Vec<String>| {
            let mut stack = vec![at];
            let mut toks = Vec::new();
            while let Some(x) = stack.pop() {
                for c in node_table::children_of(table, Position(x)).iter().rev() {
                    stack.push(c.0);
                }
                if kind(x) == k.name {
                    toks.push(x);
                }
            }
            toks.sort_unstable_by_key(|t| span(*t).0);
            for t in toks {
                let (s, e) = span(t);
                out.push(String::from_utf8_lossy(&src[s..e]).into_owned());
            }
        };
        let mut paths: std::collections::BTreeSet<(String, String)> = std::collections::BTreeSet::new();
        let aliases = k.alias.map_or(&[][..], |c| node_table::ids_of_kind(table, ryo::languages::KindBits(c)));
        for &Position(a) in aliases {
            let imp = node_table::parent_of(table, Position(a)).0;
            let is_from = kind(imp) == k.from;
            let mut level = 0;
            let mut parts: Vec<String> = Vec::new();
            if is_from {
                for c in node_table::children_of(table, Position(imp)) {
                    let ck = kind(c.0);
                    if ck == k.alias {
                        break;
                    }
                    if ck == k.dot {
                        level += 1;
                    } else if ck == k.ellipsis {
                        // `...` is one token of three relative levels
                        level += 3;
                    } else if ck == k.ident {
                        names_of(c.0, &mut parts);
                    }
                }
            }
            let id = node_table::children_of(table, Position(a)).iter().find(|c| kind(c.0) == k.ident).map(|c| c.0);
            match id {
                Some(i) => names_of(i, &mut parts),
                None => {
                    if !is_from {
                        continue;
                    }
                }
            }
            if level != 0 || parts.is_empty() || parts[0].is_empty() {
                continue;
            }
            paths.insert((parts[0].clone(), parts.join(".")));
        }
        let col = |c: &str| names.iter().position(|x| x == c).expect("a column of the relation");
        let (fc, rc, pc, mc, kc) = (col("file"), col("root"), col("path"), col("membership"), col("kind"));
        let mut rows: std::collections::BTreeSet<(String, String)> = std::collections::BTreeSet::new();
        for (root, path) in &paths {
            let comps: Vec<&str> = path.split('.').filter(|c| !c.is_empty()).collect();
            for i in 1..=comps.len() {
                rows.insert((root.clone(), comps[..i].join(".")));
            }
        }
        let mut cols: Vec<Vec<u64>> = vec![Vec::new(); names.len()];
        let mut count = 0;
        for (root, path) in rows {
            let kinds = self.observed_path_kinds(&path);
            for kd in kinds {
                let state = match kd {
                    "module" | "package" | "namespace" => "present",
                    "absent" => "absent",
                    _ => "unknown",
                };
                if finish::request::wants(mask, fc) { cols[fc].push(u64::from(file)); }
                for (c, text) in [(rc, root.as_str()), (pc, path.as_str()), (mc, state), (kc, kd)] {
                    if finish::request::wants(mask, c) { cols[c].push(u64::from(run.sym(text.as_bytes()))); }
                }
                count += 1;
            }
        }
        Ok(finish::Table { names, tys, cols, n: count })
    }

    /// A file-system observation of a dotted child requires an observed parent under the configured search roots.
    pub(super) fn observed_path_kinds(&mut self, path: &str) -> Vec<&'static str> {
        let kinds = self.classified(path);
        let present = |ks: &[&str]| ks.iter().any(|k| matches!(*k, "module" | "package" | "namespace"));
        match path.rsplit_once('.') {
            Some((parent, _)) if present(&kinds) && !present(&self.classified(parent)) => vec!["unknown"],
            _ => kinds,
        }
    }

    /// The entries a dotted path names under the search roots (observed once a run each).
    fn classified(&mut self, dotted: &str) -> Vec<&'static str> {
        if let Some(entry) = self.classified.get(dotted) {
            return entry.kinds.clone();
        }
        let Some(search_roots) = &self.search_roots else { return vec!["unknown"] };
        let candidates = dotted.rsplit_once('.').and_then(|(parent, _)| self.classified.get(parent));
        if candidates.is_some_and(|entry| entry.directories.is_empty()) {
            self.classified.insert(dotted.to_owned(), PathObservation { kinds: vec!["absent"], directories: Vec::new() });
            return vec!["absent"];
        }
        let mut directories = Vec::new();
        let fs = dotted.replace('.', "/");
        let (mut module, mut package, mut namespace) = (false, false, false);
        for (root, search) in search_roots.iter().enumerate() {
            if candidates.is_some_and(|entry| entry.directories.binary_search(&root).is_err()) { continue; }
            let base = if search.is_empty() { format!("{}/{fs}", self.root_dir) } else { format!("{}/{search}/{fs}", self.root_dir) };
            let file = |p: &str| std::path::Path::new(p).is_file();
            module |= file(&format!("{base}.py")) || file(&format!("{base}.pyi"));
            let dir = std::path::Path::new(&base).is_dir();
            if dir {
                directories.push(root);
            }
            let init = dir && (file(&format!("{base}/__init__.py")) || file(&format!("{base}/__init__.pyi")));
            package |= dir && init;
            namespace |= dir && !init;
        }
        let mut kinds = Vec::new();
        for (p, k) in [(module, "module"), (package, "package"), (namespace, "namespace")] {
            if p {
                kinds.push(k);
            }
        }
        if kinds.is_empty() {
            kinds.push("absent");
        }
        self.classified.insert(dotted.to_owned(), PathObservation { kinds: kinds.clone(), directories });
        kinds
    }

    /// A directory with an initializer extends its parent's package or starts a top-level package.
    /// Both import facts share this observation for the lifetime of the project.
    pub(super) fn package(&mut self, dir: &std::path::Path) -> Option<&Package> {
        if dir.as_os_str().is_empty() {
            return None;
        }
        if !self.packages.contains_key(dir) {
            let package = if let Some(name) = dir.file_name()
                && std::path::Path::new(&self.root_dir).join(dir).join("__init__.py").is_file()
            {
                let name = name.to_string_lossy();
                Some(match dir.parent().and_then(|parent| self.package(parent)) {
                    Some(parent) => Package { dotted: format!("{}.{name}", parent.dotted), root_name: parent.root_name.clone() },
                    None => Package { dotted: name.to_string(), root_name: name.into_owned() },
                })
            } else {
                None
            };
            self.packages.insert(dir.to_owned(), package);
        }
        self.packages.get(dir).and_then(Option::as_ref)
    }

    /// `module-identity` of each of `files`: inside a package (its directory holds `__init__.py`), the dotted path from the
    /// topmost such directory's parent, the module and package directory names and the topmost package's name. A file's row
    /// depends on the project's layout only (the initializers of its directories), never on any file's content.
    fn identity_row(&mut self, path: &std::path::Path) -> Option<[String; 4]> {
        let dir = path.parent()?;
        let package = self.package(dir)?;
        let module = module_stem(path)?;
        Some([format!("{}.{module}", package.dotted), module.to_owned(),
            dir.file_name().unwrap_or_default().to_string_lossy().into_owned(), package.root_name.clone()])
    }

    fn identity_cell(row: &[String; 4], file: u32, column: &str, mut word: impl FnMut(&str) -> u64) -> u64 {
        match column {
            "file" => u64::from(file),
            "path" => word(&row[0]),
            "module-name" => word(&row[1]),
            "package-name" => word(&row[2]),
            "package-root-name" => word(&row[3]),
            _ => finish::NULL,
        }
    }

    /// The prefix uses the same layout observation in its file-local dictionary.
    pub(super) fn identity_columns(&mut self, path: &str, file: u32, words: &mut ryo::languages::FileWords, out: &mut ryo::languages::FilledColumns) -> bool {
        use ryo::languages::{self, CellWord, Relation};
        use ryo_relations::domain::{relations::service as catalog, terms::types::NameText};
        let columns = catalog::columns(Relation::ModuleIdentity);
        out.0.clear();
        out.0.resize_with(columns.len(), Default::default);
        if let Some(row) = self.identity_row(std::path::Path::new(path)) {
            for (col, data) in columns.into_iter().zip(&mut out.0) {
                let value = Self::identity_cell(&row, file, catalog::column_name(col).0, |text| languages::facts::intern(words, NameText(text)).0);
                data.0.push(CellWord(value));
            }
        }
        true
    }

    fn module_identity(&mut self, run: &mut finish::Run, mask: u64, files: &[u32]) -> Result<finish::Table, String> {
        let (_, names, tys) = run.catalog(MODULE_IDENTITY_REL).ok_or("no module-identity")?;
        let mut cols: Vec<Vec<u64>> = vec![Vec::new(); names.len()];
        let mut count = 0;
        for &f in files {
            let Some(row) = self.identity_row(std::path::Path::new(&run.paths[f as usize])) else { continue };
            for (c, name) in names.iter().enumerate() {
                if finish::request::wants(mask, c) {
                    cols[c].push(Self::identity_cell(&row, f, name, |text| u64::from(run.sym(text.as_bytes()))));
                }
            }
            count += 1;
        }
        Ok(finish::Table { names, tys, cols, n: count })
    }
}
