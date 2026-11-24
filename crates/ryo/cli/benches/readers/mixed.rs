//! Compose independent file-local answers. Every listed reader runs the same
//! lowering and executor. Project scope needs a different evaluation phase.
use super::{declaration, reader_run, thread_cpu};
use ryo::languages::{Language, Recognition};
use ryo::linter::{Languages, RelativePath};
use std::collections::BTreeMap;

pub(super) fn run(readers: Vec<Language>, pack: &str, request: &str, output: &str, files: Vec<String>) -> bool {
    let readers: Vec<_> = readers.into_iter().map(|l| Languages(vec![l])).collect();
    let mut groups = BTreeMap::<usize, Vec<String>>::new();
    for file in files {
        let path = RelativePath(file.into());
        let mut recognized = readers
            .iter()
            .enumerate()
            .filter(|(_, l)| l.reads(&path) == Recognition::Reads);
        let Some((index, _)) = recognized.next() else {
            eprintln!("no source reader for {}", path.0);
            return false;
        };
        if recognized.next().is_some() {
            eprintln!("multiple source readers for {}", path.0);
            return false;
        }
        groups.entry(index).or_default().push(path.0.into());
    }
    let mut rows = super::finish::json_output::Records::default();
    for (index, paths) in groups {
        if !reader_run::run_files::<true>(
            thread_cpu(),
            readers[index].0[0].clone(),
            declaration(),
            pack,
            request,
            reader_run::Output::Rows(&mut rows),
            Some(paths),
        ) {
            return false;
        }
    }
    rows.write(output);
    true
}
