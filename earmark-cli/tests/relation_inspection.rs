use assert_cmd::Command;
use serde_json::Value;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn relation_inspection_and_explanation() {
    let dir = tempdir().unwrap();
    let workspace = workspace_root();

    // 1. Init
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("init")
        .assert()
        .success();

    // 2. Register classes
    let class_path = workspace.join("docs/declarations/examples/classes/finding.yaml");
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("declare")
        .arg("register")
        .arg("--kind")
        .arg("class")
        .arg(&class_path)
        .assert()
        .success();

    // 3. Deposit objects
    let dep1 = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("deposit")
        .arg("--class")
        .arg("finding")
        .arg("--title")
        .arg("Finding 1")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let id1 = serde_json::from_slice::<Value>(&dep1).unwrap()["data"]["object_id"]
        .as_str()
        .unwrap()
        .to_string();

    let dep2 = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("deposit")
        .arg("--class")
        .arg("source_note")
        .arg("--title")
        .arg("Source 1")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let id2 = serde_json::from_slice::<Value>(&dep2).unwrap()["data"]["object_id"]
        .as_str()
        .unwrap()
        .to_string();

    // 4. Create a privileged relation using the real canonical path
    let rel_id = {
        use earmark_core::*;
        use earmark_index::*;
        use earmark_store::*;

        let store = GitCanonicalStore::new(dir.path());
        let index = DerivedIndex::open(dir.path()).unwrap();

        let payload = RelationPayload {
            source: ObjectRef::new(
                ObjectId::parse(&id1).unwrap(),
                VersionId::new(),
                Kind::Object,
                Some("finding".to_string()),
            ),
            target: ObjectRef::new(
                ObjectId::parse(&id2).unwrap(),
                VersionId::new(),
                Kind::Object,
                Some("source_note".to_string()),
            ),
            relation_type: REL_TYPE_USED_INSTRUCTION.to_string(),
            qualifiers: std::collections::BTreeMap::new(),
            scope: None,
        };

        let rel_ref = earmark_exec::persist_relation_canonical(
            &store,
            &index,
            payload,
            Provenance::direct_input("test"),
            RelationCreationMode::PrivilegedSystem,
            None,
        )
        .unwrap();

        rel_ref.id.to_string()
    };

    // 5. Test em relation show
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("relation")
        .arg("show")
        .arg(&rel_id)
        .assert()
        .success();

    // 6. Test em relation explain surfaces creation mode
    let explain_output = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("relation")
        .arg("explain")
        .arg(&rel_id)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let explain_text = String::from_utf8(explain_output).unwrap();
    assert!(explain_text.contains("RELATION Explanation"));
    assert!(explain_text.contains("used_instruction"));
    assert!(explain_text.contains("Creation Mode: privileged_system"));

    // 7. Test em relation list
    Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("relation")
        .arg("list")
        .arg("--source-id")
        .arg(&id1)
        .assert()
        .success();

    // 8. Create a relation with explicit auth headers and verify explain surfaces them
    let rel_id2 = {
        use earmark_core::*;
        use earmark_index::*;
        use earmark_store::*;
        use std::collections::BTreeMap;

        let store = GitCanonicalStore::new(dir.path());
        let index = DerivedIndex::open(dir.path()).unwrap();

        let source_id = ObjectId::parse(&id1).unwrap();
        let target_id = ObjectId::parse(&id2).unwrap();
        let payload = RelationPayload {
            source: ObjectRef::new(
                source_id,
                VersionId::new(),
                Kind::Object,
                Some("finding".to_string()),
            ),
            target: ObjectRef::new(
                target_id,
                VersionId::new(),
                Kind::Object,
                Some("source_note".to_string()),
            ),
            relation_type: "mentions".to_string(),
            qualifiers: BTreeMap::new(),
            scope: None,
        };
        let mut headers = BTreeMap::new();
        headers.insert(
            "relation_auth_endpoint".to_string(),
            HeaderValue::String("source".to_string()),
        );
        headers.insert(
            "relation_auth_class".to_string(),
            HeaderValue::String("finding".to_string()),
        );
        headers.insert(
            "relation_auth_authority".to_string(),
            HeaderValue::String("source".to_string()),
        );
        headers.insert(
            "relation_auth_direction".to_string(),
            HeaderValue::String("outgoing".to_string()),
        );
        let rel_ref2 = earmark_exec::persist_relation_canonical(
            &store,
            &index,
            payload,
            Provenance::direct_input("test"),
            RelationCreationMode::Declared,
            Some(headers),
        )
        .unwrap();
        rel_ref2.id.to_string()
    };

    let explain_json = Command::cargo_bin("earmark-cli")
        .unwrap()
        .arg("--root")
        .arg(dir.path())
        .arg("--json")
        .arg("relation")
        .arg("explain")
        .arg(&rel_id2)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let explain_val: serde_json::Value = serde_json::from_slice(&explain_json).unwrap();
    let auth = &explain_val["data"]["related"]["authorization"];
    assert!(!auth.is_null(), "authorization should not be null");
    assert_eq!(auth["endpoint"], "source");
    assert_eq!(auth["class"], "finding");
    assert_eq!(auth["authority"], "source");
    assert_eq!(auth["direction"], "outgoing");
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}
