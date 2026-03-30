//! Hydrate: construct Rust values from [`Document`] contents.
//!
//! The [`Hydrate`] trait maps a document [`Value`] to a Rust type.
//! [`HydrateRoot`] handles the special case of hydrating a struct
//! directly from the document root map.
//!
//! # Helpers
//!
//! - [`resolve_key`] / [`try_resolve_key`] resolve a map key to a
//!   single value (erroring on concurrent-write conflicts).
//! - [`hydrate_field`] / [`hydrate_optional_field`] combine key
//!   resolution with hydration in one step.  The derive macros
//!   emit calls to these helpers.
//! - [`expect_map`] extracts a [`NodeId`] from a [`Value::Map`],
//!   returning [`Error::TypeMismatch`] otherwise.
//! - [`value_type_name`] returns a human-readable name for a
//!   [`Value`] variant (used in error messages).

use automerge_cat::{Document, Float64, NodeId, Value};

use crate::error::Error;

// ---------------------------------------------------------------------------
// Traits
// ---------------------------------------------------------------------------

/// Construct `Self` from a document [`Value`].
///
/// Scalars extract directly; compound types (structs, enums)
/// expect [`Value::Map`] and read their fields from the nested
/// map node.
pub trait Hydrate: Sized {
    /// Hydrate from a single resolved document value.
    ///
    /// # Errors
    ///
    /// Returns [`Error::TypeMismatch`] when the value variant does
    /// not match what this type expects.
    fn hydrate(doc: &Document, value: &Value) -> Result<Self, Error>;
}

/// Hydrate `Self` directly from the document root map.
///
/// Auto-derived for named structs; each struct field is read
/// from a key in the root map.
pub trait HydrateRoot: Sized {
    /// Read all fields from the root map of `doc`.
    ///
    /// # Errors
    ///
    /// Propagates any field-level hydration errors.
    fn hydrate_root(doc: &Document) -> Result<Self, Error>;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Human-readable name for a [`Value`] variant.
#[must_use]
pub fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "Null",
        Value::Bool(_) => "Bool",
        Value::Int(_) => "Int",
        Value::Uint(_) => "Uint",
        Value::Float(_) => "Float",
        Value::Str(_) => "Str",
        Value::Map(_) => "Map",
        Value::List(_) => "List",
        Value::Text(_) => "Text",
    }
}

/// Extract a [`NodeId`] from a [`Value::Map`].
///
/// # Errors
///
/// Returns [`Error::TypeMismatch`] if `value` is not a map reference.
pub fn expect_map(value: &Value) -> Result<NodeId, Error> {
    match value {
        Value::Map(id) => Ok(*id),
        other => Err(Error::TypeMismatch {
            expected: "Map",
            found: value_type_name(other),
        }),
    }
}

/// Try to resolve a map key to a single value.
///
/// - Empty register (key absent or deleted) => `Ok(None)`
/// - Exactly one value => `Ok(Some(value))`
/// - Multiple concurrent values => `Err(UnresolvedConcurrency)`
///
/// # Errors
///
/// - [`Error::UnresolvedConcurrency`] on multiple values.
/// - [`Error::Document`] if the node is missing or wrong type.
pub fn try_resolve_key<'a>(
    doc: &'a Document,
    node: NodeId,
    key: &str,
) -> Result<Option<&'a Value>, Error> {
    let values = doc.get_key(node, key)?;
    match values.len() {
        0 => Ok(None),
        1 => values
            .into_iter()
            .next()
            .map(Some)
            .ok_or_else(|| Error::MissingKey {
                node,
                key: key.to_string(),
            }),
        _ => Err(Error::UnresolvedConcurrency {
            node,
            key: key.to_string(),
        }),
    }
}

/// Resolve a map key to exactly one value.
///
/// # Errors
///
/// - [`Error::MissingKey`] if the key has no values.
/// - [`Error::UnresolvedConcurrency`] on multiple values.
/// - [`Error::Document`] if the node is missing or wrong type.
pub fn resolve_key<'a>(
    doc: &'a Document,
    node: NodeId,
    key: &str,
) -> Result<&'a Value, Error> {
    try_resolve_key(doc, node, key)?.ok_or_else(|| Error::MissingKey {
        node,
        key: key.to_string(),
    })
}

/// Resolve a key and hydrate the value.
///
/// Convenience wrapper used by derive-generated code.
///
/// # Errors
///
/// Propagates resolution and hydration errors.
pub fn hydrate_field<T: Hydrate>(
    doc: &Document,
    node: NodeId,
    key: &str,
) -> Result<T, Error> {
    resolve_key(doc, node, key).and_then(|v| T::hydrate(doc, v))
}

/// Resolve a key and hydrate, treating an absent key as `None`.
///
/// Convenience wrapper used by derive-generated code for
/// `Option<T>` fields.
///
/// # Errors
///
/// Propagates resolution (on conflict) and hydration errors.
pub fn hydrate_optional_field<T: Hydrate>(
    doc: &Document,
    node: NodeId,
    key: &str,
) -> Result<Option<T>, Error> {
    try_resolve_key(doc, node, key)?
        .map(|v| T::hydrate(doc, v))
        .transpose()
}

// ---------------------------------------------------------------------------
// Primitive impls
// ---------------------------------------------------------------------------

impl Hydrate for bool {
    fn hydrate(_doc: &Document, value: &Value) -> Result<Self, Error> {
        match value {
            Value::Bool(b) => Ok(*b),
            other => Err(Error::TypeMismatch {
                expected: "Bool",
                found: value_type_name(other),
            }),
        }
    }
}

impl Hydrate for i64 {
    fn hydrate(_doc: &Document, value: &Value) -> Result<Self, Error> {
        match value {
            Value::Int(n) => Ok(*n),
            other => Err(Error::TypeMismatch {
                expected: "Int",
                found: value_type_name(other),
            }),
        }
    }
}

impl Hydrate for u64 {
    fn hydrate(_doc: &Document, value: &Value) -> Result<Self, Error> {
        match value {
            Value::Uint(n) => Ok(*n),
            other => Err(Error::TypeMismatch {
                expected: "Uint",
                found: value_type_name(other),
            }),
        }
    }
}

impl Hydrate for f64 {
    fn hydrate(_doc: &Document, value: &Value) -> Result<Self, Error> {
        match value {
            Value::Float(fl) => Ok(fl.value()),
            other => Err(Error::TypeMismatch {
                expected: "Float",
                found: value_type_name(other),
            }),
        }
    }
}

impl Hydrate for Float64 {
    fn hydrate(_doc: &Document, value: &Value) -> Result<Self, Error> {
        match value {
            Value::Float(fl) => Ok(*fl),
            other => Err(Error::TypeMismatch {
                expected: "Float",
                found: value_type_name(other),
            }),
        }
    }
}

impl Hydrate for String {
    fn hydrate(_doc: &Document, value: &Value) -> Result<Self, Error> {
        match value {
            Value::Str(s) => Ok(s.clone()),
            other => Err(Error::TypeMismatch {
                expected: "Str",
                found: value_type_name(other),
            }),
        }
    }
}

impl<T: Hydrate> Hydrate for Option<T> {
    fn hydrate(doc: &Document, value: &Value) -> Result<Self, Error> {
        match value {
            Value::Null => Ok(None),
            v => T::hydrate(doc, v).map(Some),
        }
    }
}

impl<T: Hydrate> Hydrate for Vec<T> {
    fn hydrate(doc: &Document, value: &Value) -> Result<Self, Error> {
        match value {
            Value::List(node_id) => doc
                .list_elements(*node_id)?
                .into_iter()
                .map(|v| T::hydrate(doc, v))
                .collect(),
            other => Err(Error::TypeMismatch {
                expected: "List",
                found: value_type_name(other),
            }),
        }
    }
}
