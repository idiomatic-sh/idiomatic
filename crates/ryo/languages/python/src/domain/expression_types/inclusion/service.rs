//! Type inclusion between basic types, and the union of two branch resolutions that keeps the wider of two included types.
use crate::domain::expression_types::types::{BasicType, Resolution, TypeInclusion};

/// Check whether the left type is included in the right type.
fn subtype(left: BasicType, right: BasicType) -> TypeInclusion {
    if left == right
        || matches!(
            (left, right),
            (
                BasicType::Boolean,
                BasicType::Integer | BasicType::Float | BasicType::Complex
            ) | (BasicType::Integer, BasicType::Float | BasicType::Complex)
                | (BasicType::Float, BasicType::Complex)
        )
    {
        TypeInclusion::Included
    } else {
        TypeInclusion::Separate
    }
}

/// Combine branch results while preserving known subtype relationships.
pub(in super::super) fn union(left: Resolution, right: Resolution) -> Resolution {
    match (left, right) {
        (Resolution::TypeError, _) | (_, Resolution::TypeError) => Resolution::TypeError,
        (Resolution::Unknown, _) | (_, Resolution::Unknown) => Resolution::Unknown,
        (Resolution::Atom(left_kind), Resolution::Atom(right_kind)) => {
            if subtype(left_kind, right_kind) == TypeInclusion::Included {
                Resolution::Atom(right_kind)
            } else if subtype(right_kind, left_kind) == TypeInclusion::Included {
                Resolution::Atom(left_kind)
            } else {
                Resolution::Union(vec![left_kind, right_kind])
            }
        }
        (Resolution::Atom(left_kind), Resolution::Union(mut right_types)) => {
            if !right_types
                .iter()
                .any(|candidate| subtype(left_kind, *candidate) == TypeInclusion::Included)
            {
                right_types.push(left_kind);
            }
            Resolution::Union(right_types)
        }
        (Resolution::Union(mut left_types), Resolution::Atom(right_kind)) => {
            if !left_types
                .iter()
                .any(|candidate| subtype(right_kind, *candidate) == TypeInclusion::Included)
            {
                left_types.push(right_kind);
            }
            Resolution::Union(left_types)
        }
        (Resolution::Union(mut left_types), Resolution::Union(right_types)) => {
            for candidate in right_types {
                if !left_types
                    .iter()
                    .any(|known| subtype(candidate, *known) == TypeInclusion::Included)
                {
                    left_types.push(candidate);
                }
            }
            Resolution::Union(left_types)
        }
    }
}
