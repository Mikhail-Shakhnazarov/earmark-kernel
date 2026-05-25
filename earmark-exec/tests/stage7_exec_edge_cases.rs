use earmark_core::{
    ClassDefinition, ClassStandingRules, JsonSchemaRef, Kind, ObjectRef, Provenance,
    RelationPayload, Standing,
};
use earmark_index::DerivedIndex;
use earmark_store::{GitCanonicalStore, ObjectStore, StoredObject, StoredPayload, WorkspaceLayout};
use std::collections::BTreeMap;
use tempfile::tempdir;

fn setup_store_and_index() -> (tempfile::TempDir, GitCanonicalStore, DerivedIndex) {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();
    let mut index = DerivedIndex::open(dir.path()).unwrap();
    (dir, store, index)
}

fn create_object(
    store: &GitCanonicalStore,
    kind: Kind,
    class: Option<&str>,
    body: &str,
) -> ObjectRef {
    let stored = StoredObject::new(
        kind,
        class.map(|s| s.to_string()),
        Standing::default(),
        Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_markdown(body),
        vec![],
    );
    store.write_object(&stored).unwrap();
    stored.object_ref()
}

fn register_class(
    store: &GitCanonicalStore,
    index: &mut DerivedIndex,
    name: &str,
    relation_rules: Vec<earmark_core::RelationRule>,
) {
    let def = ClassDefinition {
        name: name.to_string(),
        version: "1".to_string(),
        kind: "object".to_string(),
        required_headers: vec![],
        payload_schema: JsonSchemaRef("inline:any".to_string()),
        standing_rules: ClassStandingRules::default(),
        relation_rules,
        validators: vec![],
    };
    let json = serde_json::to_string(&def).unwrap();
    let stored = StoredObject::new(
        Kind::Object,
        Some("class_definition".to_string()),
        Standing::default(),
        Provenance::direct_input("system"),
        BTreeMap::new(),
        StoredPayload::from_markdown(&json),
        vec![],
    );
    let obj_ref = store.write_object(&stored).unwrap();
    index
        .upsert_head_object_from_store(store, &obj_ref.id)
        .unwrap();
}

#[test]
fn test_unauthorized_relation_rejected_if_no_rule() {
    let (_dir, store, mut index) = setup_store_and_index();

    let source_ref = create_object(&store, Kind::Object, Some("note"), "source");
    let target_ref = create_object(&store, Kind::Object, Some("finding"), "target");

    register_class(&store, &mut index, "note", vec![]);
    register_class(&store, &mut index, "finding", vec![]);

    let payload = RelationPayload {
        source: source_ref.clone(),
        target: target_ref.clone(),
        relation_type: "unauthorized_rel".to_string(),
        qualifiers: BTreeMap::new(),
        scope: None,
    };

    let res = earmark_exec::persist_relation_canonical(
        &store,
        &mut index,
        payload,
        Provenance::direct_input("test"),
        earmark_core::RelationCreationMode::Declared,
        None,
    );

    assert!(res.is_err());
    let err = res.unwrap_err().to_string();
    assert!(err.contains("no matching rule found for relation"));
}

#[test]
fn test_counterparty_class_restriction_respected() {
    let (_dir, store, mut index) = setup_store_and_index();

    let source_ref = create_object(&store, Kind::Object, Some("note"), "source");
    let target_ref = create_object(&store, Kind::Object, Some("unsupported_class"), "target");

    register_class(
        &store,
        &mut index,
        "note",
        vec![earmark_core::RelationRule {
            relation_type: "references".to_string(),
            counterparty_classes: vec!["finding".to_string()],
            direction: Some("outgoing".to_string()),
            authorizing_endpoint: Some("source".to_string()),
        }],
    );
    register_class(&store, &mut index, "finding", vec![]);
    register_class(&store, &mut index, "unsupported_class", vec![]);

    let payload = RelationPayload {
        source: source_ref.clone(),
        target: target_ref.clone(),
        relation_type: "references".to_string(),
        qualifiers: BTreeMap::new(),
        scope: None,
    };

    let res = earmark_exec::persist_relation_canonical(
        &store,
        &mut index,
        payload,
        Provenance::direct_input("test"),
        earmark_core::RelationCreationMode::Declared,
        None,
    );

    assert!(res.is_err());
    let err = res.unwrap_err().to_string();
    assert!(err.contains("no matching rule found for relation"));
}

#[test]
fn test_direction_restrictions_respected() {
    let (_dir, store, mut index) = setup_store_and_index();

    let source_ref = create_object(&store, Kind::Object, Some("note"), "source");
    let target_ref = create_object(&store, Kind::Object, Some("finding"), "target");

    register_class(
        &store,
        &mut index,
        "note",
        vec![earmark_core::RelationRule {
            relation_type: "references".to_string(),
            counterparty_classes: vec!["finding".to_string()],
            direction: Some("incoming".to_string()),
            authorizing_endpoint: Some("source".to_string()),
        }],
    );
    register_class(&store, &mut index, "finding", vec![]);

    let payload = RelationPayload {
        source: source_ref.clone(),
        target: target_ref.clone(),
        relation_type: "references".to_string(),
        qualifiers: BTreeMap::new(),
        scope: None,
    };

    let res = earmark_exec::persist_relation_canonical(
        &store,
        &mut index,
        payload,
        Provenance::direct_input("test"),
        earmark_core::RelationCreationMode::Declared,
        None,
    );

    assert!(res.is_err());
    let err = res.unwrap_err().to_string();
    assert!(err.contains("no matching rule found for relation"));
}

#[test]
fn test_redaction_secrets_in_url() {
    use earmark_exec::redaction::redact_sensitive;

    let url_with_creds = "https://admin:supersecret123@api.github.com/repos";
    assert_eq!(
        redact_sensitive(url_with_creds),
        "https://[REDACTED_CREDENTIALS]@api.github.com/repos"
    );

    let url_with_query =
        "https://api.openai.com/v1/chat?api_key=sk-proj-xyz123&token=my_secret_token";
    let redacted = redact_sensitive(url_with_query);
    assert!(!redacted.contains("sk-proj-xyz123"));
    assert!(!redacted.contains("my_secret_token"));
    assert!(redacted.contains("api_key=[REDACTED_KEY]"));
    assert!(redacted.contains("token=[REDACTED_TOKEN]"));
}

#[test]
fn test_redaction_auth_headers() {
    use earmark_exec::redaction::redact_sensitive;

    let header = "Authorization: Bearer sk-secret-token-value-here";
    let redacted = redact_sensitive(header);
    assert!(!redacted.contains("sk-secret-token-value-here"));
    assert!(redacted.contains("Authorization: Bearer [REDACTED_TOKEN]"));

    let custom_header = "x-api-key: my_raw_api_key_value";
    let redacted_custom = redact_sensitive(custom_header);
    assert!(!redacted_custom.contains("my_raw_api_key_value"));
    assert!(redacted_custom.contains("x-api-key: [REDACTED_KEY]"));
}

#[test]
fn test_redaction_failure_messages_and_missing_env() {
    use earmark_exec::redaction::redact_sensitive;

    let msg = "Could not execute request: <unset:GEMINI_API_KEY>";
    assert_eq!(
        redact_sensitive(msg),
        "Could not execute request: <unset:[REDACTED_ENV]>"
    );

    let msg_empty = "Could not execute request: <empty:ANTHROPIC_API_KEY>";
    assert_eq!(
        redact_sensitive(msg_empty),
        "Could not execute request: <empty:[REDACTED_ENV]>"
    );

    std::env::set_var("STAGE7_TEST_API_KEY", "secret_val");
    let logged_msg = "failed resolving env variable 'STAGE7_TEST_API_KEY' for auth";
    let redacted_log = redact_sensitive(logged_msg);
    assert!(!redacted_log.contains("STAGE7_TEST_API_KEY"));
    assert!(redacted_log.contains("'[REDACTED_ENV]'"));
}
