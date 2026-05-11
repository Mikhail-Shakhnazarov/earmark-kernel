use earmark_exec::provider::compiled_provider_capabilities;

#[cfg(feature = "http-provider")]
use earmark_exec::provider::default_provider_registry;

use earmark_exec::provider::ProviderCapabilityStatus;

#[test]
fn test_default_registry_includes_http_generation_with_feature() {
    #[cfg(feature = "http-provider")]
    {
        let registry = default_provider_registry();
        assert!(registry.get("http_generation").is_some());
    }
}

#[test]
fn test_compiled_capabilities_report_http_generation_status() {
    let capabilities = compiled_provider_capabilities();
    let http = capabilities
        .iter()
        .find(|c| c.provider == "http_generation")
        .expect("http_generation capability should be reported");

    #[cfg(feature = "http-provider")]
    assert_eq!(http.status, ProviderCapabilityStatus::Available);

    #[cfg(not(feature = "http-provider"))]
    assert_eq!(http.status, ProviderCapabilityStatus::CompileDisabled);
}
