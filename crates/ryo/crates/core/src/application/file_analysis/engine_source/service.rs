//! The engine's own rows of a file as one of the run's sources: the file's row, its line count, its executable bit and its read
//! failure, each written only when the run demands it and the file's rules ask for it, and an executable bit the edge did not
//! supply said to be read in part.
use super::super::types::EngineSource;
use crate::application::file_analysis::constants::INPUT_EDGE_DID_NOT_SUPPLY_EXECUTABLE;
use crate::domain::reading::types::EngineRows;
use crate::domain::visit::service::lines_in;
use crate::domain::visit::types::BorrowedBytes;
use ryo_execution_runtime::outbound::facts::errors::FactsError;
use ryo_execution_runtime::outbound::facts::types::fact_sources::value::FactSource;
use ryo_language::domain::symbols::service::intern_derived;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::collections::relations::filesystem_executable::item::FilesystemExecutableRow;
use ryo_relations::domain::relations::collections::relations::filesystem_file::item::FilesystemFileRow;
use ryo_relations::domain::relations::collections::relations::filesystem_line_count::item::FilesystemLineCountRow;
use ryo_relations::domain::relations::collections::relations::filesystem_read_failure::item::FilesystemReadFailureRow;
use ryo_relations::domain::relations::types::{
    ColumnsView, Flag, Integer, Missing, Offset, Relation, RowWriter, Span,
};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::files::types::ExecutablePermission;
use ryo_values::domain::stored_facts::types::{PartialAt, PartialCause, PartialScope, ReadInPart};

impl FactSource for EngineSource<'_> {
    fn serves(&self) -> &[Relation] {
        self.serves
    }

    fn read(
        &mut self,
        symbols: &mut Symbols,
        wanted: &[Relation],
        out: &mut dyn RowWriter,
    ) -> Result<Vec<ReadInPart>, FactsError> {
        let demanded = |relation: Relation, asked: EngineRows| {
            asked == EngineRows::Asked && wanted.contains(&relation)
        };
        let mut said = Vec::new();
        // An executable bit the edge did not supply is said to be read in part whenever the file's rules ask for it: what is
        // partial is the file's, not the read's.
        if self.asked.executable == EngineRows::Asked {
            match self.executable {
                posix @ (ExecutablePermission::PosixExecutable
                | ExecutablePermission::PosixNotExecutable) => {
                    if wanted.contains(&Relation::FilesystemExecutable) {
                        out.write(FilesystemExecutableRow {
                            path: self.file,
                            executable: if posix == ExecutablePermission::PosixExecutable {
                                Flag::Raised
                            } else {
                                Flag::Lowered
                            },
                        });
                    }
                }
                ExecutablePermission::Unsupported => {}
                ExecutablePermission::Unknown => said.push(ReadInPart {
                    relation: Relation::FilesystemExecutable,
                    cause: PartialCause(INPUT_EDGE_DID_NOT_SUPPLY_EXECUTABLE.into()),
                    lacking: Missing::Any,
                    at: PartialScope::OneFile(PartialAt(self.file.0)),
                }),
            }
        }
        if demanded(Relation::FilesystemFile, self.asked.file) {
            out.write(FilesystemFileRow { path: self.file });
        }
        if demanded(Relation::FilesystemLineCount, self.asked.lines) {
            out.write(FilesystemLineCountRow {
                path: self.file,
                lines: Integer(lines_in(BorrowedBytes(&self.bytes.0)).0),
            });
        }
        if let Some(detail) = self.failure
            && demanded(Relation::FilesystemReadFailure, self.asked.read_failure)
        {
            out.write(FilesystemReadFailureRow {
                path: self.file,
                detail: intern_derived(symbols, ByteSlice(&[]), NameText(detail.0)),
                span: Span {
                    start: Offset(0),
                    end: Offset(0),
                },
            });
        }
        Ok(said)
    }

    fn admit(&mut self, _relation: Relation, _rows: &ColumnsView<'_>) -> Result<(), FactsError> {
        // The engine's rows are its own: nothing a language wrote to admit.
        Ok(())
    }
}
