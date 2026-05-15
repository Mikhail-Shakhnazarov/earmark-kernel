use std::collections::BTreeMap;
use std::path::{Path};
use earmark_core::{FlexibleVersionRef, VersionRef, WorkflowDeclaration, WorkflowDeclarationOperation, WorkflowOperationKind, ObjectId, VersionId};
use earmark_declarations::resolve_workflow_declaration;

#[test]
fn test_resolve_path_success() {
    let mut registry = BTreeMap::new();
    let id = ObjectId::parse("obj_1234567890abcdef1234567890abcdef").unwrap();
    let vid = VersionId::parse("ver_1234567890abcdef1234567890abcdef").unwrap();
    let vref = VersionRef::new(id, vid);
    
    let base_path = Path::new("/tmp/earmark/system");
    let instruction_path = base_path.join("instructions/hello.md");
    registry.insert(instruction_path.clone(), vref.clone());

    let decl = WorkflowDeclaration {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        operations: vec![WorkflowDeclarationOperation {
            id: "op1".to_string(),
            kind: WorkflowOperationKind::Transform,
            instruction: Some(FlexibleVersionRef::Path("instructions/hello.md".to_string())),
            compiled_context: None,
            policy: None,
            provider_profile: None,
            input_contracts: Vec::new(),
            output_contracts: Vec::new(),
        }],
        edges: Vec::new(),
        guards: Vec::new(),
        output_contracts: Vec::new(),
    };

    let workflow_path = base_path.join("main.yaml");
    let resolved = resolve_workflow_declaration(&workflow_path, decl, &registry).unwrap();
    
    assert_eq!(resolved.operations[0].instruction, Some(vref));
}

#[test]
fn test_resolve_path_failure_rich_error() {
    let registry = BTreeMap::new();
    let base_path = Path::new("/tmp/earmark/system");
    let workflow_path = base_path.join("main.yaml");

    let decl = WorkflowDeclaration {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        operations: vec![WorkflowDeclarationOperation {
            id: "extract_findings".to_string(),
            kind: WorkflowOperationKind::Transform,
            instruction: Some(FlexibleVersionRef::Path("missing.md".to_string())),
            compiled_context: None,
            policy: None,
            provider_profile: None,
            input_contracts: Vec::new(),
            output_contracts: Vec::new(),
        }],
        edges: Vec::new(),
        guards: Vec::new(),
        output_contracts: Vec::new(),
    };

    let result = resolve_workflow_declaration(&workflow_path, decl, &registry);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    
    assert!(err.contains("invalid workflow reference at operation 'extract_findings'.instruction"));
    assert!(err.contains("unresolved path reference 'missing.md'"));
    assert!(err.contains("main.yaml"));
}

#[test]
fn test_resolve_mixed_references() {
    let mut registry = BTreeMap::new();
    let id_path = ObjectId::parse("obj_11111111111111111111111111111111").unwrap();
    let vid_path = VersionId::parse("ver_11111111111111111111111111111111").unwrap();
    let vref_path = VersionRef::new(id_path, vid_path);

    let id_durable = ObjectId::parse("obj_22222222222222222222222222222222").unwrap();
    let vid_durable = VersionId::parse("ver_22222222222222222222222222222222").unwrap();
    let vref_durable = VersionRef::new(id_durable, vid_durable);
    
    let base_path = Path::new("/tmp/earmark/system");
    let instr_path = base_path.join("instr.md");
    registry.insert(instr_path.clone(), vref_path.clone());

    let decl = WorkflowDeclaration {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        operations: vec![WorkflowDeclarationOperation {
            id: "op1".to_string(),
            kind: WorkflowOperationKind::Transform,
            instruction: Some(FlexibleVersionRef::Path("instr.md".to_string())),
            compiled_context: Some(FlexibleVersionRef::Ref(vref_durable.clone())),
            policy: None,
            provider_profile: None,
            input_contracts: Vec::new(),
            output_contracts: Vec::new(),
        }],
        edges: Vec::new(),
        guards: Vec::new(),
        output_contracts: Vec::new(),
    };

    let workflow_path = base_path.join("main.yaml");
    let resolved = resolve_workflow_declaration(&workflow_path, decl, &registry).unwrap();
    
    assert_eq!(resolved.operations[0].instruction, Some(vref_path));
    assert_eq!(resolved.operations[0].compiled_context, Some(vref_durable));
}

#[test]
fn test_flexible_ref_deserialization_failure() {
    // Missing 'id' field
    let yaml = r#"
name: test
version: "1.0.0"
operations:
  - id: op1
    kind: transform
    instruction:
      version_id: ver_1234567890abcdef1234567890abcdef
"#;
    let err = earmark_core::parse_yaml::<WorkflowDeclaration>(yaml).unwrap_err().to_string();
    assert!(err.contains("missing 'id' field"));

    // Malformed 'id' field
    let yaml2 = r#"
name: test
version: "1.0.0"
operations:
  - id: op1
    kind: transform
    instruction:
      id: not_an_obj_id
      version_id: ver_1234567890abcdef1234567890abcdef
"#;
    let err2 = earmark_core::parse_yaml::<WorkflowDeclaration>(yaml2).unwrap_err().to_string();
    assert!(err2.contains("must start with obj_"));
}
