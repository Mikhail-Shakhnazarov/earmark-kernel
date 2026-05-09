use earmark_core::*;
use proptest::prelude::*;

fn arb_instruction_payload() -> impl Strategy<Value = InstructionPayload> {
    (
        "[a-z][a-z0-9_-]{0,63}",
        "[0-9]+\\.[0-9]+\\.[0-9]+",
        "[A-Za-z0-9 _.,:;!?()/+-]{1,128}",
        prop::collection::vec("[a-z]{1,32}", 0..5),
        prop::collection::vec("[a-z]{1,32}", 0..5),
        "local|delegated",
        prop::option::of(arb_version_ref()),
        "full|summary|none",
        "system|user",
        prop::collection::vec("[A-Za-z0-9 _.,:;!?()/+-]{1,80}", 1..8)
    ).prop_map(|(name, version, purpose, input_classes, output_classes, execution_policy, provider_profile, trace_policy, register, body)| {
        InstructionPayload {
            name,
            version,
            purpose,
            input_classes,
            output_classes,
            execution_policy: execution_policy.to_string(),
            provider_profile,
            trace_policy: trace_policy.to_string(),
            register: register.to_string(),
            body: MarkdownBody::new(body.join("\n")),
        }
    })
}

fn arb_version_ref() -> impl Strategy<Value = VersionRef> {
    (
        "obj_[a-z0-9]{32}",
        "ver_[a-z0-9]{32}"
    ).prop_map(|(id, vid)| {
        VersionRef::new(
            ObjectId::parse(id).unwrap(),
            VersionId::parse(vid).unwrap()
        )
    })
}

proptest! {
    #[test]
    fn test_object_id_roundtrip(s in "obj_[a-z0-9]{32}") {
        let id = ObjectId::parse(&s).expect("should be valid");
        assert_eq!(id.as_str(), s);
        
        let serialized = serde_json::to_string(&id).expect("should serialize");
        let deserialized: ObjectId = serde_json::from_str(&serialized).expect("should deserialize");
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_version_id_roundtrip(s in "ver_[a-z0-9]{32}") {
        let id = VersionId::parse(&s).expect("should be valid");
        assert_eq!(id.as_str(), s);
        
        let serialized = serde_json::to_string(&id).expect("should serialize");
        let deserialized: VersionId = serde_json::from_str(&serialized).expect("should deserialize");
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_symbolic_name_roundtrip(s in "[a-z][a-z0-9_-]{0,63}") {
        let name = SymbolicName::parse(&s).expect("should be valid");
        assert_eq!(name.as_str(), s);
        
        let serialized = serde_json::to_string(&name).expect("should serialize");
        let deserialized: SymbolicName = serde_json::from_str(&serialized).expect("should deserialize");
        assert_eq!(name, deserialized);
    }

    #[test]
    fn test_instruction_payload_markdown_roundtrip(payload in arb_instruction_payload()) {
        let rendered = payload.to_markdown().expect("should render");
        let parsed = InstructionPayload::parse_markdown(&rendered).expect("should parse");
        
        assert_eq!(payload.name, parsed.name);
        assert_eq!(payload.version, parsed.version);
        assert_eq!(payload.purpose, parsed.purpose);
        assert_eq!(payload.input_classes, parsed.input_classes);
        assert_eq!(payload.output_classes, parsed.output_classes);
        assert_eq!(payload.execution_policy, parsed.execution_policy);
        assert_eq!(payload.provider_profile, parsed.provider_profile);
        assert_eq!(payload.trace_policy, parsed.trace_policy);
        assert_eq!(payload.register, parsed.register);
        assert_eq!(payload.body.as_str().trim(), parsed.body.as_str().trim());
    }

    #[test]
    fn test_yaml_stability(s in ".{1,1024}") {
        // Generic YAML stability for a simple map
        let mut map = std::collections::BTreeMap::new();
        map.insert("key".to_string(), s.clone());
        
        let yaml = to_yaml(&map).expect("should render yaml");
        let parsed: std::collections::BTreeMap<String, String> = parse_yaml(&yaml).expect("should parse yaml");
        assert_eq!(map, parsed);
    }

    #[test]
    fn test_invalid_object_id_rejected(s in "[^o][^b][^j][^_].{0,124}") {
        if !s.starts_with("obj_") {
            assert!(ObjectId::parse(s).is_err());
        }
    }

    #[test]
    fn test_invalid_version_id_rejected(s in "[^v][^e][^r][^_].{0,124}") {
        if !s.starts_with("ver_") {
            assert!(VersionId::parse(s).is_err());
        }
    }
}
