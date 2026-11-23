//! Compare independent executions so oracle reads cannot warm a source memo or
//! change the deferred-side decision in the candidate execution.
use super::super::{R, Schema, Table};
use std::{
    fs::File,
    io::{BufWriter, Write},
};

#[derive(Default)]
struct Count {
    node: usize,
    calls: u64,
    rows: u64,
    empty: u64,
}

pub(super) struct Audit {
    counts: Vec<Count>,
    writer: BufWriter<File>,
}

impl Audit {
    pub(super) fn open(n: usize) -> Option<Self> {
        let path = std::env::var_os("VEC_COL_JOIN_SHADOW")?;
        Some(Self {
            counts: (0..n).map(|_| Count::default()).collect(),
            writer: BufWriter::new(File::create(path).expect("create join shadow")),
        })
    }
    pub(super) fn record(
        &mut self,
        index: usize,
        node: usize,
        file: u32,
        schema: &Schema,
        t: &Table,
    ) -> R<()> {
        if t.names != schema.names
            || t.tys != schema.tys
            || t.cols.len() != schema.names.len()
            || t.cols.iter().any(|c| c.len() != t.n)
        {
            return Err(format!("join shadow schema or column length at {node}"));
        }
        let c = &mut self.counts[index];
        c.node = node;
        c.calls += 1;
        c.rows += t.n as u64;
        c.empty += u64::from(t.n == 0);
        let out = &mut self.writer;
        let mut write = || -> std::io::Result<()> {
            out.write_all(&file.to_le_bytes())?;
            out.write_all(&(node as u64).to_le_bytes())?;
            out.write_all(&(t.n as u64).to_le_bytes())?;
            if c.calls == 1 {
                out.write_all(&(schema.names.len() as u64).to_le_bytes())?;
                for (name, ty) in schema.names.iter().zip(schema.tys.iter()) {
                    out.write_all(&(name.len() as u64).to_le_bytes())?;
                    out.write_all(name.as_bytes())?;
                    out.write_all(&[*ty as u8])?;
                }
            }
            for col in &t.cols {
                for word in col {
                    out.write_all(&word.to_le_bytes())?;
                }
            }
            Ok(())
        };
        write().map_err(|e| e.to_string())
    }
    pub(super) fn report(&mut self) {
        self.writer.flush().expect("flush join shadow");
        for c in &self.counts {
            if c.calls != 0 {
                println!("COL_JOIN\t{}\t{}\t{}\t{}", c.node, c.calls, c.rows, c.empty);
            }
        }
        println!("COL_JOIN_BOUND\t{}", self.counts.len());
    }
}
