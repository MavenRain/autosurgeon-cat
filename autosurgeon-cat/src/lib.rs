//! Derive-driven mapping between Rust types and
//! [`automerge_cat`] documents.
//!
//! This crate provides two core traits:
//!
//! - [`Hydrate`] constructs a Rust value from a document
//!   [`Value`].
//! - [`Reconcile`] writes a Rust value into a
//!   [`Session`], performing smart
//!   diffing so only changed fields produce new CRDT operations.
//!
//! Both traits can be derived for structs, newtype wrappers, and
//! enums via `#[derive(Hydrate, Reconcile)]`.
//!
//! # Quick start
//!
//! ```rust,ignore
//! use autosurgeon_cat::{Hydrate, Reconcile};
//!
//! #[derive(Hydrate, Reconcile)]
//! struct Config {
//!     name: String,
//!     count: u64,
//!     enabled: bool,
//! }
//!
//! // Write to a document
//! let session = autosurgeon_cat::reconcile(
//!     &Config { name: "demo".into(), count: 42, enabled: true },
//!     &session,
//! )?;
//!
//! // Read back
//! let config: Config = autosurgeon_cat::hydrate(session.document())?;
//! ```

pub mod error;
pub mod hydrate;
pub mod reconcile;

// -- trait re-exports -------------------------------------------------------

pub use error::Error;

pub use hydrate::{Hydrate, HydrateRoot};
pub use reconcile::{Reconcile, ReconcileRoot};

// -- helper re-exports (used by derive-generated code) ----------------------

pub use hydrate::{
    expect_map, hydrate_field, hydrate_optional_field, resolve_key,
    try_resolve_key, value_type_name,
};
pub use reconcile::ensure_map;

// -- derive macro re-exports ------------------------------------------------

pub use autosurgeon_cat_derive::{Hydrate, Reconcile};

// -- re-exports from automerge-cat (used by derive-generated code) ----------

pub use automerge_cat::{Document, Float64, NodeId, Session, Value};

// ---------------------------------------------------------------------------
// Free functions
// ---------------------------------------------------------------------------

/// Hydrate a value from the document root.
///
/// The type must implement [`HydrateRoot`], which is auto-derived
/// for named structs.  Each struct field is read from a key in
/// the root map.
///
/// # Errors
///
/// Propagates field-level hydration errors.
pub fn hydrate<T: HydrateRoot>(doc: &automerge_cat::Document) -> Result<T, Error> {
    T::hydrate_root(doc)
}

/// Reconcile a value into the document root.
///
/// The type must implement [`ReconcileRoot`], which is auto-derived
/// for named structs.  Each struct field is written to a key in
/// the root map.
///
/// # Errors
///
/// Propagates field-level reconciliation errors.
pub fn reconcile<T: ReconcileRoot>(
    value: &T,
    session: &automerge_cat::Session,
) -> Result<automerge_cat::Session, Error> {
    value.reconcile_root(session)
}
