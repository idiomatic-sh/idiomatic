//! What a certification verified and what it left unassessed, as the measure commands announce it.
use super::super::super::constants::{ACCOUNTED, INTERIM};
use super::super::rule_tally::value::RuleTally;
use crate::domain::suite::types::{audit::value::Audit, certificate::value::Certificate};
use ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage;
use std::path::Path;

/// The certification line: its scope, the verified subset against the tool's tally, and the certificate and artifacts it issued.
#[derive(Clone, Copy, Debug)]
pub struct CertificationSummary<'a> {
    /// The scope the run is held to.
    pub scope: BenchmarkCoverage,
    /// The tool's defaults, deferrals, pending rules and authored total.
    pub tally: &'a RuleTally,
    /// The audit the run certified.
    pub audit: &'a Audit,
    /// The certificate issued.
    pub certificate: &'a Certificate,
    /// Where the certified profile was staged.
    pub profile: &'a Path,
}

impl std::fmt::Display for CertificationSummary<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let RuleTally {
            defaults,
            deferred,
            pending,
            authored,
        } = self.tally;
        write!(
            f,
            "scope: {}; {} verified / {} shipped defaults; {} assessed deferrals; {} assessed pending; {} unassessed; {} authored outside subset; {} cases; certificate={}; artifacts={}",
            match self.scope {
                BenchmarkCoverage::Accounted => ACCOUNTED,
                BenchmarkCoverage::Interim => INTERIM,
            },
            self.audit.authored_count(),
            defaults.0,
            deferred.0,
            pending.0,
            defaults.0 - authored.0 - deferred.0 - pending.0,
            authored.0 - self.audit.authored_count().0,
            self.certificate.cases(),
            self.certificate.fingerprint(),
            self.profile.display()
        )
    }
}
