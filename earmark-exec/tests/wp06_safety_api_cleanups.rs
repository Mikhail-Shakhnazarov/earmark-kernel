use earmark_core::{Kind, Provenance};
use earmark_store::{StoredObject, StoredPayload};
use earmark_exec::provider::{reset_provider_circuit_registry_for_tests, provider_circuit_registry};

#[test]
fn test_stored_object_builder_success() {
    let payload = StoredPayload::from_markdown("# Test");
    let provenance = Provenance::direct_input("test_actor");
    
    let obj = StoredObject::builder(Kind::Object, payload.clone())
        .class("test_class")
        .provenance(provenance.clone())
        .header("title", "Test Title")
        .build()
        .unwrap();
        
    assert_eq!(obj.envelope.kind, Kind::Object);
    assert_eq!(obj.envelope.class, Some("test_class".to_string()));
    assert_eq!(obj.envelope.provenance.actor, "test_actor");
    assert_eq!(obj.envelope.title(), Some("Test Title".to_string()));
    assert_eq!(obj.payload, payload);
}

#[test]
fn test_stored_object_builder_missing_provenance() {
    let payload = StoredPayload::from_markdown("# Test");
    
    let res = StoredObject::builder(Kind::Object, payload)
        .class("test_class")
        .build();
        
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), "provenance is required");
}

#[test]
fn test_provider_circuit_reset() {
    reset_provider_circuit_registry_for_tests();
    
    {
        let mut lock = provider_circuit_registry().lock().unwrap();
        lock.insert("test_key".to_string(), Default::default());
        assert!(!lock.is_empty());
    }
    
    reset_provider_circuit_registry_for_tests();
    
    {
        let lock = provider_circuit_registry().lock().unwrap();
        assert!(lock.is_empty());
    }
}
