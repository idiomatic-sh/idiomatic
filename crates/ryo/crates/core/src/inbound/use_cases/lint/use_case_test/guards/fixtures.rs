//! The runs the guards hold, over three files of the fake language with inline rules, and the breach each guard finds: an
//! unchanged warm run over the store a cold run left reads no file; no file hands a rule more than four rows for each row of its
//! tree; and a damaged store answers as if there were none.

#![expect(
    clippy::expect_used,
    reason = "a guard whose lint cannot run has nothing to hold: the panic is the failure of the row"
)]

use crate::inbound::use_cases::lint::fixtures::given::Given;
use crate::inbound::use_cases::lint::fixtures::given::request;
use crate::inbound::use_cases::lint::fixtures::given::violations;
use crate::inbound::use_cases::lint::fixtures::stores::store_of;
use crate::inbound::use_cases::lint::fixtures::worlds::compiled_world;
use crate::inbound::use_cases::lint::interface::LintUseCase as _;
use crate::outbound::language::fixtures::fake::LanguageFake;
use crate::outbound::lint_store::interface::LintStorePort;
use crate::outbound::row_sink::types::KeptRow;
use ryo_execution_runtime::outbound::observer::factory::counting;
use ryo_values::domain::runs::types::Handover;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// The most rows a file may hand one rule's steps after the files for each row of its syntax tree: a rule hands the rows of the
/// relations it reads, each of which has at most a row for each node or token of the tree, so four inputs of the tree's size fit
/// and a step whose rows grow with pairs of them does not.
const HANDOVER_WIDTH_BOUND: u64 = 4;

/// The files every guard lints: two that define one function and one that defines another.
const FILES: [(&str, &str); 3] = [
    ("one.rs", "copied() {}"),
    ("two.rs", "copied() {}"),
    ("three.rs", "alone() {}"),
];

/// The rows beyond the first the fake language reads of each file for the handover guard: a tree of five rows.
const WIDER: u32 = 4;

/// A rule across the files: the names of the functions more than one file defines.
const COPIES: &str = "let names = project(from=\"function\", columns=[\"name\", \"file\"]);\nlet counted = group-count(from=names, keys=[\"name\"], count=\"files\");\nlet copied = select * from counted where files >= 2;\nemit copied subject name key(name);";

/// A rule across the files whose step in each file pairs every function of the file with every other: what a file hands it grows
/// with the square of its tree.
const PAIRS: &str = "let pairs = join(left=\"function\", right=\"function\", left-on=[\"file\"], right-on=[\"file\"], output={name:[left,name],one:[left,id],other:[right,id]});\nlet counted = group-count(from=pairs, keys=[\"name\"], count=\"pairs\");\nemit counted subject name key(name);";

/// What a guard found broken.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Breach {
    /// A warm run over an unchanged tree read this many files.
    Read(usize),
    /// A file handed a rule this many rows for a tree of this many.
    Wide { handed: u64, parsed: u64 },
    /// The run after the store was damaged read only this many files: it used what the store kept.
    Kept(usize),
    /// The run after the store was damaged answered other rows than a run that keeps nothing.
    Answered,
}

/// What one lint did: the files the corpus read, the rows it wrote and the handovers it reported.
struct Linted {
    reads: usize,
    rows: Vec<KeptRow>,
    handovers: Vec<Handover>,
}

/// One lint of [`FILES`] with `rule`, the fake language reading `extra_rows` rows beyond the first of each, over `store`, counting
/// what each file hands.
fn linted(rule: &'static str, extra_rows: u32, store: Arc<dyn LintStorePort>) -> Linted {
    let given = Given {
        files: FILES.to_vec(),
        rules: vec![rule],
        kept_in: Some(store),
        language: LanguageFake {
            named_by_text: true,
            extra_rows,
            ..LanguageFake::default()
        },
        ..violations()
    };
    let asked = request(&given);
    let world = compiled_world(given);
    let report = world
        .use_case
        .counting(|| Arc::new(counting()))
        .lint(asked)
        .expect("the rule runs");
    Linted {
        reads: world.reads.load(Ordering::SeqCst),
        rows: std::mem::take(
            &mut world
                .collected
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .rows,
        ),
        handovers: report.handovers,
    }
}

/// A cold run of [`COPIES`] over the store on disk in `directory`, held to have read every file and found the copies: a run that read
/// nothing would hold every guard.
fn cold(directory: &Path) -> Linted {
    let cold = linted(COPIES, 0, store_of("postcard", directory));
    assert_eq!(cold.reads, FILES.len(), "a cold run reads every file");
    assert!(!cold.rows.is_empty(), "a cold run finds the copies");
    cold
}

/// What the warm run is given after the cold one.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Between {
    /// The store the cold run left, read by an adapter of its own, as a fresh process reads it.
    Kept,
    /// The plant: a store that kept nothing.
    Forgotten,
}

/// The warm run over an unchanged tree reads no file and answers what the cold run did: the files it read and the rows it answered.
///
/// # Errors
///
/// [`Breach::Read`] when the warm run reads a file.
pub(crate) fn warm(between: Between) -> Result<(usize, usize), Breach> {
    let directory = tempfile::tempdir().expect("a directory");
    let cold = cold(directory.path());
    let store = match between {
        Between::Kept => store_of("postcard", directory.path()),
        Between::Forgotten => store_of("discard", directory.path()),
    };
    let warm = linted(COPIES, 0, store);
    if warm.reads > 0 {
        return Err(Breach::Read(warm.reads));
    }
    assert!(warm.rows == cold.rows, "a warm run answers what the cold run did");
    Ok((warm.reads, warm.rows.len()))
}

/// The rule a file hands its rows to.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Handed {
    /// [`COPIES`]: each file hands the names of its functions.
    Copies,
    /// The plant, [`PAIRS`]: each file hands every pair of its functions.
    Pairs,
}

/// No file hands the rule more than [`HANDOVER_WIDTH_BOUND`] rows for each row of its tree: how many files handed it, and the most
/// rows one handed.
///
/// # Errors
///
/// [`Breach::Wide`], of the widest file, when a file hands more.
pub(crate) fn handover(handed: Handed) -> Result<(usize, u64), Breach> {
    let rule = match handed {
        Handed::Copies => COPIES,
        Handed::Pairs => PAIRS,
    };
    let directory = tempfile::tempdir().expect("a directory");
    let run = linted(rule, WIDER, store_of("memory", directory.path()));
    // A run that counted no handover would hold every bound.
    assert!(!run.handovers.is_empty(), "the run reports what each file handed");
    let widest = run
        .handovers
        .iter()
        .max_by_key(|each| u128::from(each.handed.0) * 1_000 / u128::from(each.parsed.0.max(1)))
        .expect("a handover");
    if widest.handed.0 > HANDOVER_WIDTH_BOUND * widest.parsed.0.max(1) {
        return Err(Breach::Wide {
            handed: widest.handed.0,
            parsed: widest.parsed.0,
        });
    }
    Ok((run.handovers.len(), widest.handed.0))
}

/// What is done to the store the cold run left before the next run.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Damage {
    /// Cut to half its length, as a copy that stopped in the middle leaves it.
    Cut,
    /// Replaced by bytes that are not a store.
    Replaced,
    /// One byte, in the middle of the file, flipped.
    Flipped,
    /// Removed.
    Removed,
    /// The plant: left as it was.
    Untouched,
}

/// The run after the store is damaged reads every file and answers what a run that keeps nothing does: the files it read and the
/// rows it answered.
///
/// # Errors
///
/// [`Breach::Kept`] when it reads fewer files, [`Breach::Answered`] when it answers other rows.
pub(crate) fn damaged(damage: Damage) -> Result<(usize, usize), Breach> {
    let directory = tempfile::tempdir().expect("a directory");
    let none = linted(COPIES, 0, store_of("discard", directory.path()));
    let _cold = cold(directory.path());
    let file = std::fs::read_dir(directory.path())
        .expect("the cache directory")
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.is_file())
        .expect("the store the cold run kept");
    let mut bytes = std::fs::read(&file).expect("the store");
    match damage {
        Damage::Cut => bytes.truncate(bytes.len() / 2),
        Damage::Replaced => bytes = b"not a store".to_vec(),
        Damage::Flipped => {
            let middle = bytes.len() / 2;
            *bytes.get_mut(middle).expect("a byte") ^= 0xff;
        }
        Damage::Removed | Damage::Untouched => {}
    }
    match damage {
        Damage::Removed => std::fs::remove_file(&file).expect("the store removed"),
        Damage::Untouched => {}
        Damage::Cut | Damage::Replaced | Damage::Flipped => {
            std::fs::write(&file, bytes).expect("the store damaged");
        }
    }
    let after = linted(COPIES, 0, store_of("postcard", directory.path()));
    if after.reads < FILES.len() {
        return Err(Breach::Kept(after.reads));
    }
    if after.rows != none.rows {
        return Err(Breach::Answered);
    }
    Ok((after.reads, after.rows.len()))
}
