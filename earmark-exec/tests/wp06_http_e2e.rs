#![cfg(feature = "http-provider")]

use earmark_connected_context::{WorkSurfaceManifest, WorkSurfaceObject};
use earmark_core::{
    HttpAuthConfig, HttpAuthKind, HttpGenerationProfile, HttpRequestTemplate,
    HttpResponseExtraction, Kind, ObjectId, ObjectRef, ProviderProfile, VersionId, VersionRef,
};
use earmark_exec::{HttpGenerationAdapter, ProviderRegistry, ProviderService};
use earmark_index::DerivedIndex;
use earmark_store::{CanonicalStore, GitCanonicalStore, StoredObject, StoredPayload};
use httpmock::MockServer;
use std::collections::BTreeMap;
use std::sync::Arc;
use tempfile::tempdir;

#[test]
#[cfg(feature = "http-provider")]
fn test_http_provider_e2e_content_rendering() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    let _index = DerivedIndex::open(dir.path()).unwrap();

    // 1. Create an input object with real payload
    let input_payload = "This is the source evidence text.";
    let stored_input = StoredObject::new(
        Kind::Object,
        Some("evidence".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("user"),
        BTreeMap::from([(
            "title".to_string(),
            earmark_core::HeaderValue::String("Source Doc".to_string()),
        )]),
        StoredPayload::from_json_bytes(input_payload.as_bytes().to_vec()),
        vec![],
    );
    let input_version = store.write_object(&stored_input).unwrap();
    let input_ref = ObjectRef::new(
        stored_input.envelope.id.clone(),
        input_version.version_id,
        Kind::Object,
        Some("evidence".to_string()),
    );

    // 2. Mock Server
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(httpmock::Method::POST)
            .path("/v1/chat")
            // Assert that we receive the actual content in the body
            .body_contains("This is the source evidence text.")
            .body_contains("Summarize this doc");
        then.status(200).json_body(serde_json::json!({
            "choices": [{ "message": { "content": "Summary: doc is about X" } }],
            "usage": { "total_tokens": 50 }
        }));
    });

    // 3. Provider Profile
    let profile = ProviderProfile {
        name: "e2e_test".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        provider: "http_generation".to_string(),
        model: "gpt-mock".to_string(),
        endpoint_env: None,
        auth_env: None,
        budget: earmark_core::ProviderBudget::default(),
        allowed_operations: vec!["transform".to_string()],
        exposure: earmark_core::ProviderExposure {
            allow_prose_objects: true,
            allow_structured_declarations: false,
            allow_work_surface_only: false,
            allow_export_requests: false,
        },
        response_contract: earmark_core::ProviderResponseContract {
            format: "markdown".to_string(),
            must_return_candidate_only: true,
            must_include_lineage: false,
        },
        http: Some(HttpGenerationProfile {
            method: Some("POST".to_string()),
            url_template: format!("{}/v1/chat", server.base_url()),
            auth: HttpAuthConfig {
                kind: HttpAuthKind::None,
                ..Default::default()
            },
            request: HttpRequestTemplate {
                content_type: Some("application/json".to_string()),
                body: serde_json::json!({
                    "model": "{{model}}",
                    "messages": [{ "role": "user", "content": "{{input_text}}" }]
                }),
            },
            response: HttpResponseExtraction {
                text_path: "$.choices[0].message.content".to_string(),
                input_tokens_path: Some("$.usage.total_tokens".to_string()),
                ..Default::default()
            },
        }),
    };

    // 4. Setup Provider Service
    let mut registry = ProviderRegistry::new();
    registry.register(Arc::new(HttpGenerationAdapter));

    // 5. Instruction
    let instruction = earmark_core::InstructionPayload {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        purpose: "testing".to_string(),
        input_classes: vec![],
        output_classes: vec![],
        body: earmark_core::MarkdownBody::new("Summarize this doc".to_string()),
        execution_policy: "delegated".to_string(),
        provider_profile: None,
        trace_policy: "none".to_string(),
        register: "none".to_string(),
    };

    // 6. Build ProviderRequest (Simulating what transition.rs does)
    let rendered_input = earmark_exec::helpers::render_provider_input(
        &store,
        &instruction,
        None,
        std::slice::from_ref(&input_ref),
        &profile,
    )
    .unwrap();

    let request = earmark_core::ProviderRequest {
        request_id: "req_e2e".to_string(),
        run_id: "run_e2e".to_string(),
        work_packet: ObjectRef::new(
            earmark_core::ObjectId::new(),
            earmark_core::VersionId::new(),
            Kind::WorkPacket,
            None,
        ),
        provider_profile: VersionRef::new(
            earmark_core::ObjectId::new(),
            earmark_core::VersionId::new(),
        ),
        instruction_text: instruction.body.as_str().to_string(),
        context_text: None,
        input_text: rendered_input,
        work_surface_manifest: None,
        inputs: vec![input_ref],
        response_contract: profile.response_contract.clone(),
        issued_at: chrono::Utc::now(),
    };

    // 7. Execute via service (ProviderRegistry implements ProviderService)
    let outcome = registry.provide(&profile, request, "transform").unwrap();

    // 8. Assertions
    mock.assert();
    let response = outcome.response.unwrap();
    assert_eq!(response.candidate_payload, "Summary: doc is about X");
    assert_eq!(response.usage.unwrap().input_tokens, Some(50));
}

#[test]
#[cfg(feature = "http-provider")]
fn test_http_provider_rendering_with_manifest() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());

    // 1. Objects
    let stored_input = StoredObject::new(
        Kind::Object,
        None,
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("user"),
        BTreeMap::new(),
        StoredPayload::from_markdown("Active input content"),
        vec![],
    );
    let input_v = store.write_object(&stored_input).unwrap();
    let input_ref = ObjectRef::new(
        stored_input.envelope.id.clone(),
        input_v.version_id,
        Kind::Object,
        None,
    );

    let stored_manifest_obj = StoredObject::new(
        Kind::Object,
        None,
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("user"),
        BTreeMap::from([(
            "title".to_string(),
            earmark_core::HeaderValue::String("Manifest Doc".to_string()),
        )]),
        StoredPayload::from_markdown("Manifest-only content"),
        vec![],
    );
    let manifest_v = store.write_object(&stored_manifest_obj).unwrap();
    let manifest_ref = ObjectRef::new(
        stored_manifest_obj.envelope.id.clone(),
        manifest_v.version_id,
        Kind::Object,
        None,
    );

    // 2. Manifest
    let manifest = WorkSurfaceManifest {
        surface_id: "surf1".to_string(),
        compiled_context: VersionRef::new(ObjectId::new(), VersionId::new()),
        work_packet: None,
        generated_at: chrono::Utc::now(),
        objects: vec![WorkSurfaceObject {
            object: manifest_ref.clone(),
            title: Some("Manifest Doc".to_string()),
            path: "doc.md".to_string(),
            excerpt_range: None,
            lineage: vec![],
        }],
        boundary_relations: vec![],
        constraints: BTreeMap::new(),
        warnings: vec![],
    };

    // 3. Profile
    let profile = ProviderProfile {
        name: "test".to_string(),
        version: "1".to_string(),
        description: None,
        provider: "http_generation".to_string(),
        model: "m".to_string(),
        endpoint_env: None,
        auth_env: None,
        budget: earmark_core::ProviderBudget::default(),
        allowed_operations: vec!["transform".to_string()],
        exposure: earmark_core::ProviderExposure {
            allow_prose_objects: true,
            allow_structured_declarations: false,
            allow_work_surface_only: false,
            allow_export_requests: false,
        },
        response_contract: earmark_core::ProviderResponseContract {
            format: "markdown".to_string(),
            must_return_candidate_only: true,
            must_include_lineage: false,
        },
        http: None,
    };

    let instruction = earmark_core::InstructionPayload {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        purpose: "testing".to_string(),
        input_classes: vec![],
        output_classes: vec![],
        body: earmark_core::MarkdownBody::new("hi".to_string()),
        execution_policy: "delegated".to_string(),
        provider_profile: None,
        trace_policy: "none".to_string(),
        register: "none".to_string(),
    };

    // 4. Render
    let rendered = earmark_exec::helpers::render_provider_input(
        &store,
        &instruction,
        Some(&manifest),
        std::slice::from_ref(&input_ref),
        &profile,
    )
    .unwrap();

    // Must contain both:
    // - manifest-only object
    // - active input (because allow_work_surface_only is false)
    assert!(rendered.contains("Active input content"));
    assert!(rendered.contains("Manifest-only content"));
    assert!(rendered.contains("Manifest Doc"));
    assert!(rendered.contains("[Active Input]"));
}

#[test]
#[cfg(feature = "http-provider")]
fn test_http_provider_exposure_structured_hiding() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());

    // Structured object (Workflow)
    let stored_workflow = StoredObject::new(
        Kind::Workflow,
        None,
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("user"),
        BTreeMap::new(),
        StoredPayload::from_markdown("SECRET_WORKFLOW_STEPS"),
        vec![],
    );
    let v = store.write_object(&stored_workflow).unwrap();
    let workflow_ref = ObjectRef::new(
        stored_workflow.envelope.id.clone(),
        v.version_id,
        Kind::Workflow,
        None,
    );

    // Profile with allow_structured_declarations = false
    let profile = ProviderProfile {
        name: "test".to_string(),
        version: "1".to_string(),
        description: None,
        provider: "http_generation".to_string(),
        model: "m".to_string(),
        endpoint_env: None,
        auth_env: None,
        budget: earmark_core::ProviderBudget::default(),
        allowed_operations: vec!["transform".to_string()],
        exposure: earmark_core::ProviderExposure {
            allow_prose_objects: true,
            allow_structured_declarations: false,
            allow_work_surface_only: false,
            allow_export_requests: false,
        },
        response_contract: earmark_core::ProviderResponseContract {
            format: "markdown".to_string(),
            must_return_candidate_only: true,
            must_include_lineage: false,
        },
        http: None,
    };

    let instruction = earmark_core::InstructionPayload {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        purpose: "testing".to_string(),
        input_classes: vec![],
        output_classes: vec![],
        body: earmark_core::MarkdownBody::new("hi".to_string()),
        execution_policy: "delegated".to_string(),
        provider_profile: None,
        trace_policy: "none".to_string(),
        register: "none".to_string(),
    };

    let rendered = earmark_exec::helpers::render_provider_input(
        &store,
        &instruction,
        None,
        &[workflow_ref],
        &profile,
    )
    .unwrap();

    assert!(!rendered.contains("SECRET_WORKFLOW_STEPS"));
    assert!(rendered.contains("Structured declarations hidden by exposure policy"));
    assert!(rendered.contains("Kind: workflow"));
}

#[test]
#[cfg(feature = "http-provider")]
fn test_http_provider_exposure_prose_hiding() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());

    let stored_input = StoredObject::new(
        Kind::Object,
        None,
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("user"),
        BTreeMap::from([(
            "title".to_string(),
            earmark_core::HeaderValue::String("Private Doc".to_string()),
        )]),
        StoredPayload::from_markdown("PRIVATE_PROSE_CONTENT"),
        vec![],
    );
    let v = store.write_object(&stored_input).unwrap();
    let input_ref = ObjectRef::new(
        stored_input.envelope.id.clone(),
        v.version_id,
        Kind::Object,
        None,
    );

    // Profile with allow_prose_objects = false
    let profile = ProviderProfile {
        name: "test".to_string(),
        version: "1".to_string(),
        description: None,
        provider: "http_generation".to_string(),
        model: "m".to_string(),
        endpoint_env: None,
        auth_env: None,
        budget: earmark_core::ProviderBudget::default(),
        allowed_operations: vec!["transform".to_string()],
        exposure: earmark_core::ProviderExposure {
            allow_prose_objects: false,
            allow_structured_declarations: true,
            allow_work_surface_only: false,
            allow_export_requests: false,
        },
        response_contract: earmark_core::ProviderResponseContract {
            format: "markdown".to_string(),
            must_return_candidate_only: true,
            must_include_lineage: false,
        },
        http: None,
    };

    let instruction = earmark_core::InstructionPayload {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        purpose: "testing".to_string(),
        input_classes: vec![],
        output_classes: vec![],
        body: earmark_core::MarkdownBody::new("hi".to_string()),
        execution_policy: "delegated".to_string(),
        provider_profile: None,
        trace_policy: "none".to_string(),
        register: "none".to_string(),
    };

    let rendered = earmark_exec::helpers::render_provider_input(
        &store,
        &instruction,
        None,
        &[input_ref],
        &profile,
    )
    .unwrap();

    assert!(!rendered.contains("PRIVATE_PROSE_CONTENT"));
    assert!(rendered.contains("Payload content hidden by exposure policy"));
    assert!(rendered.contains("Private Doc")); // Metadata remains
}

#[test]
#[cfg(feature = "http-provider")]
fn test_http_provider_exposure_class_definition_hiding() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());

    // Class definition object (Kind::Object, class="class_definition")
    let stored_class = StoredObject::new(
        Kind::Object,
        Some("class_definition".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("user"),
        BTreeMap::from([(
            "title".to_string(),
            earmark_core::HeaderValue::String("My Class".to_string()),
        )]),
        StoredPayload::from_markdown("SECRET_CLASS_SCHEMA"),
        vec![],
    );
    let v = store.write_object(&stored_class).unwrap();
    let class_ref = ObjectRef::new(
        stored_class.envelope.id.clone(),
        v.version_id,
        Kind::Object,
        Some("class_definition".to_string()),
    );

    // Profile with allow_prose_objects = true, allow_structured_declarations = false
    let profile = ProviderProfile {
        name: "test".to_string(),
        version: "1".to_string(),
        description: None,
        provider: "http_generation".to_string(),
        model: "m".to_string(),
        endpoint_env: None,
        auth_env: None,
        budget: earmark_core::ProviderBudget::default(),
        allowed_operations: vec!["transform".to_string()],
        exposure: earmark_core::ProviderExposure {
            allow_prose_objects: true,
            allow_structured_declarations: false,
            allow_work_surface_only: false,
            allow_export_requests: false,
        },
        response_contract: earmark_core::ProviderResponseContract {
            format: "markdown".to_string(),
            must_return_candidate_only: true,
            must_include_lineage: false,
        },
        http: None,
    };

    let instruction = earmark_core::InstructionPayload {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        purpose: "testing".to_string(),
        input_classes: vec![],
        output_classes: vec![],
        body: earmark_core::MarkdownBody::new("hi".to_string()),
        execution_policy: "delegated".to_string(),
        provider_profile: None,
        trace_policy: "none".to_string(),
        register: "none".to_string(),
    };

    let rendered = earmark_exec::helpers::render_provider_input(
        &store,
        &instruction,
        None,
        &[class_ref],
        &profile,
    )
    .unwrap();

    assert!(!rendered.contains("SECRET_CLASS_SCHEMA"));
    assert!(rendered.contains("Structured declarations hidden by exposure policy"));
    assert!(rendered.contains("My Class")); // Metadata remains
    assert!(rendered.contains("Class: class_definition"));
}
