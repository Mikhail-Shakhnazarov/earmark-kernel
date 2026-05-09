use earmark_core::{CoreError, ObjectId};

#[test]
fn test_path_traversal_hardened() {
    // 1. Verify that parse rejects traversal strings
    let malicious = "../../../traversal";
    let res = ObjectId::parse(malicious);
    assert!(matches!(res, Err(CoreError::InvalidIdentifier(_))));

    // 2. Verify that even with a "legal" looking but still potentially problematic string (if it had dots), it would fail
    // But our regex [a-z0-9]{32} strictly prevents dots.
    
    let bad_prefix = "bad.prefix";
    assert!(ObjectId::parse(bad_prefix).is_err());

    let invalid_start = "1abc";
    assert!(ObjectId::parse(invalid_start).is_err());
}
