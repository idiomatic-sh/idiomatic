//! The `kinds` service: what a language's kinds are made of, with no IO and no ports. A language's kind is one of its vocabularies,
//! each a field-less enum with a `u16` representation that derives `strum`'s `EnumCount`, `EnumIter`, `FromRepr` and `IntoStaticStr`:
//! its nodes, its tokens and, when it has any, its extras, the kinds its standard lacks. The element table holds a kind's number: the
//! nodes first, then the tokens, then the extras, each in the order of its enum; its name is its variant's in its vocabulary's
//! spelling. `kinds!` writes the four functions every language has over them, so that none writes the numbering by hand. A kind's
//! number is its variant's cast to `u16`: clippy leaves the casts of a macro to the crate that defines it, and this crate's own
//! expansions, in its tests, allow them.

/// Writes the numbering of the language's kind `$kind`, an enum whose variants are `Node($node)`, `Token($token)` and, when given,
/// `Extra($extra)`: `kind_bits`, the number the element table holds for a kind; `kind_of`, the kind a number stands for, `$unknown` for a
/// number no kind has; `kind_name`, its name; and `kinds`, every kind in the order of their numbers. `$vis` is their visibility.
#[macro_export]
macro_rules! kinds {
    ($vis:vis $kind:ident { Node($node:ident), Token($token:ident), Extra($extra:ident) $(,)? } unknown $unknown:expr) => {
        /// The number the element table holds for a kind: the nodes first, then the tokens, then the extras.
        #[must_use]
        $vis const fn kind_bits(kind: $kind) -> $crate::domain::relations::types::KindBits {
            const NODES: u16 = <$node as ::strum::EnumCount>::COUNT as u16;
            const EXTRAS: u16 = NODES + <$token as ::strum::EnumCount>::COUNT as u16;
            $crate::domain::relations::types::KindBits(match kind {
                $kind::Node(node) => node as u16,
                $kind::Token(token) => NODES + token as u16,
                $kind::Extra(extra) => EXTRAS + extra as u16,
            })
        }

        /// The kind the number stands for; the unknown kind for a number no kind has.
        #[must_use]
        $vis fn kind_of(bits: $crate::domain::relations::types::KindBits) -> $kind {
            let nodes = u16::try_from(<$node as ::strum::EnumCount>::COUNT).unwrap_or(u16::MAX);
            let extras = nodes.saturating_add(
                u16::try_from(<$token as ::strum::EnumCount>::COUNT).unwrap_or(u16::MAX),
            );
            let number = bits.0;
            let found = if number < nodes {
                $node::from_repr(number).map($kind::Node)
            } else if number < extras {
                $token::from_repr(number - nodes).map($kind::Token)
            } else {
                $extra::from_repr(number - extras).map($kind::Extra)
            };
            found.unwrap_or($unknown)
        }

        /// A kind's name, its vocabulary's: text that leaves the engine.
        #[must_use]
        $vis fn kind_name(kind: $kind) -> $crate::domain::terms::types::NameText<'static> {
            $crate::domain::terms::types::NameText(match kind {
                $kind::Node(node) => node.into(),
                $kind::Token(token) => token.into(),
                $kind::Extra(extra) => extra.into(),
            })
        }

        /// Every kind, in the order of their numbers.
        $vis fn kinds() -> impl Iterator<Item = $kind> {
            <$node as ::strum::IntoEnumIterator>::iter()
                .map($kind::Node)
                .chain(<$token as ::strum::IntoEnumIterator>::iter().map($kind::Token))
                .chain(<$extra as ::strum::IntoEnumIterator>::iter().map($kind::Extra))
        }
    };
    ($vis:vis $kind:ident { Node($node:ident), Token($token:ident) $(,)? } unknown $unknown:expr) => {
        /// The number the element table holds for a kind: the nodes first, then the tokens.
        #[must_use]
        $vis const fn kind_bits(kind: $kind) -> $crate::domain::relations::types::KindBits {
            const NODES: u16 = <$node as ::strum::EnumCount>::COUNT as u16;
            $crate::domain::relations::types::KindBits(match kind {
                $kind::Node(node) => node as u16,
                $kind::Token(token) => NODES + token as u16,
            })
        }

        /// The kind the number stands for; the unknown kind for a number no kind has.
        #[must_use]
        $vis fn kind_of(bits: $crate::domain::relations::types::KindBits) -> $kind {
            let nodes = u16::try_from(<$node as ::strum::EnumCount>::COUNT).unwrap_or(u16::MAX);
            let number = bits.0;
            let found = if number < nodes {
                $node::from_repr(number).map($kind::Node)
            } else {
                $token::from_repr(number - nodes).map($kind::Token)
            };
            found.unwrap_or($unknown)
        }

        /// A kind's name, its vocabulary's: text that leaves the engine.
        #[must_use]
        $vis fn kind_name(kind: $kind) -> $crate::domain::terms::types::NameText<'static> {
            $crate::domain::terms::types::NameText(match kind {
                $kind::Node(node) => node.into(),
                $kind::Token(token) => token.into(),
            })
        }

        /// Every kind, in the order of their numbers.
        $vis fn kinds() -> impl Iterator<Item = $kind> {
            <$node as ::strum::IntoEnumIterator>::iter()
                .map($kind::Node)
                .chain(<$token as ::strum::IntoEnumIterator>::iter().map($kind::Token))
        }
    };
}
