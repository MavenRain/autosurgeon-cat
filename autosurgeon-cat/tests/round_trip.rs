//! Integration tests: reconcile then hydrate (round-trip).

use automerge_cat::{ReplicaId, Session};
use autosurgeon_cat::{Hydrate, Reconcile};

// ---------------------------------------------------------------------------
// Test types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct Config {
    name: String,
    count: u64,
    enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct WithOptional {
    label: String,
    score: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct WithFloat {
    value: f64,
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct Nested {
    name: String,
    inner: Config,
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct WithList {
    tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct NewtypeWrapper(u64);

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
enum Status {
    Active,
    Suspended,
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
enum RichEnum {
    Off,
    Score(u64),
    Named { reason: String },
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_session() -> Session {
    Session::new(ReplicaId::new(1))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn round_trip_named_struct() -> Result<(), autosurgeon_cat::Error> {
    let session = make_session();
    let original = Config {
        name: "test".to_string(),
        count: 42,
        enabled: true,
    };
    let session = autosurgeon_cat::reconcile(&original, &session)?;
    let hydrated: Config = autosurgeon_cat::hydrate(session.document())?;
    assert_eq!(original, hydrated);
    Ok(())
}

#[test]
fn round_trip_with_some() -> Result<(), autosurgeon_cat::Error> {
    let session = make_session();
    let original = WithOptional {
        label: "hello".to_string(),
        score: Some(99),
    };
    let session = autosurgeon_cat::reconcile(&original, &session)?;
    let hydrated: WithOptional = autosurgeon_cat::hydrate(session.document())?;
    assert_eq!(original, hydrated);
    Ok(())
}

#[test]
fn round_trip_with_none() -> Result<(), autosurgeon_cat::Error> {
    let session = make_session();
    let original = WithOptional {
        label: "hello".to_string(),
        score: None,
    };
    let session = autosurgeon_cat::reconcile(&original, &session)?;
    let hydrated: WithOptional = autosurgeon_cat::hydrate(session.document())?;
    assert_eq!(original, hydrated);
    Ok(())
}

#[test]
fn round_trip_nested_struct() -> Result<(), autosurgeon_cat::Error> {
    let session = make_session();
    let original = Nested {
        name: "outer".to_string(),
        inner: Config {
            name: "inner".to_string(),
            count: 7,
            enabled: false,
        },
    };
    let session = autosurgeon_cat::reconcile(&original, &session)?;
    let hydrated: Nested = autosurgeon_cat::hydrate(session.document())?;
    assert_eq!(original, hydrated);
    Ok(())
}

#[test]
fn round_trip_vec() -> Result<(), autosurgeon_cat::Error> {
    let session = make_session();
    let original = WithList {
        tags: vec!["a".to_string(), "b".to_string(), "c".to_string()],
    };
    let session = autosurgeon_cat::reconcile(&original, &session)?;
    let hydrated: WithList = autosurgeon_cat::hydrate(session.document())?;
    assert_eq!(original, hydrated);
    Ok(())
}

#[test]
fn round_trip_unit_enum() -> Result<(), autosurgeon_cat::Error> {
    let session = make_session();
    let parent = Config {
        name: "demo".to_string(),
        count: 1,
        enabled: true,
    };
    let session = autosurgeon_cat::reconcile(&parent, &session)?;
    // Write a unit enum variant at a key
    let root = session.document().root();
    let session = Status::Active.reconcile(&session, root, "status")?;
    let hydrated = <Status as autosurgeon_cat::Hydrate>::hydrate(
        session.document(),
        autosurgeon_cat::resolve_key(session.document(), root, "status")?,
    )?;
    assert_eq!(Status::Active, hydrated);
    Ok(())
}

#[test]
fn round_trip_newtype_enum() -> Result<(), autosurgeon_cat::Error> {
    let session = make_session();
    let root = session.document().root();
    let original = RichEnum::Score(42);
    let session = original.reconcile(&session, root, "val")?;
    let hydrated = <RichEnum as autosurgeon_cat::Hydrate>::hydrate(
        session.document(),
        autosurgeon_cat::resolve_key(session.document(), root, "val")?,
    )?;
    assert_eq!(original, hydrated);
    Ok(())
}

#[test]
fn round_trip_named_enum() -> Result<(), autosurgeon_cat::Error> {
    let session = make_session();
    let root = session.document().root();
    let original = RichEnum::Named {
        reason: "policy".to_string(),
    };
    let session = original.reconcile(&session, root, "val")?;
    let hydrated = <RichEnum as autosurgeon_cat::Hydrate>::hydrate(
        session.document(),
        autosurgeon_cat::resolve_key(session.document(), root, "val")?,
    )?;
    assert_eq!(original, hydrated);
    Ok(())
}

#[test]
fn smart_reconcile_skips_unchanged_scalar() -> Result<(), autosurgeon_cat::Error> {
    let session = make_session();
    let original = Config {
        name: "test".to_string(),
        count: 42,
        enabled: true,
    };
    let session = autosurgeon_cat::reconcile(&original, &session)?;
    let clock_after_first = session.clock_value();

    // Reconcile the same value again
    let session = autosurgeon_cat::reconcile(&original, &session)?;
    let clock_after_second = session.clock_value();

    // Clock should not advance (no new operations)
    assert_eq!(clock_after_first, clock_after_second);
    Ok(())
}

#[test]
fn smart_reconcile_updates_changed_field() -> Result<(), autosurgeon_cat::Error> {
    let session = make_session();
    let v1 = Config {
        name: "test".to_string(),
        count: 1,
        enabled: true,
    };
    let session = autosurgeon_cat::reconcile(&v1, &session)?;

    let v2 = Config {
        name: "test".to_string(),
        count: 2,
        enabled: true,
    };
    let session = autosurgeon_cat::reconcile(&v2, &session)?;

    let hydrated: Config = autosurgeon_cat::hydrate(session.document())?;
    assert_eq!(v2, hydrated);
    Ok(())
}

#[test]
fn round_trip_newtype() -> Result<(), autosurgeon_cat::Error> {
    let session = make_session();
    let root = session.document().root();
    let original = NewtypeWrapper(123);
    let session = original.reconcile(&session, root, "wrapped")?;
    let hydrated = <NewtypeWrapper as autosurgeon_cat::Hydrate>::hydrate(
        session.document(),
        autosurgeon_cat::resolve_key(session.document(), root, "wrapped")?,
    )?;
    assert_eq!(original, hydrated);
    Ok(())
}

#[test]
fn round_trip_float() -> Result<(), autosurgeon_cat::Error> {
    let session = make_session();
    let original = WithFloat { value: 1.234_567 };
    let session = autosurgeon_cat::reconcile(&original, &session)?;
    let hydrated: WithFloat = autosurgeon_cat::hydrate(session.document())?;
    assert!((original.value - hydrated.value).abs() < f64::EPSILON);
    Ok(())
}
