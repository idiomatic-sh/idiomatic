//! The shape of the data this command does not control: its switches become the service's request, and the service's
//! report becomes the answer printed: what was written, and what to run next.

use asymptote_core::domain::ratchet::types::Writing;
use asymptote_core::inbound::use_cases::init::errors::InitError;
use asymptote_core::inbound::use_cases::init::types::{Configuration, InitReport, InitRequest};
use asymptote_core::outbound::hooks::errors::HooksError;

/// The service's request for the edge's switches.
pub(super) fn request(edge: &crate::edge::Edge) -> InitRequest {
    InitRequest {
        start: edge.start.clone(),
        writing: edge.writing,
    }
}

/// How `init` ends for `error`: a failure where the user's repository or hooks are the problem, an error where the
/// system is.
pub(super) fn outcome_of(error: &InitError) -> crate::outcome::Outcome {
    match error {
        InitError::Foreign { .. } | InitError::Hooks(HooksError::NotARepository { .. }) => {
            crate::outcome::Outcome::Failure
        }
        InitError::Manifest(_) | InitError::Lock(_) | InitError::Hooks(_) => {
            crate::outcome::Outcome::Error
        }
    }
}

/// What a new product's author runs next, a line each.
pub(super) const NEXT: [&str; 2] = [
    "asymptote check                      run the tools against the lock",
    "asymptote up --reason <why>          accept what they find now, once",
];

/// What `init` prints.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct InitReportSchema {
    /// `created` where the product had no `asymptote.toml`, `kept` where it had one.
    pub configuration: String,
    /// The hooks whose script is written, or would be.
    pub installed: Vec<String>,
    /// The hooks whose script of asymptote's is removed, or would be.
    pub removed: Vec<String>,
    /// Whether anything was written.
    pub written: bool,
}

impl std::fmt::Display for InitReportSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let verb = if self.written { "wrote" } else { "would write" };
        if self.configuration == "created" {
            writeln!(f, "{verb} asymptote.toml and an empty lock")?;
        } else {
            writeln!(f, "kept asymptote.toml: it is there, and is left as it is")?;
        }
        for hook in &self.installed {
            writeln!(f, "{verb} the {hook} hook")?;
        }
        let removal = if self.written {
            "removed"
        } else {
            "would remove"
        };
        for hook in &self.removed {
            writeln!(f, "{removal} the {hook} hook: it is no longer in [hooks]")?;
        }
        if self.written {
            for step in NEXT {
                writeln!(f, "next: {step}")?;
            }
        }
        Ok(())
    }
}

/// The answer printed for the service's report.
pub(super) fn answer(report: &InitReport) -> InitReportSchema {
    report.into()
}

impl From<&InitReport> for InitReportSchema {
    fn from(report: &InitReport) -> Self {
        Self {
            configuration: match report.configuration {
                Configuration::Created => "created",
                Configuration::Kept => "kept",
            }
            .to_owned(),
            installed: report
                .installed
                .iter()
                .map(|hook| hook.0.to_string())
                .collect(),
            removed: report
                .removed
                .iter()
                .map(|hook| hook.0.to_string())
                .collect(),
            written: report.writing == Writing::Write,
        }
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

impl crate::output::Encoded for InitReportSchema {
    fn json(
        &self,
        to: &mut dyn std::io::Write,
        spacing: crate::output::Spacing,
    ) -> std::io::Result<()> {
        match spacing {
            crate::output::Spacing::Compact => serde_json::to_writer(to, self),
            crate::output::Spacing::Indented => serde_json::to_writer_pretty(to, self),
        }
        .map_err(std::io::Error::from)
    }
}
