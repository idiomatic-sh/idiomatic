#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]

/// Declares a fieldless kind and every one of its variants once: `ALL` is generated from the same list as the enum, so a kind
/// cannot be added and left out of `ALL`.
macro_rules! kinds {
    ($(#[$meta:meta])* $vis:vis enum $kind:ident { $( $(#[$doc:meta])* $variant:ident, )+ }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        $vis enum $kind {
            $( $(#[$doc])* $variant, )+
        }

        impl $kind {
            /// Every variant, in declaration order: generated from the enum's own list.
            $vis const ALL: &'static [Self] = &[ $( Self::$variant, )+ ];
        }
    };
}

pub mod application;
pub mod domain;
pub mod inbound;
pub mod outbound;

#[cfg(any(test, feature = "fixtures"))]
pub mod cases;

/// The plan's one list of operators: generates `Lowering`, the node of every listed operator, and its dispatches. Each entry's
/// node type must implement `domain::growth::types::Operator` (E0277 otherwise), and each variant must be a `NodeKind`
/// (E0599 otherwise), so no operator is planned without its growth declaration and no kind the counters report drifts from
/// the list. In a test build each node type must also give its instances.
///
/// # Examples
///
/// A node type without the contract does not compile:
///
/// ```compile_fail,E0277
/// #[derive(Clone, Debug, PartialEq)]
/// struct Bare;
/// ryo_operators::lowerings! { Bare(Bare) }
/// ```
#[macro_export]
#[doc(hidden)]
macro_rules! lowerings {
    ($( $(#[$doc:meta])* $variant:ident($node:ty) ),+ $(,)?) => {
        /// The kernels one node of the physical plan is lowered to: one variant per operator of the one list.
        #[derive(Clone, Debug, PartialEq)]
        pub(crate) enum Lowering {
            $( $(#[$doc])* $variant($node), )+
        }

        // Every listed node type declares its growth, in every build: each declaration is read as a function, which only an
        // operator has.
        $(
            const _: fn(&$node) -> $crate::domain::growth::types::Growth =
                <$node as $crate::domain::growth::types::Operator>::growth;
        )+

        impl Lowering {
            /// The kind of its kernels, as the node counters report it.
            pub(crate) fn kind(&self) -> $crate::domain::physical_plan::types::node_counters::value::NodeKind {
                match self {
                    $( Self::$variant(_) => $crate::domain::physical_plan::types::node_counters::value::NodeKind::$variant, )+
                }
            }

            /// How its rows grow: its operator's declaration for this instance, which the run budget reads of every making.
            pub(crate) fn growth(&self) -> $crate::domain::growth::types::Growth {
                match self {
                    $( Self::$variant(node) => <$node as $crate::domain::growth::types::Operator>::growth(node), )+
                }
            }

            /// Every listed operator's instances over `columns`, each with its kind.
            #[cfg(test)]
            pub(crate) fn witnesses(
                columns: $crate::domain::growth::types::WitnessColumns,
            ) -> ::std::vec::Vec<(
                $crate::domain::physical_plan::types::node_counters::value::NodeKind,
                $crate::domain::growth::types::Witnessed<Self>,
            )> {
                let mut all = ::std::vec::Vec::new();
                $(
                    all.extend(
                        <$node as $crate::domain::growth::types::Witnesses>::witnesses(columns)
                            .into_iter()
                            .map(|witness| {
                                (
                                    $crate::domain::physical_plan::types::node_counters::value::NodeKind::$variant,
                                    $crate::domain::growth::types::Witnessed {
                                        node: Self::$variant(witness.node),
                                        roles: witness.roles,
                                    },
                                )
                            }),
                    );
                )+
                all
            }
        }
    };
}
