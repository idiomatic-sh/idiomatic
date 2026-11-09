//! Generation, request and partition identity: what a partition supplies and who it matches.
use ryo_execution_store::application::file_dag::relations::interface::RelationStore as _;
use super::super::{
    Agreement, ApplicabilityProof, FileGeneration, Partition, PartitionSupply, RequiredPartitions,
};
use ryo_execution_domain::domain::{
    coverage::types::RuleIndex, operating::errors::OperatingError,
    rule_intent::types::ApplicabilityReason,
};
use ryo_execution_store::application::file_dag::relations::RelationOwner;
use ryo_relations::domain::relations::types::FileId;
use std::sync::Arc;

impl FileGeneration {
    pub(crate) fn matches(
        &self,
        expected: &Self,
        generation: &ryo_execution_store::application::file_dag::relations::types::Generation,
    ) -> Agreement {
        if self.file == expected.file
            && self.generation.same(&expected.generation).is_ok()
            && generation.same(&expected.generation).is_ok()
        {
            Agreement::Same
        } else {
            Agreement::Different
        }
    }
    pub(crate) fn file(&self) -> FileId {
        self.file
    }
    pub(crate) fn from_owner(file: FileId, owner: &RelationOwner) -> Self {
        Self {
            file,
            generation: owner.generation(),
        }
    }
}

impl RequiredPartitions {
    pub(crate) fn from_request(
        request: &Arc<crate::application::file_dag::program::request_identity::RequestIdentity>,
        files: &[ryo_values::domain::files::types::CorpusInput],
        covered: &[ryo_execution_domain::domain::coverage::types::Covered],
        skipped: &[ryo_values::domain::runs::types::Skipped],
        inapplicable: &[FileId],
        refused: &[FileId],
    ) -> Result<Self, OperatingError> {
        if request.corpus() != files {
            return Err(OperatingError::NotRead);
        }
        let mut ids = Vec::with_capacity(files.len());
        for (at, _) in files.iter().enumerate() {
            ids.push(FileId(
                u32::try_from(at).map_err(|_| OperatingError::ShapeMismatch)?,
            ));
        }
        if covered
            .iter()
            .any(|entry| entry.file.0 as usize >= files.len())
            || inapplicable
                .iter()
                .chain(refused)
                .any(|file| file.0 as usize >= files.len())
        {
            return Err(OperatingError::NotRead);
        }
        Ok(Self {
            request: Arc::clone(request),
            files: ids.into_boxed_slice(),
            covered: covered.to_vec().into_boxed_slice(),
            skipped: skipped.to_vec().into_boxed_slice(),
            inapplicable: inapplicable.to_vec().into_boxed_slice(),
            refused: refused.to_vec().into_boxed_slice(),
        })
    }
    /// An explicit syntax rejection discharges transport, never completeness.
    /// Missing, unreadable, or unknown files cannot acquire this evidence.
    pub(crate) fn skipped_files(
        &self,
        program: &crate::application::file_dag::program::preparation::types::PreparedProgram,
        rule: RuleIndex,
    ) -> Result<std::collections::BTreeSet<FileId>, OperatingError> {
        use ryo_values::domain::runs::types::{RuleApplicability, SkipReason};
        let id = program.rule_id(rule)?;
        let mut files = std::collections::BTreeSet::new();
        for skipped in self.skipped.iter().filter(|skipped| &skipped.rule == id) {
            if program.applicability(rule)? != RuleApplicability::AcceptedSyntax
                || skipped.reason != SkipReason::SyntaxRejected
            {
                return Err(OperatingError::NotRead);
            }
            let file = self
                .request
                .available_file(&skipped.path)
                .ok_or(OperatingError::NotRead)?;
            if !files.insert(file) {
                return Err(OperatingError::NotRead);
            }
        }
        Ok(files)
    }
    pub(crate) fn files(&self) -> &[FileId] {
        &self.files
    }
    /// The files the edge named as having no frame for the rule: ungoverned, or read by no language.
    pub(crate) fn inapplicable(&self) -> &[FileId] {
        &self.inapplicable
    }
    /// The files the edge named as refused to the rule: their syntax acceptance undetermined, or the rule's applicability
    /// metadata unavailable. The rule has no frame for them and lacks their input.
    pub(crate) fn refused(&self) -> &[FileId] {
        &self.refused
    }
    /// The project identity alone, as a shared execution holds it: no rule's evidence.
    pub(crate) fn identity(&self) -> Self {
        Self {
            inapplicable: Box::new([]),
            refused: Box::new([]),
            ..self.clone()
        }
    }
}

impl Partition {
    pub(crate) fn require_supply(&self, required: &Self) -> Result<(), OperatingError> {
        match self.supplies(required) {
            PartitionSupply::Supplies => Ok(()),
            PartitionSupply::Lacks => Err(OperatingError::NotRead),
        }
    }
    pub(crate) fn supplies(&self, required: &Self) -> PartitionSupply {
        let supplied = match (self, required) {
            (Self::RequestInvariant, _) => true,
            (Self::File(actual), Self::File(required)) => {
                actual.file == required.file
                    && actual.generation.same(&required.generation).is_ok()
            }
            (Self::Project(actual), Self::Project(required)) => actual == required,
            _ => false,
        };
        if supplied {
            PartitionSupply::Supplies
        } else {
            PartitionSupply::Lacks
        }
    }
}
impl RequiredPartitions {
    /// Whether both name one project identity; the files a rule has no frame for are that rule's
    /// own evidence and are not compared.
    pub(crate) fn same_request(&self, other: &Self) -> Agreement {
        if self == other {
            Agreement::Same
        } else {
            Agreement::Different
        }
    }
}

impl ApplicabilityProof {
    pub(crate) fn for_file(
        producer: RuleIndex,
        file: FileId,
        reason: ApplicabilityReason,
        owner: &RelationOwner,
    ) -> Self {
        Self {
            producer,
            file: FileGeneration::from_owner(file, owner),
            reason,
        }
    }
}

impl ApplicabilityProof {
    /// The declared producer for this positive applicability result.
    pub fn producer(&self) -> RuleIndex {
        self.producer
    }
    /// The file whose generation issued this fact; this cannot mint a new capability.
    pub fn file(&self) -> FileId {
        self.file.file
    }
    pub fn reason(&self) -> ApplicabilityReason {
        self.reason
    }
}
