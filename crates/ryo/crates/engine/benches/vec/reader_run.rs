//! Run both halves over the SDK parse contract. No intermediate plans or provider facts on disk.
use super::{finish, finisher, first, lower, reader, thread_cpu};
use ryo::languages::{self as l, ByteSlice, FileId, SourcePath, node_table};
use ryo_relations::domain::labels::types::NameRef;
use ryo_relations::domain::relations::service as catalog;
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

#[path = "reader_packet.rs"]
pub(super) mod packet;

#[path = "reader_pairs.rs"]
pub(super) mod pairs;

#[path = "reader_coverage.rs"]
pub(super) mod coverage;

/// Dense local bit numbers only for names that occur in a necessary condition.
#[derive(Default)]
struct Names {
    // Only summary vocabulary extends this table, before file dispatch.
    // A file-local spelling beyond its end has no summary bit.
    by_symbol: Vec<Option<u16>>,
    count: usize,
}

impl Names {
    fn code(&mut self, id: u32) -> u16 {
        let index = id as usize;
        if index >= self.by_symbol.len() {
            self.by_symbol.resize(index + 1, None);
        }
        *self.by_symbol[index].get_or_insert_with(|| {
            let code = u16::try_from(self.count).expect("too many distinct summary words");
            self.count += 1;
            code
        })
    }
    fn width(&self) -> usize {
        self.count.div_ceil(64).max(1)
    }
    fn mark(&self, value: u32, bits: &mut [u64]) {
        if let Some(&Some(code)) = self.by_symbol.get(value as usize) {
            bits[usize::from(code) / 64] |= 1 << (code % 64);
        }
    }
    fn present(&self, values: impl Iterator<Item = u32>) -> Vec<u64> {
        let mut bits = vec![0; self.width()];
        for value in values {
            self.mark(value, &mut bits);
        }
        bits
    }
}

/// In a separate diagnostic process, collect the kernel RSS high-water mark for
/// each phase window, resetting it to current RSS after reading it. Retained
/// allocations count in each phase that holds them.
#[derive(Default)]
struct Memory {
    enabled: bool,
    peaks: [u64; 3],
}
impl Memory {
    fn sample(&mut self, part: usize) {
        if !self.enabled {
            return;
        }
        let status = std::fs::read_to_string("/proc/self/status").expect("process memory");
        let peak = status
            .lines()
            .find_map(|line| line.strip_prefix("VmHWM:"))
            .and_then(|n| n.split_whitespace().next())
            .and_then(|n| n.parse::<u64>().ok())
            .expect("VmHWM in KiB");
        self.peaks[part] = self.peaks[part].max(peak);
        std::fs::write("/proc/self/clear_refs", b"5").expect("reset process high-water RSS");
    }
}
/// These readers resolve field names for each file. Compile name-based file
/// requirements from the lowered steps already prepared by the executor.
pub(super) fn file_requirements(
    plan: &ryo_values::domain::finish_plans::types::FinishPlan,
    language: &first::Language,
) -> Vec<(String, (bool, Vec<Vec<String>>, Vec<Vec<String>>))> {
    use ryo_values::domain::finish_plans::types::RuleEmit;
    assert!(plan.unplanned.is_empty(), "unplanned file queries");
    let mut skips = Vec::with_capacity(plan.steps.len());
    for step in &plan.steps {
        let terms = super::pair_summary::conjuncts(step);
        skips.push(super::pair_skip::Skip::with_terms(step, &skips, language, &terms));
    }
    plan.rules
        .iter()
        .map(|rule| {
            let requirements = match &rule.emit {
                RuleEmit::Written(emit) => skips[emit.step.0].root(),
                RuleEmit::Unwritten => Default::default(),
            };
            (rule.rule.0.to_string(), requirements)
        })
        .collect()
}

/// SDK packets have node-addressable columns or file-wide rows.
pub(super) fn fact_family(name: &str) -> super::pair_summary::cost::Family {
    use super::pair_summary::cost::Family;
    match catalog::relation_named(NameRef(name)) {
        Some(rel) if !catalog::kinds(rel).contains(&l::CellKind::Node) => Family::Sets,
        _ => Family::Fact,
    }
}

pub(super) fn run(
    start: f64,
    language: l::Language,
    declaration: first::Language,
    pack: &str,
    request: &str,
    output: &str,
) -> bool {
    run_files::<false>(start, language, declaration, pack, request, Output::Path(output), None)
}

pub(crate) enum Output<'a> {
    Path(&'a str),
    Rows(&'a mut finish::json_output::Records),
}

/// Evaluate one reader's files of a multi-reader request. The caller collects
/// the complete typed results and reports unsupported project scopes explicitly.
pub(super) fn run_files<const QUERY_ROWS: bool>(
    start: f64,
    language: l::Language,
    declaration: first::Language,
    pack: &str,
    request: &str,
    output: Output<'_>,
    paths: Option<Vec<String>>,
) -> bool {
    evaluate_files::<QUERY_ROWS>(start,language,declaration,pack,request,output,paths,None)
}

pub(super) fn evaluate_files<const QUERY_ROWS:bool>(
    start:f64, language:l::Language, mut declaration:first::Language, pack:&str, request:&str, output:Output<'_>,
    paths:Option<Vec<String>>,mut capture:Option<&mut finisher::retained::Capture>,
)->bool {
    let mut req = finish::json_input::request(request);
    if let Some(paths) = paths { req.files = paths; }
    let context = if QUERY_ROWS {
        super::rule_context(Some(request))
    } else {
        req.check_file_context();
        Default::default()
    };
    let parser = language.parse_engine();
    let (plan, typed, pack_read, pack_parse) = {
        let read = lower::read_pack(Path::new(pack), context);
        let plan = lower::second_half_plan(&read, true);
        let typed = read.typed.take().expect("executor must hand over the typed plan");
        (plan, typed, read.read, read.parse)
    };
    let emits: BTreeSet<_> = plan.emits.iter().map(|emit| emit.0.as_str()).collect();
    for selected in &req.selected_rules {
        assert!(
            emits.contains(selected.as_str()),
            "no lowered emit for {selected}"
        );
    }
    // The parser declares population coverage. The catalog supplies column types.
    use ryo_relations::domain::catalog::{
        service::{fact_form, scope_of},
        types::RowScope,
    };
    use ryo_relations::domain::relations::types::FactForm;
    let bases: BTreeSet<_> = plan
        .nodes
        .iter()
        .filter(|n| n.op == "base")
        .map(|n| n.base().expect("base name"))
        .collect();
    let mut run = finish::Run::default();
    run.file_field_codes = true;
    let mut reader_rows = Vec::new();
    for name in bases {
        let Some(relation) = catalog::relation_named(NameRef(name)) else {
            let message=format!("no catalog declaration for {name}");
            if let Some(c)=capture.as_deref_mut() {c.refused_admission(&message);}
            eprintln!("{message}");
            return false;
        };
        if catalog::class_of(relation) == ryo_relations::domain::relations::types::Class::Engine {
            let (_, names, types) = run.catalog(name).expect("host schema");
            let columns: Vec<_> = names.iter().zip(types.iter())
                .map(|(name, ty)| (name.as_str(), *ty)).collect();
            run.declare(name, &columns, finish::Supplier::Host);
            continue;
        }
        if ryo_relations::domain::catalog::service::scope_of(relation)
            == ryo_relations::domain::catalog::types::RowScope::Project
        {
            let message=format!("project-scoped {name} needs a project reader and a complete inventory");
            if let Some(c)=capture.as_deref_mut() {c.refused_admission(&message);}
            eprintln!("{message}");
            return false;
        }
        if parser.includes_expansions(relation) {
            run.reader_tables.insert(name.to_owned());
            // A provider may append semantic positions under a native holder. Only
            // the row supplier describes that relation on this host.
            if fact_form(relation) == FactForm::NodeTable {
                reader_rows.push(name.to_owned());
            }
        }
        if matches!(fact_form(relation), FactForm::Filled(_))
            && scope_of(relation) == RowScope::File
            && parser.absence_of(relation).is_none()
        {
            reader_rows.push(name.to_owned());
            declaration.facts.push(name.to_owned());
        }
    }
    let summaries_start = thread_cpu();
    let kind_names = parser.kind_names();
    let vocab = pairs::vocabulary(&kind_names.0, parser.stable_fields());
    // The SDK gives an actual text cell even when kinds mix tokens and non-tokens.
    declaration.text_kinds = (0..kind_names.0.len())
        .map(|k| u16::try_from(k).expect("kind code"))
        .collect();
    assert!(typed.unplanned.is_empty());
    let mut program =
        super::pair_summary::prepare_with_reader_rows(&typed, &vocab, &declaration, &reader_rows, true);
    let clauses_by_root = std::mem::take(&mut program.skips);
    drop(typed);
    let summaries_cpu = thread_cpu() - summaries_start;
    run.empty = run.sym(b"");
    let unknown_words = reader::public_words(&mut run, parser.unknown_reasons());
    run.links();
    run.kind_sym = kind_names
        .0
        .iter()
        .map(|k| run.sym(k.0.as_bytes()))
        .collect();
    run.root_dir = req.root.clone();
    run.paths = req.files.clone().into();
    let mut kinds = Names::default();
    let mut fields = Names::default();
    let mut words = Names::default();
    for (_, clauses, _) in &clauses_by_root {
        for name in clauses.iter().flatten() {
            let (tag, term) = name.split_once(':').expect("tagged vocabulary name");
            let id = run.sym(term.as_bytes());
            if tag == "k" {
                kinds.code(id);
            } else {
                fields.code(id);
            }
        }
    }
    let source: HashMap<_, _> = program.roots.iter().map(|(rule, _)| rule.as_str())
        .zip(&clauses_by_root).collect();
    let mut syntax = Vec::new();
    let mut spellings: Vec<Vec<Vec<Vec<u16>>>> = Vec::new();
    for emit in &plan.emits {
        let (never, clauses, word_clauses) = source[emit.0.as_str()];
        let mut masks = Vec::new();
        for clause in clauses {
            let mask = |prefix: &str, names: &Names| {
                names.present(
                    clause
                        .iter()
                        .filter_map(|term| term.strip_prefix(prefix))
                        .map(|term| run.ids[term.as_bytes()]),
                )
            };
            masks.push((mask("k:", &kinds), mask("f:", &fields)));
        }
        syntax.push((*never, masks));
        let branch = word_clauses
            .iter()
            .map(|clause| {
                clause
                    .iter()
                    .map(|term| words.code(run.sym(term.as_bytes())))
                    .collect()
            })
            .collect();
        spellings.push(vec![branch]);
    }
    let mut skip = super::file_skip::Skip::new(
        syntax
            .iter()
            .map(|(never, masks)| (*never, masks.as_slice())),
        spellings.iter().map(Vec::as_slice),
        kinds.width(),
        fields.width(),
    );
    drop(source);
    drop((clauses_by_root, syntax, spellings));
    // The parser needs the union of declared relations before its first file. A particular
    // emit's transitive relation mask is needed only when a file first asks for that emit.
    let mut base_facts = vec![None; plan.nodes.len()];
    let mut distinct = BTreeSet::new();
    for (slot, n) in plan.nodes.iter().enumerate() {
        if let Some(name) = n.base() {
            let rel = catalog::relation_named(NameRef(name)).expect("reader catalog");
            if !matches!(run.supplier(name), finish::Supplier::Host) && (ryo_relations::domain::catalog::service::fact_form(rel)
                != ryo_relations::domain::relations::types::FactForm::NodeTable
                || run.reader_tables.contains(name))
            {
                // The parser writes a complete empty relation for a construct its
                // language lacks. Only an unimplemented reader leaves the answer open.
                if parser.absence_of(rel) == Some(l::AbsenceCause::NotYetRead) {
                    let message=format!("provider does not yet read {name}");
            if let Some(c)=capture.as_deref_mut() {c.refused_admission(&message);}
            eprintln!("{message}");
                    return false;
                }
                distinct.insert(rel);
                base_facts[slot] = Some(rel);
            }
        }
    }
    let relations: Vec<_> = distinct.into_iter().collect();
    let base_bits: Vec<_> = base_facts.into_iter().map(|rel| {
        rel.map(|value| relations.binary_search(&value).expect("catalog relation"))
    }).collect();
    let width = relations.len().div_ceil(64);
    let mut emitted_facts = vec![None; plan.emits.len()];
    let mut visited = vec![usize::MAX; plan.nodes.len()];
    let mut pending = Vec::new();
    let mut requested = vec![0u64; width];
    let mut facts = Vec::with_capacity(relations.len());
    let mut decisions = pairs::Decisions::new(program, &plan);
    let mut relations_for = |wanted: &[usize], facts: &mut Vec<l::Relation>| {
        requested.fill(0);
        for &emit in wanted {
            let mask = emitted_facts[emit].get_or_insert_with(|| {
                let mut mask = vec![0u64; width];
                pending.push(plan.emits[emit].1);
                while let Some(slot) = pending.pop() {
                    if visited[slot] == emit { continue; }
                    visited[slot] = emit;
                    if let Some(bit) = base_bits[slot] { mask[bit / 64] |= 1 << (bit % 64); }
                    pending.extend_from_slice(&plan.nodes[slot].inputs);
                }
                mask
            });
            for (word, &extra) in requested.iter_mut().zip(mask.iter()) {
                *word |= extra;
            }
        }
        facts.clear();
        for (i, &rel) in relations.iter().enumerate() {
            if requested[i / 64] & (1 << (i % 64)) != 0 { facts.push(rel); }
        }
    };
    let selected = req.selected_rules.join(",");
    let mut args = vec!["--rules", &selected, "--applicability", request];
    if let Output::Path(path) = &output { args.extend(["--out", *path]); }
    let mut fin = finisher::Finisher::new(
        &plan,
        run,
        &args,
        req.rule_applicability.as_ref().map(|accepted| (request, accepted)),
    );
    fin.query_rows = QUERY_ROWS;
    fin.capture = capture.as_deref_mut().map(std::mem::take);
    let startup = thread_cpu() - start;
    println!(
        "startup preparation: pack-read {:.6} parse {:.6} file-summaries {:.6}",
        pack_read, pack_parse, summaries_cpu
    );
    let mut memory = Memory {
        enabled: std::env::var_os("VEC_PHASE_RSS").is_some(),
        ..Default::default()
    };
    memory.sample(0);
    let mut first_cpu = 0.;
    let mut second_cpu = 0.;
    let mut fills_made = 0u64;
    let check_without_skip = std::env::var_os("VEC_NO_SKIP").is_some();
    if check_without_skip {
        println!("VEC_NO_SKIP: checking every selected file/emit pair");
    }
    let mut wanted = Vec::new();
    let mut active = vec![0u64; plan.emits.len()];
    let mut elements = l::Elements::default();
    let mut table = l::NodeTable::default();
    let mut packet = packet::Packet::new();
    let mut coverage = coverage::Coverage::default();
    let mut symbols = l::Symbols::default();
    let reader_relations: Vec<_> = relations.iter().copied()
        .chain(std::iter::once(l::Relation::SyntaxTree)).collect();
    for (index, path) in req.files.iter().enumerate() {
        if fin.retained_file(path) { continue; }
        let t0 = thread_cpu();
        let bytes = std::fs::read(Path::new(&req.root).join(path)).expect("read source");
        let id = FileId(index.try_into().expect("file id"));
        let parse = l::ParseRequest {
            file: id,
            path: SourcePath(path),
            bytes: ByteSlice(&bytes),
            fields: l::FieldRequest::Wanted,
            notes: l::NoteRequest::Unwanted,
        };
        parser
            .build_native_file(parse, &mut elements, &mut symbols, l::NodeAccess::Query(&reader_relations))
            .expect("parse file");
        if let Some(c) = &mut fin.capture { c.visited(path,&bytes); }
        let mut provider = reader::ReaderFacts::new(id.0, &unknown_words);
        reader::project_tree(
            &elements,
            &symbols,
            &bytes,
            &mut fin.run,
            &mut table,
            &mut provider,
        );
        let native = l::native_table(&elements).cloned();
        let table = native.as_deref().unwrap_or(&table);
        let mut visible_kinds = kinds.present(
            node_table::kind_codes(&table)
                .iter()
                .map(|&k| fin.run.kind_sym[usize::from(k)]),
        );
        let mut visible_fields = fields.present(
            node_table::field_codes(&table)
                .iter()
                .map(|&f| fin.run.field_sym[usize::from(f)]),
        );
        let mut visible_words = words.present(provider.text.iter().copied());
        // The necessary-condition scan may see a superset: include vocabulary of
        // every additional position without treating it as a concrete tree node.
        for pos in provider.text.len() as u32..l::end_of(&elements).0 {
            let pos = l::Position(pos);
            kinds.mark(
                fin.run.kind_sym[usize::from(l::kind_at(&elements, pos).0)],
                &mut visible_kinds,
            );
            let field = reader::symbol(
                &mut fin.run,
                &unknown_words,
                &symbols,
                &bytes,
                l::field_name_at(&elements, pos),
            );
            fields.mark(field, &mut visible_fields);
            let word = reader::symbol(&mut fin.run, &unknown_words, &symbols, &bytes, l::text_at(&elements, pos));
            words.mark(word, &mut visible_words);
        }
        // A missing scalar has the empty spelling in a text predicate.
        // Allowing that spelling for the file is a necessary-condition superset.
        words.mark(fin.run.empty, &mut visible_words);
        skip.file(&visible_kinds, &visible_fields);
        let only = req.file_rules.get(path);
        fin.wanted_into( |i| {
            only.is_none_or(|rule| rule == &plan.emits[i].0)
                && (check_without_skip || (skip.alive(i) && skip.words_alive(i, &visible_words)))
        }, &mut wanted);
        packet.clear();
        relations_for(&wanted, &mut facts);
        if !wanted.is_empty() && !check_without_skip {
            decisions.filter(
                &mut wanted,
                &table,
                super::pair_checks::SyntaxText {
                    ids: &provider.text,
                    symbols: &fin.run.texts,
                    dictionary: &fin.run.ids,
                    positions: None,
                    distinct: None,
                    patterns: None,
                    column: None,
                },
                &bytes,
                path,
                &mut elements,
                &mut symbols,
                id,
                &facts,
                &mut packet,
                |elements, symbols, rows| {
                    parser.walk_file(elements, symbols, ByteSlice(&bytes), id, &facts, rows);
                },
            );
            if !packet.ready {
                relations_for(&wanted, &mut facts);
            }
        }
        let t1 = thread_cpu();
        first_cpu += t1 - t0;
        memory.sample(1);
        let visits = fin.visits(&wanted);
        if visits && !packet.ready {
            parser.walk_file(
                &mut elements,
                &mut symbols,
                ByteSlice(&bytes),
                id,
                &facts,
                &mut packet.rows,
            );
            packet.index(&facts);
        }
        if let Some(c) = &mut fin.capture {
            use finisher::retained::UnknownOrigin;
            use ryo_relations::domain::relations::types::Missing;
            for (rel, cause, missing) in &packet.rows.partials {
                let kind = match missing { Missing::Any => None, Missing::Kind(kind) => Some(kind.0) };
                c.unknown(path, catalog::name(*rel).0, kind, cause, UnknownOrigin::Partition);
            }
            for (rel, subject, cause) in &packet.rows.partials_at {
                if *subject == id {
                    c.unknown(path, catalog::name(*rel).0, None, cause, UnknownOrigin::Partition);
                } else {
                    c.partial(path);
                }
            }
            if !unknown_words.is_empty() {
                let mut scalar = |relation: &str, word: l::Symbol, origin: UnknownOrigin| {
                    if let Some(&reason) = unknown_words.get(&word.0) {
                        let cause = std::str::from_utf8(&fin.run.texts[reason as usize]).expect("published cause");
                        c.unknown(path, relation, None, cause, origin);
                    }
                };
                for n in 0..l::end_of(&elements).0 {
                    let pos = l::Position(n);
                    scalar("syntax-tree", l::field_name_at(&elements, pos), UnknownOrigin::Field(n));
                    scalar("syntax-tree", l::text_at(&elements, pos), UnknownOrigin::Text(n));
                }
                let mut offset = 0;
                for (row, &relation) in packet.rows.relations.iter().enumerate() {
                    let kinds = catalog::kinds(relation);
                    for (column, kind) in kinds.iter().enumerate() {
                        if *kind == l::CellKind::Symbol {
                            let word = l::Symbol(packet.rows.words[offset + column] as u32);
                            scalar(catalog::name(relation).0, word, UnknownOrigin::Cell(row as u32, column as u32));
                        }
                    }
                    offset += kinds.len();
                }
            }
        }
        if !visits {
            continue;
        }
        packet
            .rows
            .partials
            .retain(|(rel, _, missing)| coverage.needed(&plan, *rel, *missing));
        for &i in &wanted {
            active[i] += 1;
        }
        fin.file(
            index,
            &table,
            &bytes,
            &mut packet.view(&mut provider, &symbols, &bytes),
            &wanted,
        );
        fills_made += provider.fills_made;
        drop((provider, bytes));
        second_cpu += thread_cpu() - t1;
        memory.sample(2);
    }
    println!("pair checks {}", decisions.steps);
    if let Some(meter) = &decisions.program.meter {
        meter.report(&decisions.program);
    }
    println!("reader fills made {fills_made}");
    for (emit, files) in plan.emits.iter().zip(active) {
        println!("HANDOFF\t{}\t{files}", emit.0);
    }
    fin.time = second_cpu;
    let reporting = thread_cpu();
    if let Some(c) = capture.take() { *c = fin.capture.take().unwrap(); }
    let ok = fin.end(match output {
        Output::Rows(rows) => finish::json_output::Projection::Collect(rows),
        Output::Path(_) => finish::json_output::Projection::All,
    });
    second_cpu += thread_cpu() - reporting;
    memory.sample(2);
    if memory.enabled {
        println!(
            "RSS_KIB phase-peaks startup/first/second {:?}",
            memory.peaks
        );
    }
    println!("CPU startup {startup:.6} first {first_cpu:.6} second {second_cpu:.6}");
    ok
}
