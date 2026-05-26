use earmark_core::RuntimeProvenance;
use earmark_exec::ProviderRegistry;
use earmark_index::DerivedIndex;
use earmark_runtime_tools::{DepositValidationContext, RuntimeToolSurface};
use earmark_store::{GitCanonicalStore, WorkspaceLayout};
use serde_json::json;
use tempfile::tempdir;

#[test]
fn test_boundary_validation() {
    let tmp = tempdir().unwrap();
    let store = GitCanonicalStore::new(tmp.path());
    store.init_layout().unwrap();
    let mut index = DerivedIndex::open(tmp.path()).unwrap();
    let providers = ProviderRegistry::default();

    let mut surface = RuntimeToolSurface {
        store: &store,
        index: &mut index,
        provider_service: &providers,
    };

    let prov = RuntimeProvenance {
        actor: "tester".to_string(),
        source_type: "test".to_string(),
    };

    // 1. Invalid class name
    let res = surface.deposit_object(
        "InvalidClass".to_string(),
        None,
        None,
        json!("body"),
        prov.clone(),
        DepositValidationContext::default(),
    );
    assert!(res.is_err());
    println!("Invalid class error: {:?}", res.err().unwrap());

    // 2. Title too long
    let long_title = "a".repeat(1000);
    let res = surface.deposit_object(
        "valid_class".to_string(),
        None,
        Some(long_title),
        json!("body"),
        prov.clone(),
        DepositValidationContext::default(),
    );
    assert!(res.is_err());
    println!("Long title error: {:?}", res.err().unwrap());

    // 3. Payload too large
    let huge_payload = "a".repeat(11 * 1024 * 1024); // 11 MiB
    let res = surface.deposit_object(
        "valid_class".to_string(),
        None,
        None,
        json!(huge_payload),
        prov.clone(),
        DepositValidationContext::default(),
    );
    assert!(res.is_err());
    println!("Huge payload error: {:?}", res.err().unwrap());

    // 4. Valid deposit
    let res = surface.deposit_object(
        "valid_class".to_string(),
        None,
        Some("Short Title".to_string()),
        json!("Short body"),
        prov.clone(),
        DepositValidationContext::default(),
    );
    assert!(res.is_ok());
}
