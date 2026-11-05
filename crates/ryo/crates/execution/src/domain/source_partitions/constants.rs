//! Diagnostic rendering for partition declaration invariants.
use super::types::PartitionRefusal;
impl std::fmt::Display for PartitionRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::BindingOrdinal => "observation binding ordinal is not its declared slot",
            Self::MissingDeclaration => "observation input has no producer declaration",
            Self::UndeclaredInput => "observation input ordinal is undeclared",
            Self::MissingConsumer => "observation has no authored consumer declaration",
            Self::ConsumerMismatch => "observation does not match its actual consumer declaration",
            Self::MissingQuery => "observation producer has no executable query",
            Self::MissingRule => "observation producer rule is missing",
            Self::MissingOutput => "observation producer output ordinal is undeclared",
            Self::OutputMismatch => "observation producer differs from its authored output",
            Self::MissingStep => "observation producer root step is missing",
            Self::IndeterminatePhase => "relation has no concrete producer phase",
            Self::MissingBinding => "observation producer root binding is missing",
            Self::PhaseMismatch => "observation phase disagrees with its actual producer root",
        })
    }
}
