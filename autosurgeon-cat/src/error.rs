//! Project-wide error type.

use automerge_cat::NodeId;

/// Errors produced during hydration or reconciliation.
#[derive(Debug)]
pub enum Error {
    /// An error from the underlying `automerge-cat` document layer.
    Document(automerge_cat::Error),
    /// A map key has no observed values.
    MissingKey {
        /// The map node that was queried.
        node: NodeId,
        /// The key that was not found.
        key: String,
    },
    /// A map key has multiple concurrent values (unresolved conflict).
    UnresolvedConcurrency {
        /// The map node containing the conflict.
        node: NodeId,
        /// The key with multiple values.
        key: String,
    },
    /// Expected a specific [`Value`](automerge_cat::Value) variant but found another.
    TypeMismatch {
        /// The expected variant name.
        expected: &'static str,
        /// The variant name that was found.
        found: &'static str,
    },
    /// An unknown enum variant name was encountered during hydration.
    UnknownVariant {
        /// The variant name that was not recognised.
        variant: String,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Document(e) => write!(f, "document error: {e}"),
            Self::MissingKey { node, key } => {
                write!(f, "missing key \"{key}\" in node {node:?}")
            }
            Self::UnresolvedConcurrency { node, key } => {
                write!(
                    f,
                    "unresolved concurrent values at \"{key}\" in node {node:?}"
                )
            }
            Self::TypeMismatch { expected, found } => {
                write!(f, "type mismatch: expected {expected}, found {found}")
            }
            Self::UnknownVariant { variant } => {
                write!(f, "unknown enum variant: \"{variant}\"")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Document(e) => Some(e),
            Self::MissingKey { .. }
            | Self::UnresolvedConcurrency { .. }
            | Self::TypeMismatch { .. }
            | Self::UnknownVariant { .. } => None,
        }
    }
}

impl From<automerge_cat::Error> for Error {
    fn from(e: automerge_cat::Error) -> Self {
        Self::Document(e)
    }
}
