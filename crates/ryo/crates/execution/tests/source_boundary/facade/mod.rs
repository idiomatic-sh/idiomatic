//! The facade re-exports are gone: only the owning modules name the file and project runs.
use super::fixtures::{Compilation, Expectation, Probe};
const PROGRAM: &str = "src/application/file_dag/program.rs";
const OWNING_PATHS: &str = r#"
mod facade_owning_paths {
    use crate::application::file_dag::program::facade_runtime::file_execution::open_file as _;
    use crate::application::file_dag::program::facade_runtime::project_execution::open_project as _;
    use crate::application::file_dag::program::facade_runtime::types::ProgramServices as _;
}
"#;
const DELETED_REEXPORTS: &str = r#"
mod facade_deleted_reexports {
    use crate::application::file_dag::program::facade_runtime::{
        FileRun, ProgramServices, ProjectRoot, ProjectRun, evaluate_roots, open_file, open_project,
    };
}
"#;
pub(super) fn controls() -> Vec<Probe> {
    vec![
        Probe {
            compilation: Compilation::Library,
            owner: PROGRAM,
            source: OWNING_PATHS.into(),
            expected: Expectation::Accept,
        },
        Probe {
            compilation: Compilation::Library,
            owner: PROGRAM,
            source: DELETED_REEXPORTS.into(),
            expected: Expectation::Refuse {
                code: "E0432",
                first: "open_file",
                second: "facade_runtime",
            },
        },
    ]
}
