//! Python lexical extents in physical source coordinates.
use super::{
    finish::{Run, Supplier, Table, Ty},
    import_syntax::line_starts,
};
use ryo::languages::node_table;
use ryo_python::fixtures::columns::ColumnFile;

pub(super) const COMMENT: &str = "comment-position";
pub(super) const BREAKS: &str = "comment-line-breaks";
pub(super) const MULTILINE: &str = "multiline-lexeme";

pub(super) fn declare(run: &mut Run) {
    use Ty::{File, Node, Num, Span, Sym};
    run.declare(
        COMMENT,
        &[
            ("id", Node),
            ("file", File),
            ("span", Span),
            ("line", Num),
            ("own-line", Num),
        ],
        Supplier::Provider,
    );
    run.declare(
        BREAKS,
        &[
            ("id", Node),
            ("file", File),
            ("previous-end", Num),
            ("logical-end", Num),
            ("following-end", Num),
            ("has-logical", Num),
            ("has-following", Num),
        ],
        Supplier::Provider,
    );
    run.declare(
        MULTILINE,
        &[
            ("id", Node),
            ("file", File),
            ("line", Num),
            ("endline", Num),
            ("form", Sym),
            ("component-start", Num),
            ("component-end", Num),
        ],
        Supplier::Provider,
    );
}

pub(super) fn fill(
    run: &mut Run,
    file: &ColumnFile<'_, '_>,
    src: &[u8],
    number: u32,
    relation: &str,
) -> Result<Table, String> {
    let (_, names, types) = run
        .catalog(relation)
        .ok_or_else(|| format!("missing {relation}"))?;
    let mut rows = run.storage.table(names, types, 0);
    let mut write = |row: &[u64]| {
        for (col, value) in rows.cols.iter_mut().zip(row) {
            col.push(*value);
        }
        rows.rows.0 += 1;
    };
    let lines = std::cell::OnceCell::new();
    let starts = || lines.get_or_init(|| line_starts(src));
    let file_id = u64::from(number);
    if relation == COMMENT {
        // A comment is on its own physical line if the line prefix contains only lexical whitespace or the initial BOM.
        for &id in file.comment_positions() {
            let span = node_table::span_of(file.table, id);
            let lines = starts();
            let line = lines.partition_point(|at| *at <= span.start.0);
            let prefix = &src[lines[line - 1] as usize..span.start.0 as usize];
            let prefix = if line == 1 {
                prefix.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(prefix)
            } else {
                prefix
            };
            let standalone = prefix.iter().all(|b| matches!(b, b' ' | b'\t' | 12));
            write(&[
                u64::from(id.0),
                file_id,
                u64::from(span.start.0) << 32 | u64::from(span.end.0),
                line as u64,
                u64::from(standalone),
            ]);
        }
    } else if relation == BREAKS {
        let offset = |at: Option<u32>| at.map_or(super::finish::NULL, u64::from);
        file.visit_comment_breaks(|id, last, logical, following| {
            write(&[
                id.0.into(),
                file_id,
                last.into(),
                offset(logical),
                offset(following),
                logical.is_some().into(),
                following.is_some().into(),
            ])
        });
    } else {
        // Python's multiline lexical unit reaches from its first physical line to the line containing its end offset.
        let mut extents = Vec::new();
        file.visit_multiline_lexemes(src, |id, form| {
            let span = node_table::span_of(file.table, id);
            let lines = starts();
            let first = lines.partition_point(|at| *at <= span.start.0);
            let last = lines.partition_point(|at| *at <= span.end.0);
            if first < last {
                extents.push((first, last, id.0, run.sym(form.as_bytes())));
            }
        });
        // Overlapping inclusive physical-line extents form a connected component, regardless of lexical form.
        extents.sort_unstable();
        let mut component = extents.as_slice();
        while let Some(&(first, mut last, _, _)) = component.first() {
            let mut count = 1;
            while count < component.len() && component[count].0 <= last {
                last = last.max(component[count].1);
                count += 1;
            }
            for &(a, b, id, form) in &component[..count] {
                write(&[
                    id.into(),
                    file_id,
                    a as u64,
                    b as u64,
                    form.into(),
                    first as u64,
                    last as u64,
                ]);
            }
            component = &component[count..];
        }
    }
    Ok(rows)
}
