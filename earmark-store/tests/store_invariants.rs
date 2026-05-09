use earmark_core::*;
use earmark_store::*;
use proptest::prelude::*;
use tempfile::tempdir;

proptest! {
    #[test]
    fn test_payload_ref_deterministic(bytes in prop::collection::vec(0u8..255u8, 0..1024)) {
        let ref1 = PayloadRef::from_bytes(&bytes);
        let ref2 = PayloadRef::from_bytes(&bytes);
        assert_eq!(ref1, ref2);
        assert!(ref1.0.starts_with("sha256:"));
    }

    #[test]
    fn test_store_roundtrip(
        kind in prop_oneof![Just(Kind::Object), Just(Kind::Instruction), Just(Kind::Policy)],
        name in prop::option::of("[a-z]{1,10}"),
        payload_bytes in prop::collection::vec(0u8..255u8, 1..1024)
    ) {
        let dir = tempdir().expect("failed to create temp dir");
        let store = GitCanonicalStore::new(dir.path());
        store.init_layout().expect("failed to init layout");

        let object = StoredObject::new(
            kind.clone(),
            name.clone(),
            Standing::default(),
            Provenance::direct_input("test"),
            std::collections::BTreeMap::new(),
            StoredPayload {
                format: PayloadEncoding::Json,
                bytes: payload_bytes.clone(),
            },
            vec![],
        );

        let vref = store.write_object(&object).expect("failed to write object");
        let read_back = store.read_version(&vref).expect("failed to read back");

        assert_eq!(read_back.envelope.kind, kind);
        assert_eq!(read_back.envelope.id, vref.id);
        assert_eq!(read_back.envelope.version_id, vref.version_id);
        assert_eq!(read_back.payload.bytes, payload_bytes);
    }

    #[test]
    fn test_relation_roundtrip(
        source_id in "obj_[a-z0-9]{32}",
        target_id in "obj_[a-z0-9]{32}",
        rel_type in "[a-z]{1,20}"
    ) {
        let dir = tempdir().expect("failed to create temp dir");
        let store = GitCanonicalStore::new(dir.path());
        store.init_layout().expect("failed to init layout");

        let source = ObjectRef::new(ObjectId::parse(source_id).unwrap(), VersionId::new(), Kind::Object, None);
        let target = ObjectRef::new(ObjectId::parse(target_id).unwrap(), VersionId::new(), Kind::Object, None);

        let rel_payload = RelationPayload {
            source: source.clone(),
            target: target.clone(),
            relation_type: rel_type.clone(),
            qualifiers: std::collections::BTreeMap::new(),
            scope: Some("test".to_string()),
        };

        let object = StoredObject::new(
            Kind::Relation,
            None,
            Standing::default(),
            Provenance::direct_input("test"),
            std::collections::BTreeMap::new(),
            StoredPayload::from_json_bytes(serde_json::to_vec(&rel_payload).unwrap()),
            vec![],
        );

        let vref = store.write_object(&object).expect("failed to write relation");
        let read_back = store.read_version(&vref).expect("failed to read back relation");
        let parsed_rel: RelationPayload = serde_json::from_slice(&read_back.payload.bytes).expect("failed to parse rel payload");

        assert_eq!(parsed_rel.source.id, source.id);
        assert_eq!(parsed_rel.target.id, target.id);
        assert_eq!(parsed_rel.relation_type, rel_type);
    }
}
