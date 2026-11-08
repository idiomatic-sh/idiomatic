//! Generation, request and partition identity: what a partition supplies and who it matches.
use super::super::{
    Agreement, ApplicabilityProof, FileGeneration, Partition, PartitionSupply,
    RequiredPartitions,
};
use crate::application::file_dag::relations::RelationOwner;
use ryo_execution_domain::domain::{
    coverage::types::RuleIndex, operating::errors::OperatingError,
    rule_intent::types::ApplicabilityReason,
};
use ryo_relations::domain::relations::types::FileId;
use std::sync::Arc;

impl FileGeneration {
    pub(in crate::application::file_dag::program) fn matches(
        &self,
        expected: &Self,
        generation: &Arc<()>,
    ) -> Agreement {
        if self.file == expected.file
            && Arc::ptr_eq(&self.generation, &expected.generation)
            && Arc::ptr_eq(generation, &expected.generation)
        {
            Agreement::Same
        } else {
            Agreement::Different
        }
    }
    pub(in crate::application::file_dag::program) fn file(&self) -> FileId {
        self.file
    }
    pub(in crate::application::file_dag::program) fn from_owner(
        file: FileId,
        owner: &RelationOwner,
    ) -> Self {
        Self {
            file,
            generation: owner.generation_guard(),
        }
    }
}

impl RequiredPartitions {
    pub(in crate::application::file_dag::program) fn from_request(
        request: &Arc<crate::application::file_dag::program::request_identity::RequestIdentity>,
        files: &[ryo_values::domain::files::types::CorpusInput],
        covered: &[ryo_execution_domain::domain::coverage::types::Covered],
        skipped: &[ryo_values::domain::runs::types::Skipped],
        inapplicable: &[FileId],
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
        })
    }
    /// An explicit syntax rejection discharges transport, never completeness.
    /// Missing, unreadable, or unknown files cannot acquire this evidence.
    pub(in crate::application::file_dag) fn skipped_files(
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
    pub(in crate::application) fn files(&self) -> &[FileId] {
        &self.files
    }
    /// The files the edge named as having no frame for the rule: ungoverned, or read by no language.
    pub(in crate::application) fn inapplicable(&self) -> &[FileId] {
        &self.inapplicable
    }
    /// The project identity alone, as a shared execution holds it: no rule's evidence.
    pub(in crate::application::file_dag::program) fn identity(&self) -> Self {
        Self {
            inapplicable: Box::new([]),
            ..self.clone()
        }
    }
}

impl Partition {
    pub(in crate::application::file_dag::program) fn require_supply(
        &self,
        required: &Self,
    ) -> Result<(), OperatingError> {
        match self.supplies(required) {
            PartitionSupply::Supplies => Ok(()),
            PartitionSupply::Lacks => Err(OperatingError::NotRead),
        }
    }
    pub(in crate::application::file_dag::program) fn supplies(
        &self,
        required: &Self,
    ) -> PartitionSupply {
        let supplied = match (self, required) {
            (Self::RequestInvariant, _) => true,
            (Self::File(actual), Self::File(required)) => {
                actual.file == required.file
                    && Arc::ptr_eq(&actual.generation, &required.generation)
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
    pub(in crate::application::file_dag) fn same_request(&self, other: &Self) -> Agreement {
        if self == other {
            Agreement::Same
        } else {
            Agreement::Different
        }
    }
}

impl ApplicabilityProof {
    pub(in crate::application::file_dag::program) fn for_file(
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
