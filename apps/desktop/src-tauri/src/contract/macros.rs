//! `wire_enum!`: a string enum of the contract together with the list of
//! all its values (`ALL`). The fixtures export `ALL`, and the TypeScript
//! parity test compares it with the TypeScript list in both directions
//! (ADR-0018).

/// Defines a unit-only enum that travels as a camelCase string.
macro_rules! wire_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $($(#[$vmeta:meta])* $variant:ident),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        $vis enum $name {
            $($(#[$vmeta])* $variant),*
        }

        impl $name {
            /// Every value, in declaration order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),*];
        }
    };
}

pub(crate) use wire_enum;
