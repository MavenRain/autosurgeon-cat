# autosurgeon-cat

Derive-driven mapping between Rust types and
[automerge-cat](https://github.com/MavenRain/automerge-cat) CRDT documents,
built on [comp-cat-rs](https://github.com/MavenRain/comp-cat-theory).

autosurgeon-cat is to automerge-cat what
[autosurgeon](https://github.com/automerge/autosurgeon) is to automerge:
a serde-inspired layer that lets you derive `Hydrate` (document to Rust)
and `Reconcile` (Rust to document) for your domain types.

## Features

- **`#[derive(Hydrate, Reconcile)]`** for named structs, newtype wrappers,
  and enums (unit, newtype, and struct variants).
- **Smart reconciliation**: unchanged scalar fields are skipped, so only
  real changes produce new CRDT operations.  This preserves concurrent
  edits from other replicas.
- **Root-level free functions**: `hydrate(doc)` and `reconcile(value, session)`
  for reading/writing a struct at the document root.
- **Built-in impls** for `bool`, `i64`, `u64`, `f64`, `Float64`, `String`,
  `Option<T>`, and `Vec<T>`.

## Quick start

Add the dependency (path or git, until published):

```toml
[dependencies]
autosurgeon-cat = { path = "../autosurgeon-cat/autosurgeon-cat" }
```

Define your types and derive:

```rust
use autosurgeon_cat::{Hydrate, Reconcile};

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct Config {
    name: String,
    count: u64,
    enabled: bool,
}
```

Write to a document, then read back:

```rust
use automerge_cat::{ReplicaId, Session};

let session = Session::new(ReplicaId::new(1));

let original = Config {
    name: "demo".into(),
    count: 42,
    enabled: true,
};

// Reconcile into the document root
let session = autosurgeon_cat::reconcile(&original, &session)?;

// Hydrate from the document
let hydrated: Config = autosurgeon_cat::hydrate(session.document())?;
assert_eq!(original, hydrated);
```

## Document representation

| Rust type | Document representation |
|-----------|------------------------|
| Named struct `{ a, b }` | Map node with keys `"a"`, `"b"` |
| Newtype struct `Foo(T)` | Same as inner type `T` |
| `enum::Unit` | `Value::Str("Unit")` |
| `enum::Newtype(T)` | Map `{ "Newtype": <T> }` |
| `enum::Named { x }` | Map `{ "Named": { "x": ... } }` |
| `Vec<T>` | List node |
| `Option<T>` | `None` = absent key; `Some(v)` = value |
| `bool` / `i64` / `u64` / `f64` / `String` | Corresponding `Value` scalar |

## Smart reconciliation

When you reconcile the same struct twice, autosurgeon-cat compares each
field against the current document state:

- **Scalars**: if the value already matches (single resolved value, no
  concurrent conflicts), the write is skipped entirely.
- **Structs**: each field is reconciled independently, so changing one
  field does not touch the others.  This means concurrent edits to
  different fields by different replicas merge cleanly.
- **Vecs**: currently replaced wholesale (a new list node is created).
  Fine-grained list diffing is a planned improvement.

## Working with enums and nested types at non-root locations

For types that are not at the document root (e.g. an enum field, or a
value written at a specific map key), use the trait methods directly:

```rust
use autosurgeon_cat::{Hydrate, Reconcile};

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
enum Status { Active, Suspended }

// Write at a specific key
let root = session.document().root();
let session = Status::Active.reconcile(&session, root, "status")?;

// Read back
let value = autosurgeon_cat::resolve_key(session.document(), root, "status")?;
let status = Status::hydrate(session.document(), value)?;
```

## Architecture

The crate is a Cargo workspace with two packages:

- **`autosurgeon-cat`**: the library crate with traits (`Hydrate`,
  `Reconcile`, `HydrateRoot`, `ReconcileRoot`), primitive
  implementations, helper functions, and re-exports.
- **`autosurgeon-cat-derive`**: the proc-macro crate that implements
  `#[derive(Hydrate)]` and `#[derive(Reconcile)]`.

Both depend on `comp-cat-rs` (category-theory foundation) and
`automerge-cat` (CRDT document model).

## Building

```sh
cargo build
```

## Testing

```sh
cargo test
```

## Linting

```sh
RUSTFLAGS="-D warnings" cargo clippy --all-targets
```

## License

MIT
