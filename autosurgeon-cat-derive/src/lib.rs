//! Derive macros for `autosurgeon-cat`.
//!
//! Provides `#[derive(Hydrate)]` and `#[derive(Reconcile)]` for
//! mapping Rust types to and from `automerge-cat` documents.
//!
//! # Supported shapes
//!
//! - **Named structs** map to document map nodes (each field is a key).
//! - **Newtype structs** (`struct Foo(Bar)`) delegate to the inner type.
//! - **Enums** use string values for unit variants and single-key maps
//!   for struct/newtype variants.

mod hydrate;
mod reconcile;

/// Derive the `Hydrate` trait (and `HydrateRoot` for named structs).
///
/// Named structs also get `HydrateRoot`, which reads fields
/// directly from the document root map.
#[proc_macro_derive(Hydrate)]
pub fn derive_hydrate(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    hydrate::expand(&input).into()
}

/// Derive the `Reconcile` trait (and `ReconcileRoot` for named structs).
///
/// Named structs also get `ReconcileRoot`, which writes fields
/// directly into the document root map.
#[proc_macro_derive(Reconcile)]
pub fn derive_reconcile(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    reconcile::expand(&input).into()
}
