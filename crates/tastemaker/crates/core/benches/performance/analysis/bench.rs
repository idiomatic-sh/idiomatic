//! The analysis hex's adapters back to back: the ryo adapter that keeps nothing between calls, and the one that keeps what a
//! call leaves in a store, each validating the worked queries `explain ryoql` shows.

use tastemaker_core::domain::catalog::constants::WORKED_QUERIES;
use tastemaker_core::domain::definitions::types::QueryText;
use tastemaker_core::domain::files::types::MachinePath;
use tastemaker_core::domain::settings::types::{Build, Store};
use tastemaker_core::domain::writing::types::Writing;
use tastemaker_core::outbound::analysis::factory;
use tastemaker_core::outbound::analysis::interface::AnalysisPort;
use tastemaker_core::outbound::analysis::types::{AnalysisSettings, Memory};
use tastemaker_core::outbound::scheduler::factory::pool;
use tastemaker_core::outbound::scheduler::types::WorkerCount;

/// The analysis adapters the factory builds.
#[derive(Clone, Copy, Debug)]
enum Variant {
    /// Keeps nothing between its lint calls.
    Ryo,
    /// Keeps what a lint call leaves for the next.
    RyoKept,
}

impl Variant {
    /// Every adapter, run on the same queries.
    const ALL: [Self; 2] = [Self::Ryo, Self::RyoKept];

    /// The adapter this variant is, built through its factory function.
    fn build(self) -> Box<dyn AnalysisPort> {
        let settings = AnalysisSettings {
            pool: pool(WorkerCount(1)),
            memory: Memory::Machine,
        };
        let languages = ryo::linter::Languages::default();
        match self {
            Self::Ryo => Box::new(factory::ryo(languages, settings)),
            Self::RyoKept => Box::new(factory::ryo_kept(
                languages,
                settings,
                &Store {
                    directory: MachinePath(std::env::temp_dir().display().to_string().into()),
                    build: Build("bench".into()),
                    writing: Writing::DryRun,
                },
            )),
        }
    }
}

fn main() {
    divan::main();
}

/// Validates every worked query through the adapter.
#[divan::bench(args = Variant::ALL)]
fn validate_worked_queries(variant: Variant) -> usize {
    let adapter = variant.build();
    WORKED_QUERIES
        .iter()
        .filter(|(_, query)| adapter.validate(&QueryText((*query).into())).is_ok())
        .count()
}
