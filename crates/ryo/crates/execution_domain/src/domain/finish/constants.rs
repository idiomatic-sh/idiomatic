//! Diagnostic text emitted for typed producer-publication refusals.
use super::types::PublicationReason;
use std::fmt;

impl fmt::Display for PublicationReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ReservedArena => {
                "cross publication requires an ordinary arena slot and fresh place log"
            }
            Self::CrossNameMismatch => "cross name does not match its producer symbol",
            Self::PlaceUnrepresentable => "name place is not representable",
            Self::PlaceMissing => "name place is absent from producer",
            Self::SourceTextMissing => "source-backed cross name has no retained source text",
            Self::StartUnrepresentable => "name start is not representable",
            Self::EndUnrepresentable => "name end is not representable",
            Self::LocalRangeMissing => "local cross name range is unavailable",
            Self::LocalHashMismatch => "local cross name hash changed",
            Self::LocalTextInvalid => "local cross name is not UTF-8",
            Self::NormalizedIdentityMismatch => "normalized cross name changed identity",
            Self::InheritedArenaUnrepresentable => "inherited arena is not representable",
            Self::InheritedRangeMissing => "inherited name arena or range is unavailable",
            Self::InheritedTextMismatch => "inherited name does not match its retained text",
            Self::PortablePlaceLogFull => "portable place log is full",
            Self::ProducerSymbolUnescaped => "a producer-local symbol did not escape",
            Self::CrossPlaceUnescaped => "a local cross place escaped",
            Self::EscapedPlaceUnrepresentable => "escaped place is not representable",
            Self::EscapedPlaceMissing => "escaped place is missing",
            Self::EscapedArenaUnstable => "escaped name has no stable ordinary arena",
        };
        formatter.write_str(message)
    }
}
