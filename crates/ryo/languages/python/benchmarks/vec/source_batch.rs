//! Read adjacent request entries in the same directory together, in their
//! original order. A batch lives only until the next directory is opened.
use std::fs::File;
use std::io::{Read, Result};
use std::path::Path;

#[derive(Default)]
pub(super) struct SourceBatch {
    bytes: Vec<u8>,
    ends: Vec<usize>,
    first: usize,
}

impl SourceBatch {
    pub(super) fn file(&mut self, files: &[String], index: usize) -> Result<&[u8]> {
        if self.ends.is_empty() || index >= self.first + self.ends.len() - 1 {
            self.bytes.clear();
            self.ends.clear();
            self.ends.push(0);
            self.first = index;
            let directory = Path::new(&files[index]).parent();
            for name in &files[index..] {
                if Path::new(name).parent() != directory {
                    break;
                }
                File::open(name)?.read_to_end(&mut self.bytes)?;
                self.ends.push(self.bytes.len());
                // Bound read-ahead even for a project laid out in a single
                // directory. A single input may itself exceed this byte budget.
                if self.bytes.len() >= 1024 * 1024 {
                    break;
                }
            }
        }
        let at = index - self.first;
        Ok(&self.bytes[self.ends[at]..self.ends[at + 1]])
    }
}
