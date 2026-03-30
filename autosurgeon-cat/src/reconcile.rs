//! Reconcile: write Rust values into [`Session`] documents.
//!
//! The [`Reconcile`] trait provides two operations:
//!
//! - [`reconcile`](Reconcile::reconcile) writes a value at a
//!   map key, performing smart diffing so unchanged scalars are
//!   not re-written (preserving CRDT semantics).
//! - [`to_value`](Reconcile::to_value) converts a value into a
//!   document [`Value`], creating any needed container nodes in
//!   the session.
//!
//! [`ReconcileRoot`] handles the special case of writing a struct's
//! fields directly into the document root map.
//!
//! # Helpers
//!
//! - [`ensure_map`] finds or creates a map node at a given key.

use automerge_cat::{Float64, NodeId, Origin, Session, Tag, Value};

use crate::error::Error;

// ---------------------------------------------------------------------------
// Traits
// ---------------------------------------------------------------------------

/// Write `Self` into a document.
///
/// The two methods serve different call-sites:
///
/// - [`reconcile`](Self::reconcile) targets a specific key in a
///   map node (used by struct field reconciliation and the derive
///   macros).
/// - [`to_value`](Self::to_value) produces a standalone [`Value`]
///   (used when inserting elements into a list).
pub trait Reconcile {
    /// Write this value at `key` in the map node `node`.
    ///
    /// Implementations should skip the write when the document
    /// already holds an identical value (smart reconciliation).
    ///
    /// # Errors
    ///
    /// Propagates document-layer errors.
    fn reconcile(
        &self,
        session: &Session,
        node: NodeId,
        key: &str,
    ) -> Result<Session, Error>;

    /// Convert to a document [`Value`], potentially creating new
    /// container nodes in the session.
    ///
    /// # Errors
    ///
    /// Propagates document-layer errors from node creation.
    fn to_value(&self, session: &Session) -> Result<(Session, Value), Error>;
}

/// Reconcile `Self` directly into the document root map.
///
/// Auto-derived for named structs; each struct field is written
/// to a key in the root map.
pub trait ReconcileRoot {
    /// Write all fields into the root map of the session's
    /// document.
    ///
    /// # Errors
    ///
    /// Propagates field-level reconciliation errors.
    fn reconcile_root(&self, session: &Session) -> Result<Session, Error>;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Ensure a map container exists at `key` in the map node `node`.
///
/// If the key already points to a [`Value::Map`], returns its
/// [`NodeId`].  Otherwise creates a new empty map and sets the
/// key.
///
/// # Errors
///
/// Propagates document-layer errors.
pub fn ensure_map(
    session: &Session,
    node: NodeId,
    key: &str,
) -> Result<(Session, NodeId), Error> {
    let values = session.document().get_key(node, key)?;
    values
        .iter()
        .find_map(|v| match v {
            Value::Map(id) => Some(*id),
            _ => None,
        })
        .map_or_else(
            || {
                let (session, id) = session.create_map()?;
                session
                    .set_key(node, key, &Value::Map(id))
                    .map(|s| (s, id))
                    .map_err(Error::from)
            },
            |id| Ok((session.clone(), id)),
        )
}

/// Write a scalar value, skipping the write when the document
/// already holds the identical value.
fn reconcile_scalar(
    session: &Session,
    node: NodeId,
    key: &str,
    target: &Value,
) -> Result<Session, Error> {
    let already_matches = session
        .document()
        .get_key(node, key)
        .is_ok_and(|values| {
            values.len() == 1 && values.contains(target)
        });
    if already_matches {
        Ok(session.clone())
    } else {
        session.set_key(node, key, target).map_err(Error::from)
    }
}

// ---------------------------------------------------------------------------
// Primitive impls
// ---------------------------------------------------------------------------

impl Reconcile for bool {
    fn reconcile(
        &self,
        session: &Session,
        node: NodeId,
        key: &str,
    ) -> Result<Session, Error> {
        reconcile_scalar(session, node, key, &Value::Bool(*self))
    }

    fn to_value(&self, session: &Session) -> Result<(Session, Value), Error> {
        Ok((session.clone(), Value::Bool(*self)))
    }
}

impl Reconcile for i64 {
    fn reconcile(
        &self,
        session: &Session,
        node: NodeId,
        key: &str,
    ) -> Result<Session, Error> {
        reconcile_scalar(session, node, key, &Value::Int(*self))
    }

    fn to_value(&self, session: &Session) -> Result<(Session, Value), Error> {
        Ok((session.clone(), Value::Int(*self)))
    }
}

impl Reconcile for u64 {
    fn reconcile(
        &self,
        session: &Session,
        node: NodeId,
        key: &str,
    ) -> Result<Session, Error> {
        reconcile_scalar(session, node, key, &Value::Uint(*self))
    }

    fn to_value(&self, session: &Session) -> Result<(Session, Value), Error> {
        Ok((session.clone(), Value::Uint(*self)))
    }
}

impl Reconcile for f64 {
    fn reconcile(
        &self,
        session: &Session,
        node: NodeId,
        key: &str,
    ) -> Result<Session, Error> {
        reconcile_scalar(session, node, key, &Value::Float(Float64::new(*self)))
    }

    fn to_value(&self, session: &Session) -> Result<(Session, Value), Error> {
        Ok((session.clone(), Value::Float(Float64::new(*self))))
    }
}

impl Reconcile for Float64 {
    fn reconcile(
        &self,
        session: &Session,
        node: NodeId,
        key: &str,
    ) -> Result<Session, Error> {
        reconcile_scalar(session, node, key, &Value::Float(*self))
    }

    fn to_value(&self, session: &Session) -> Result<(Session, Value), Error> {
        Ok((session.clone(), Value::Float(*self)))
    }
}

impl Reconcile for String {
    fn reconcile(
        &self,
        session: &Session,
        node: NodeId,
        key: &str,
    ) -> Result<Session, Error> {
        reconcile_scalar(session, node, key, &Value::Str(self.clone()))
    }

    fn to_value(&self, session: &Session) -> Result<(Session, Value), Error> {
        Ok((session.clone(), Value::Str(self.clone())))
    }
}

impl<T: Reconcile> Reconcile for Option<T> {
    fn reconcile(
        &self,
        session: &Session,
        node: NodeId,
        key: &str,
    ) -> Result<Session, Error> {
        self.as_ref().map_or_else(
            || {
                let values = session.document().get_key(node, key)?;
                if values.is_empty() {
                    Ok(session.clone())
                } else {
                    session.delete_key(node, key).map_err(Error::from)
                }
            },
            |v| v.reconcile(session, node, key),
        )
    }

    fn to_value(&self, session: &Session) -> Result<(Session, Value), Error> {
        self.as_ref().map_or_else(
            || Ok((session.clone(), Value::Null)),
            |v| v.to_value(session),
        )
    }
}

impl<T: Reconcile> Reconcile for Vec<T> {
    fn reconcile(
        &self,
        session: &Session,
        node: NodeId,
        key: &str,
    ) -> Result<Session, Error> {
        let (session, value) = self.to_value(session)?;
        session.set_key(node, key, &value).map_err(Error::from)
    }

    fn to_value(&self, session: &Session) -> Result<(Session, Value), Error> {
        let (session, list_id) = session.create_list()?;
        let session = self
            .iter()
            .try_fold((session, Origin::Head), |(session, prev), elem| {
                let (session, value) = elem.to_value(&session)?;
                let tag = Tag::new(session.replica(), session.clock());
                session
                    .list_insert(list_id, prev, value)
                    .map(|s| (s, Origin::After(tag)))
                    .map_err(Error::from)
            })
            .map(|(s, _)| s)?;
        Ok((session, Value::List(list_id)))
    }
}
