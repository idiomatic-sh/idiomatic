# idiomatic-sh-ryo-sdk

The SDK of ryo: the calls a program makes to check a repository with rules written in RyoQL, and the contract a language implements to be read.

## Usage

Call `ryo::<resource>::<verb>` (`ryo::query::run`, `ryo::linter::lint`, `ryo::query::validate`, `ryo::relation::list`, ...) with the languages the program links; with none, every file is reported unread.

```rust
use ryo::linter::{Languages, MemoryBudget, WorkerPool, Workers};
use ryo::query::{DEFAULT_LIMIT, RunRequest, RyoQlQuery};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Every file is a row of `filesystem.file`, whichever language reads it.
    let query = RyoQlQuery::try_from(
        r#"let f = select * from "filesystem.file"; emit f subject path key(path);"#,
    )?;
    // The threads the run reads files on, built once by the program and handed to every call.
    let pool = WorkerPool::from(Workers(std::num::NonZeroUsize::MIN));
    // A program that links languages passes each one's `linked()` here.
    let report = ryo::query::run(
        Languages::default(),
        &pool,
        MemoryBudget::Machine,
        RunRequest {
            root: ".".into(),
            query,
            limit: DEFAULT_LIMIT,
        },
    )?;
    assert!(report.total.0 > 0, "the crate has files");
    assert_eq!(report.shown.0, report.rows.len() as u64);
    Ok(())
}
```
