use earmark_core::{
    ClassDefinition, ClassStandingRules, HeaderValue, JsonSchemaRef, Kind, RuntimeProvenance,
};
use earmark_exec::ProviderRegistry;
use earmark_index::DerivedIndex;
use earmark_runtime_tools::{DepositValidationContext, RuntimeToolSurface};
use earmark_store::{
    CanonicalStore, GitCanonicalStore, ObjectStore, StoreScanner, StoreWriteLocking, StoredObject,
    StoredPayload, WorkspaceLayout,
};
use serde_json::json;
use std::collections::BTreeMap;
use tempfile::tempdir;

#[test]
fn test_schema_enforcement() {
    let tmp = tempdir().unwrap();
    let store = GitCanonicalStore::new(tmp.path());
    store.init_layout().unwrap();
    let index = DerivedIndex::open(tmp.path()).unwrap();
    let providers = ProviderRegistry::default();

    let surface = RuntimeToolSurface {
        store: &store,
        index: &index,
        provider_service: &providers,
    };

    // 1. Register a class with a schema
    let schema = json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "age": { "type": "integer" }
        },
        "required": ["name"]
    });

    let class_def = ClassDefinition {
        name: "person".to_string(),
        version: "1.0.0".to_string(),
        kind: "object".to_string(),
        required_headers: vec![],
        payload_schema: JsonSchemaRef(serde_json::to_string(&schema).unwrap()),
        standing_rules: ClassStandingRules::default(),
        relation_rules: vec![],
        validators: vec![],
    };

    let class_obj = StoredObject::new(
        Kind::Object,
        Some("class_definition".to_string()),
        Default::default(),
        earmark_core::Provenance::direct_input("tester"),
        BTreeMap::from([(
            "title".to_string(),
            HeaderValue::String(class_def.name.clone()),
        )]),
        StoredPayload::from_yaml(earmark_core::to_yaml(&class_def).unwrap()),
        vec![],
    );

    store.write_object(&class_obj).unwrap();
    index.rebuild_from_store(&store).unwrap();

    let prov = RuntimeProvenance {
        actor: "tester".to_string(),
        source_type: "test".to_string(),
    };

    // 2. Deposit valid object
    // Note: the class name is "person", and the index should find the class_definition object with ID "person"
    let res = surface.deposit_object(
        "person".to_string(),
        None,
        None,
        json!({"name": "Alice", "age": 30}),
        prov.clone(),
        DepositValidationContext::default(),
    );
    assert!(res.is_ok());

    // 3. Deposit invalid object (missing required name)
    let res = surface.deposit_object(
        "person".to_string(),
        None,
        None,
        json!({"age": 30}),
        prov.clone(),
        DepositValidationContext::default(),
    );
    assert!(res.is_err());
    println!("Schema violation error: {:?}", res.err().unwrap());

    // 4. Deposit invalid object (wrong type)
    let res = surface.deposit_object(
        "person".to_string(),
        None,
        None,
        json!({"name": "Alice", "age": "thirty"}),
        prov.clone(),
        DepositValidationContext::default(),
    );
    assert!(res.is_err());
    println!("Schema violation error (type): {:?}", res.err().unwrap());
}
