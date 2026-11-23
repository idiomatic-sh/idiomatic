//! Compose independent file-local answers. Every listed reader runs the same
//! lowering and executor. Project scope needs a different evaluation phase.
use super::{declaration, reader_run, thread_cpu};
use ryo::languages::{Language, Recognition};
use ryo::linter::{Languages, RelativePath};
use std::{
    collections::BTreeMap,
    io::{BufRead as _, Write as _},
};

pub(super) fn run(readers: Vec<Language>, pack: &str, request: &str, output: &str) -> bool {
    let input: serde_json::Value =
        serde_json::from_slice(&std::fs::read(request).expect("request")).expect("request");
    let readers: Vec<_> = readers.into_iter().map(|l| Languages(vec![l])).collect();
    let mut groups = BTreeMap::<usize, Vec<String>>::new();
    for file in input["files"].as_array().expect("admitted paths") {
        let path = RelativePath(file.as_str().expect("relative path").into());
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
    let mut rows = Vec::new();
    for (index, paths) in groups {
        let scratch = tempfile::NamedTempFile::new().expect("temporary emitted rows");
        if !reader_run::run_files::<true>(
            thread_cpu(),
            readers[index].0[0].clone(),
            declaration(),
            pack,
            request,
            scratch.path().to_str().expect("scratch path"),
            Some(paths),
        ) {
            return false;
        }
        for line in std::io::BufReader::new(scratch.reopen().expect("emitted rows")).lines() {
            let line = line.expect("read emitted row");
            if !line.is_empty() {
                rows.push(line);
            }
        }
    }
    rows.sort_unstable();
    let mut writer = std::io::BufWriter::new(std::fs::File::create(output).expect("result"));
    writeln!(writer, "{}", rows.join("\n")).expect("write emitted rows");
    writer.flush().expect("finish emitted rows");
    true
}
