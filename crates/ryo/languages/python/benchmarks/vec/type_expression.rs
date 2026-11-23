//! redo-binding pass 4 (THROWAWAY, bead ryo-fmx3.1): the `type-expression` fact. For every element of a file, whether the
//! typing rules make it part of a type expression: the roots (annotations, `TypeAlias` values, type parameter payloads,
//! string annotations, the slices of the generic heads the pack lists, the type arguments of the typing calls: cast, NewType,
//! TypeVar, TypeAliasType, NamedTuple, TypedDict, and of the argument forms the pack lists, the first argument of `Annotated`, the field types of a
//! TypedDict form) make their subtree a type expression down to the nearest value root (every call's arguments, `Annotated`
//! metadata, the TypedDict form's keys). `type` for an element of a type expression; `value` for a value root nested in a
//! type expression (the start of a value expression some enclosing expression's type expression holds); every other
//! element is a value expression and has no row.
//!
//! Pass 5 (PRINCIPLE): the fact states the typing rules only. What one checker chooses on top (Ruff keeps a `bool(...)`
//! argument in its enclosing context, and leaves the unpacked keywords of cast / NewType / TypeAliasType and every keyword
//! of `bool` unvisited) is rule text: the pack fragments `type-definition-context` and `type-definition-skipped`.
//!
//! The prototype computes it from the provider's node table and four provider relations (qualified-name, call-argument,
//! identifier, embedded-expression); a provider would write it from its own model.

use super::finish;
use ryo::languages::{NodeTable, Position, node_table};
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::LazyLock;

static TYPED: LazyLock<regex::bytes::Regex> = LazyLock::new(|| {
    regex::bytes::Regex::new(
        "^(?:typing|typing_extensions|_typeshed)[.](?:cast|NewType|TypeVar|TypeAliasType|NamedTuple|TypedDict)$",
    )
    .expect("regex")
});


/// The modules every typing-specification path the fact tests lies in (a prefix test before the patterns); the pack's
/// lists add the first component of each of their paths.
const NAMED_MODULES: [&[u8]; 3] = [b"typing", b"_typeshed.", b"typing_extensions."];

/// Pack data the rules pass in (redo-binding pass 6, GENERALITY): the fact writer knows no such list. `generic`: the
/// heads whose subscript's slice is a type root outside a target, in value context too (a checker's choice of generic
/// names, Ruff's table); `argument_forms`: callables whose first positional argument and `type=` keyword are type
/// expressions (mypy_extensions' Arg family is mypy's convention). Read from the plan's `type-expression` input.
/// The lists of the run's plan (set once, before the first file).
pub(super) static LISTS: std::sync::OnceLock<Lists> = std::sync::OnceLock::new();

/// redo-binding pass 6/7: sets `LISTS` from the pack's inputs to the facts: `<pack dir>/inputs/type-expression.tsv` when
/// the second half is prepared from a pack, else beside the plan file (`<plan>.inputs/type-expression.tsv`, made by the
/// plan script from the pack's `inputs/`); no file, no list.
pub(super) fn load_lists(plan_or_pack: &str) {
    let path = if std::path::Path::new(plan_or_pack).is_dir() {
        format!("{}/inputs/type-expression.tsv", plan_or_pack.trim_end_matches('/'))
    } else {
        format!("{}.inputs/type-expression.tsv", plan_or_pack.trim_end_matches(".json"))
    };
    let lists = match std::fs::read_to_string(path) {
        Ok(t) => Lists::parse(&t).expect("the type-expression input"),
        Err(_) => Lists::default(),
    };
    let _ = LISTS.set(lists);
}

#[derive(Default)]
pub(super) struct Lists {
    pub(super) generic: FxHashSet<Vec<u8>>,
    pub(super) argument_forms: FxHashSet<Vec<u8>>,
    pub(super) modules: Vec<Vec<u8>>,
}

impl Lists {
    /// `<kind>\t<path>` lines, kind `generic` or `argument-form`; `#` starts a comment line.
    pub(super) fn parse(text: &str) -> Result<Lists, String> {
        let mut l = Lists::default();
        for line in text.lines().map(str::trim).filter(|x| !x.is_empty() && !x.starts_with('#')) {
            let (kind, path) = line.split_once('\t').ok_or_else(|| format!("type-expression input: {line}"))?;
            match kind {
                "generic" => l.generic.insert(path.as_bytes().to_vec()),
                "argument-form" => l.argument_forms.insert(path.as_bytes().to_vec()),
                other => return Err(format!("type-expression input kind {other}")),
            };
            let first = format!("{}.", path.split('.').next().unwrap_or(path));
            if !l.modules.iter().any(|m| m == first.as_bytes()) {
                l.modules.push(first.into_bytes());
            }
        }
        Ok(l)
    }
}

/// The typing callable a path names.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Form {
    Cast,
    NewType,
    TypeVar,
    TypeAliasType,
    NamedTuple,
    TypedDict,
    ArgumentForm,
}

fn form_of(path: &[u8], lists: &Lists) -> Option<Form> {
    if lists.argument_forms.contains(path) {
        return Some(Form::ArgumentForm);
    }
    if !TYPED.is_match(path) {
        return None;
    }
    Some(if path.ends_with(b".cast") {
        Form::Cast
    } else if path.ends_with(b".NewType") {
        Form::NewType
    } else if path.ends_with(b".TypeVar") {
        Form::TypeVar
    } else if path.ends_with(b".TypeAliasType") {
        Form::TypeAliasType
    } else if path.ends_with(b".NamedTuple") {
        Form::NamedTuple
    } else {
        Form::TypedDict
    })
}

/// The provider tables the fact reads.
pub(super) struct Inputs<'t> {
    pub(super) qualified: &'t finish::Table,
    pub(super) arguments: &'t finish::Table,
    pub(super) identifiers: &'t finish::Table,
    pub(super) embedded: &'t finish::Table,
    pub(super) lists: &'t Lists,
}

/// redo-fills pass 2: whether the file has any root `rows` starts from (an embedded expression, a name the paths test
/// names, an annotation or return annotation, a type declaration): `rows`' own "no root of any kind" test, asked before
/// its other inputs are filled (call-argument and identifier are read only past it).
pub(super) fn any_root(run: &mut finish::Run, table: &NodeTable, qualified: &finish::Table, embedded_rows: usize, lists: &Lists) -> Result<bool, String> {
    if embedded_rows != 0 {
        return Ok(true);
    }
    let (q_id, q_path) = (col(qualified, "id")?, col(qualified, "path")?);
    let _ = q_id;
    for r in 0..qualified.n {
        let t: &[u8] = &run.texts[qualified.cols[q_path][r] as usize];
        // redo-int merge: `rows`' tests as redo-binding pass 6 states them (the pack's lists `LISTS`, passed in as `rows`
        // reads them; redo-binding pass 7's own read of `LISTS` here is the same value, so not kept)
        if !NAMED_MODULES.iter().any(|m| t.starts_with(m)) && !lists.modules.iter().any(|m| t.starts_with(m)) {
            continue;
        }
        if t == b"typing.TypeAlias" || t == b"typing_extensions.TypeAlias" || t == b"_typeshed.TypeAlias" || lists.generic.contains(t) || t == b"typing.Annotated" || t == b"typing_extensions.Annotated" || t == b"typing.TypedDict" || t == b"typing_extensions.TypedDict" || form_of(t, lists).is_some() {
            return Ok(true);
        }
    }
    Ok(super::annotation_evaluation::has_annotation(run, table) || super::deferred_visit::has_type_parameters(run, table))
}

fn col(t: &finish::Table, n: &str) -> Result<usize, String> {
    t.names.iter().position(|c| c == n).ok_or_else(|| format!("no column {n}"))
}

/// The fact's rows: (element, `type` | `value`): `value` for the root of a value expression nested in a type expression.
pub(super) fn rows(run: &mut finish::Run, table: &NodeTable, inp: &Inputs<'_>) -> Result<Vec<(u32, bool)>, String> {
    let n = node_table::element_count(table).0;
    let sym = |run: &mut finish::Run, t: &[u8]| run.sym(t);
    let k_call = sym(run, b"call");
    let k_keyword = sym(run, b"keyword");
    let k_subscript = sym(run, b"subscript");
    let k_tuple = sym(run, b"tuple");
    let k_list = sym(run, b"list");
    let k_dict = sym(run, b"dict");
    let k_decl = [sym(run, b"type-alias"), sym(run, b"type-var"), sym(run, b"type-var-tuple"), sym(run, b"param-spec")];
    let f_value = sym(run, b"value");
    let f_func = sym(run, b"func");
    let f_arg = sym(run, b"arg");
    let f_annotation = sym(run, b"annotation");
    let f_returns = sym(run, b"returns");
    let f_bound = sym(run, b"bound");
    let f_default = sym(run, b"default-value");
    let f_targets = [sym(run, b"target"), sym(run, b"targets"), sym(run, b"optional-vars")];
    let f_slice = sym(run, b"slice");
    let f_elts = sym(run, b"elts");
    let f_keys = sym(run, b"keys");
    let f_values = sym(run, b"values");
    let mut kind = vec![0u32; n];
    let mut field = vec![0u32; n];
    let mut parent = vec![u32::MAX; n];
    for p in 0..n {
        let at = Position(p as u32);
        kind[p] = run.kind_sym[usize::from(node_table::kind_of(table, at).0)];
        field[p] = run.field_sym.get(usize::from(node_table::field_of(table, at).0)).copied().unwrap_or(run.empty);
        let q = node_table::parent_of(table, at).0;
        if p != 0 && q != p as u32 {
            parent[p] = q;
        }
    }
    let fields: &[u32] = &field;
    let children = move |x: u32| node_table::children_of(table, Position(x)).iter().map(|c| c.0).filter(move |c| (*c as usize) < n);
    let child_by = move |x: u32, f: u32| children(x).filter(move |c| fields[*c as usize] == f);
    let ordinal = |x: u32| node_table::ordinal_of(table, Position(x)).0;
    let subtree = |x: u32| {
        let end = node_table::subtree_end_of(table, Position(x)).0;
        x..end.max(x + 1)
    };
    // what each node's qualified paths name
    const ALIAS: u8 = 1;
    const GENERIC_NAME: u8 = 2;
    const ANNOTATED: u8 = 4;
    const TYPED_DICT: u8 = 8;
    let (q_id, q_path) = (col(inp.qualified, "id")?, col(inp.qualified, "path")?);
    let mut names: FxHashMap<u32, u8> = FxHashMap::default();
    let mut callee_forms: FxHashMap<u32, Vec<Form>> = FxHashMap::default();
    for r in 0..inp.qualified.n {
        let x = inp.qualified.cols[q_id][r] as u32;
        let t: &[u8] = &run.texts[inp.qualified.cols[q_path][r] as usize];
        // every name the tests below match starts with one of these modules
        if !NAMED_MODULES.iter().any(|m| t.starts_with(m)) && !inp.lists.modules.iter().any(|m| t.starts_with(m)) {
            continue;
        }
        let mut bits = 0;
        if t == b"typing.TypeAlias" || t == b"typing_extensions.TypeAlias" || t == b"_typeshed.TypeAlias" {
            bits |= ALIAS;
        }
        if inp.lists.generic.contains(t) {
            bits |= GENERIC_NAME;
        }
        if t == b"typing.Annotated" || t == b"typing_extensions.Annotated" {
            bits |= ANNOTATED;
        }
        if t == b"typing.TypedDict" || t == b"typing_extensions.TypedDict" {
            bits |= TYPED_DICT;
        }
        if bits != 0 {
            *names.entry(x).or_default() |= bits;
        }
        if let Some(fm) = form_of(t, inp.lists) {
            let e = callee_forms.entry(x).or_default();
            if !e.contains(&fm) {
                e.push(fm);
            }
        }
    }
    // no root of any kind: no type expression in the file
    if inp.embedded.n == 0
        && names.is_empty()
        && callee_forms.is_empty()
        && !(0..n).any(|p| (parent[p] != u32::MAX && (field[p] == f_annotation || field[p] == f_returns)) || k_decl.contains(&kind[p]))
    {
        return Ok(Vec::new());
    }
    let names_of = |x: u32| names.get(&x).copied().unwrap_or(0);
    // the typing forms of each call (by its callee's paths)
    let mut forms: FxHashMap<u32, Vec<Form>> = FxHashMap::default();
    for c in 0..n as u32 {
        if kind[c as usize] != k_call {
            continue;
        }
        for callee in child_by(c, f_func) {
            if let Some(fs) = callee_forms.get(&callee) {
                let e = forms.entry(c).or_default();
                for fm in fs {
                    if !e.contains(fm) {
                        e.push(*fm);
                    }
                }
            }
        }
    }
    let has_form = |c: u32, f: Form| forms.get(&c).is_some_and(|v| v.contains(&f));
    // keywords: value, name, owner (nearest call above)
    let (i_id, i_name) = (col(inp.identifiers, "id")?, col(inp.identifiers, "name")?);
    let mut ident: FxHashMap<u32, u32> = FxHashMap::default();
    for r in 0..inp.identifiers.n {
        let _ = ident.insert(inp.identifiers.cols[i_id][r] as u32, inp.identifiers.cols[i_name][r] as u32);
    }
    let mut keyword_values: FxHashSet<u32> = FxHashSet::default();
    // (owner call, Some(name) | None (unpacked), value)
    let mut keywords: Vec<(u32, Option<u32>, u32)> = Vec::new();
    for kw in 0..n as u32 {
        if kind[kw as usize] != k_keyword {
            continue;
        }
        let values: Vec<u32> = child_by(kw, f_value).collect();
        keyword_values.extend(values.iter().copied());
        let mut owner = parent[kw as usize];
        while owner != u32::MAX && kind[owner as usize] != k_call {
            owner = parent[owner as usize];
        }
        if owner == u32::MAX || !forms.contains_key(&owner) {
            continue;
        }
        let names: Vec<u32> = child_by(kw, f_arg).filter_map(|a| ident.get(&a).copied()).collect();
        for v in &values {
            if names.is_empty() {
                keywords.push((owner, None, *v));
            }
            for nm in &names {
                keywords.push((owner, Some(*nm), *v));
            }
        }
    }
    // call arguments
    let (a_call, a_expr, a_pos) = (col(inp.arguments, "call")?, col(inp.arguments, "expression")?, col(inp.arguments, "position")?);
    let mut types = vec![false; n];
    let mut values = vec![false; n];
    let mut positional: FxHashMap<u32, Vec<(i64, u32)>> = FxHashMap::default();
    for r in 0..inp.arguments.n {
        let (call, value, position) = (inp.arguments.cols[a_call][r] as u32, inp.arguments.cols[a_expr][r] as u32, inp.arguments.cols[a_pos][r] as i64);
        if (value as usize) >= n {
            continue;
        }
        values[value as usize] = true;
        if forms.contains_key(&call) {
            if (has_form(call, Form::Cast) && position == 0) || ((has_form(call, Form::NewType) || has_form(call, Form::TypeAliasType)) && position == 1) {
                types[value as usize] = true;
            }
            if !keyword_values.contains(&value) {
                positional.entry(call).or_default().push((position, value));
            }
        }
    }
    let name_is = |run: &finish::Run, s: u32, t: &[u8]| &*run.texts[s as usize] == t;
    // keyed values of a dictionary: each key paired with the first unpaired value after it
    let keyed_values = |d: u32| -> Vec<u32> {
        if kind[d as usize] != k_dict {
            return Vec::new();
        }
        let mut ks: Vec<u32> = child_by(d, f_keys).map(ordinal).collect();
        let mut vs: Vec<(u32, u32)> = child_by(d, f_values).map(|v| (ordinal(v), v)).collect();
        ks.sort_unstable();
        vs.sort_unstable();
        let mut out = Vec::new();
        let mut at = 0;
        for o in ks {
            while at < vs.len() && vs[at].0 <= o {
                at += 1;
            }
            if at < vs.len() {
                out.push(vs[at].1);
                at += 1;
            }
        }
        out
    };
    let mut field_dicts: Vec<u32> = Vec::new();
    for (call, ps) in &mut positional {
        ps.sort_unstable();
        ps.dedup();
        for (rank, (_, v)) in ps.iter().enumerate() {
            let v = *v;
            if has_form(*call, Form::TypeVar) && rank > 0 {
                types[v as usize] = true;
            }
            if has_form(*call, Form::ArgumentForm) && rank == 0 {
                types[v as usize] = true;
            }
            if has_form(*call, Form::NamedTuple) && rank > 0 && (kind[v as usize] == k_tuple || kind[v as usize] == k_list) {
                for pair in child_by(v, f_elts) {
                    if kind[pair as usize] != k_tuple && kind[pair as usize] != k_list {
                        continue;
                    }
                    let mut elements: Vec<(u32, u32)> = child_by(pair, f_elts).map(|e| (ordinal(e), e)).collect();
                    elements.sort_unstable();
                    elements.dedup_by_key(|e| e.1);
                    if elements.len() == 2 {
                        types[elements[1].1 as usize] = true;
                    }
                }
            }
            if has_form(*call, Form::TypedDict) && rank > 0 {
                field_dicts.push(v);
            }
        }
    }
    for &(call, name, value) in &keywords {
        let Some(name) = name else {
            if has_form(call, Form::NamedTuple) {
                field_dicts.push(value);
            }
            if has_form(call, Form::TypedDict) {
                types[value as usize] = true;
            }
            continue;
        };
        let named = |t: &[u8]| name_is(run, name, t);
        if (has_form(call, Form::Cast) && named(b"typ"))
            || (has_form(call, Form::NewType) && named(b"tp"))
            || (has_form(call, Form::TypeAliasType) && (named(b"value") || named(b"type_params")))
            || (has_form(call, Form::TypeVar) && (named(b"bound") || named(b"default")))
            || has_form(call, Form::NamedTuple)
            || has_form(call, Form::TypedDict)
        {
            types[value as usize] = true;
        }
        if has_form(call, Form::ArgumentForm) && named(b"type") && positional.get(&call).is_none_or(Vec::is_empty) {
            types[value as usize] = true;
        }
    }
    for d in field_dicts {
        if (d as usize) < n {
            for v in keyed_values(d) {
                types[v as usize] = true;
            }
        }
    }
    // the syntactic roots
    let (e_root,) = (col(inp.embedded, "root")?,);
    for r in 0..inp.embedded.n {
        let x = inp.embedded.cols[e_root][r] as usize;
        if x < n {
            types[x] = true;
        }
    }
    let mut in_target = vec![false; n];
    for p in 0..n {
        if parent[p] == u32::MAX {
            continue;
        }
        let f = field[p];
        if f == f_annotation || f == f_returns {
            types[p] = true;
        }
        if f == f_annotation && names_of(p as u32) & ALIAS != 0 {
            for v in child_by(parent[p], f_value) {
                types[v as usize] = true;
            }
        }
        if f_targets.contains(&f) {
            for q in subtree(p as u32) {
                if (q as usize) < n {
                    in_target[q as usize] = true;
                }
            }
        }
    }
    for p in 0..n as u32 {
        let k = kind[p as usize];
        if k_decl.contains(&k) {
            for c in children(p) {
                let f = field[c as usize];
                if f == f_value || f == f_bound || f == f_default {
                    types[c as usize] = true;
                }
            }
        }
        if k != k_subscript {
            continue;
        }
        let heads: Vec<u32> = child_by(p, f_value).collect();
        let named = |bit: u8| heads.iter().any(|h| names_of(*h) & bit != 0);
        let target = in_target[p as usize];
        if !target && named(GENERIC_NAME) {
            for s in child_by(p, f_slice) {
                types[s as usize] = true;
            }
        }
        if named(ANNOTATED) {
            for s in child_by(p, f_slice) {
                let mut elements: Vec<(u32, u32)> = child_by(s, f_elts).map(|e| (ordinal(e), e)).collect();
                elements.sort_unstable();
                for (i, (_, e)) in elements.iter().enumerate() {
                    if i == 0 {
                        types[*e as usize] = true;
                    } else {
                        values[*e as usize] = true;
                    }
                }
            }
        }
        if !target && named(TYPED_DICT) {
            for s in child_by(p, f_slice) {
                values[s as usize] = true;
                for c in children(s) {
                    if field[c as usize] == f_keys || field[c as usize] == f_values {
                        values[c as usize] = true;
                    }
                }
                for v in keyed_values(s) {
                    types[v as usize] = true;
                }
            }
        }
    }
    // each element: its own root, else the nearest root above it
    const UNKNOWN: u8 = 0;
    const TYPE: u8 = 1;
    const VALUE: u8 = 2;
    let mut class = vec![UNKNOWN; n];
    let mut out = Vec::new();
    let mut chain: Vec<usize> = Vec::new();
    // a type element strictly above (elements come in preorder: a parent before its children)
    let mut type_above = vec![false; n];
    for p in 0..n {
        let mut at = p;
        chain.clear();
        let found = loop {
            if class[at] != UNKNOWN {
                break class[at];
            }
            if types[at] {
                break TYPE;
            }
            if values[at] {
                break VALUE;
            }
            chain.push(at);
            let up = parent[at];
            if up == u32::MAX || chain.len() > n {
                break VALUE;
            }
            at = up as usize;
        };
        // an element's class is its own root's, else its nearest root's above
        let own = if types[p] {
            TYPE
        } else if values[p] {
            VALUE
        } else {
            found
        };
        class[p] = own;
        for c in &chain {
            class[*c] = found;
        }
        let up = parent[p];
        if up != u32::MAX && (up as usize) < p {
            type_above[p] = class[up as usize] == TYPE || type_above[up as usize];
        }
        if own == TYPE {
            out.push((p as u32, true));
        } else if values[p] && type_above[p] {
            out.push((p as u32, false));
        }
    }
    Ok(out)
}
