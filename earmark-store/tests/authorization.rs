use std::collections::BTreeMap;

use earmark_core::{Kind, Provenance, Standing};
use earmark_store::{
    authorization, GitCanonicalStore, ObjectStore, StoreError, StoredObject, StoredPayload,
};
use tempfile::tempdir;

fn make_object(kind: Kind, actor: &str) -> StoredObject {
    StoredObject::new(
        kind,
        Some("test".to_string()),
        Standing::default(),
        Provenance::direct_input(actor),
        BTreeMap::new(),
        StoredPayload::from_yaml("name: test\nversion: 1.0"),
        vec![],
    )
}

#[test]
fn non_sensitive_kinds_bypass_auth_check() {
    let dir = tempdir().unwrap();
    let store =
        GitCanonicalStore::with_authorized_actors(dir.path(), vec!["trusted-admin".to_string()]);

    let object = make_object(Kind::Instruction, "untrusted-user");
    let result = store.write_object(&object);
    assert!(result.is_ok(), "non-sensitive kinds should bypass auth");
}

#[test]
fn default_store_allows_sensitive_kinds() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());

    let object = make_object(Kind::SystemDefinition, "anyone");
    let result = store.write_object(&object);
    assert!(
        result.is_ok(),
        "default store (no policy) should allow sensitive kinds"
    );
}

#[test]
fn authorized_actor_can_write_sensitive_kind() {
    let dir = tempdir().unwrap();
    let store =
        GitCanonicalStore::with_authorized_actors(dir.path(), vec!["trusted-admin".to_string()]);

    let object = make_object(Kind::ProviderProfile, "trusted-admin");
    let result = store.write_object(&object);
    assert!(result.is_ok(), "trusted admin should be authorized");
}

#[test]
fn unauthorized_actor_rejected_for_system_kind() {
    let dir = tempdir().unwrap();
    let store =
        GitCanonicalStore::with_authorized_actors(dir.path(), vec!["trusted-admin".to_string()]);

    let object = make_object(Kind::SystemDefinition, "untrusted-user");
    let result = store.write_object(&object);
    assert!(
        matches!(result, Err(StoreError::Unauthorized(_))),
        "untrusted user should be rejected for SystemDefinition: got {:?}",
        result
    );
}

#[test]
fn unauthorized_actor_rejected_for_policy_kind() {
    let dir = tempdir().unwrap();
    let store =
        GitCanonicalStore::with_authorized_actors(dir.path(), vec!["trusted-admin".to_string()]);

    let object = make_object(Kind::Policy, "untrusted-user");
    let result = store.write_object(&object);
    assert!(
        matches!(result, Err(StoreError::Unauthorized(_))),
        "untrusted user should be rejected for Policy: got {:?}",
        result
    );
}

#[test]
fn unauthorized_actor_rejected_for_provider_profile_kind() {
    let dir = tempdir().unwrap();
    let store =
        GitCanonicalStore::with_authorized_actors(dir.path(), vec!["trusted-admin".to_string()]);

    let object = make_object(Kind::ProviderProfile, "untrusted-user");
    let result = store.write_object(&object);
    assert!(
        matches!(result, Err(StoreError::Unauthorized(_))),
        "untrusted user should be rejected for ProviderProfile: got {:?}",
        result
    );
}

#[test]
fn check_write_authorized_non_sensitive() {
    let object = make_object(Kind::Object, "untrusted-user");
    let trusted = vec!["admin".to_string()];
    assert!(authorization::check_write_authorized(&object, &trusted).is_ok());
}

#[test]
fn check_write_authorized_sensitive_no_trusted() {
    let object = make_object(Kind::SystemDefinition, "anyone");
    assert!(authorization::check_write_authorized(&object, &[]).is_ok());
}

#[test]
fn check_write_authorized_sensitive_match() {
    let object = make_object(Kind::Policy, "admin");
    let trusted = vec!["admin".to_string()];
    assert!(authorization::check_write_authorized(&object, &trusted).is_ok());
}

#[test]
fn check_write_authorized_sensitive_no_match() {
    let object = make_object(Kind::ProviderProfile, "hacker");
    let trusted = vec!["admin".to_string()];
    assert!(authorization::check_write_authorized(&object, &trusted).is_err());
}

#[test]
fn is_sensitive_kind_checks() {
    assert!(authorization::is_sensitive_kind(&Kind::SystemDefinition));
    assert!(authorization::is_sensitive_kind(&Kind::Policy));
    assert!(authorization::is_sensitive_kind(&Kind::ProviderProfile));
    assert!(!authorization::is_sensitive_kind(&Kind::Object));
    assert!(!authorization::is_sensitive_kind(&Kind::Instruction));
    assert!(!authorization::is_sensitive_kind(&Kind::Workflow));
}
