use earmark_core::{ObjectId, SymbolicName, VersionId};

#[test]
fn test_empty_ids_rejected() {
    assert!(ObjectId::parse("").is_err());
    assert!(VersionId::parse("").is_err());
    assert!(SymbolicName::parse("").is_err());
}

#[test]
fn test_overlong_ids_rejected() {
    // ObjectId / VersionId limit is 128 characters
    let overlong_obj = format!("obj_{}", "a".repeat(125));
    assert!(ObjectId::parse(overlong_obj).is_err());

    let overlong_ver = format!("ver_{}", "a".repeat(125));
    assert!(VersionId::parse(overlong_ver).is_err());

    // SymbolicName limit is 64 characters
    let overlong_symbolic = "a".repeat(65);
    assert!(SymbolicName::parse(overlong_symbolic).is_err());
}

#[test]
fn test_unicode_confusables_rejected() {
    // Replace latin 'o' in prefix with Cyrillic 'о' (U+043E)
    let cyrillic_o = "\u{043E}bj_00000000000000000000000000000001";
    assert!(ObjectId::parse(cyrillic_o).is_err());

    // Replace latin 'e' in prefix with Cyrillic 'е' (U+0435)
    let cyrillic_e = "v\u{0435}r_00000000000000000000000000000001";
    assert!(VersionId::parse(cyrillic_e).is_err());

    // Homoglyphs/confusables inside symbolic names
    let confusable_symbolic = "my\u{0430}name"; // Cyrillic 'а' instead of Latin 'a'
    assert!(SymbolicName::parse(confusable_symbolic).is_err());
}

#[test]
fn test_whitespace_around_ids_rejected() {
    let raw_obj = "obj_00000000000000000000000000000001";
    assert!(ObjectId::parse(format!(" {}", raw_obj)).is_err());
    assert!(ObjectId::parse(format!("{} ", raw_obj)).is_err());
    assert!(ObjectId::parse(format!("\t{}", raw_obj)).is_err());
    assert!(ObjectId::parse(format!("{}\n", raw_obj)).is_err());

    let raw_ver = "ver_00000000000000000000000000000001";
    assert!(VersionId::parse(format!(" {}", raw_ver)).is_err());
    assert!(VersionId::parse(format!("{} ", raw_ver)).is_err());

    let raw_sym = "my-name";
    assert!(SymbolicName::parse(format!(" {}", raw_sym)).is_err());
    assert!(SymbolicName::parse(format!("{} ", raw_sym)).is_err());
}

#[test]
fn test_wrong_prefixes_rejected() {
    assert!(ObjectId::parse("objx_00000000000000000000000000000001").is_err());
    assert!(ObjectId::parse("ob_00000000000000000000000000000001").is_err());
    assert!(ObjectId::parse("ver_00000000000000000000000000000001").is_err());

    assert!(VersionId::parse("very_00000000000000000000000000000001").is_err());
    assert!(VersionId::parse("ve_00000000000000000000000000000001").is_err());
    assert!(VersionId::parse("obj_00000000000000000000000000000001").is_err());
}

#[test]
fn test_valid_prefix_invalid_body_rejected() {
    // Non-lowercase alphanumeric character G
    assert!(ObjectId::parse("obj_0000000000000000000000000000000G").is_err());
    assert!(ObjectId::parse("obj_0000000000000000000000000000000_").is_err());
    // Too short (31 hex chars)
    assert!(ObjectId::parse("obj_0000000000000000000000000000000").is_err());
    // Too long (33 hex chars)
    assert!(ObjectId::parse("obj_000000000000000000000000000000001").is_err());

    // Non-lowercase alphanumeric character G in version ID
    assert!(VersionId::parse("ver_0000000000000000000000000000000G").is_err());
    assert!(VersionId::parse("ver_0000000000000000000000000000000_").is_err());
    // Too short version ID
    assert!(VersionId::parse("ver_0000000000000000000000000000000").is_err());
    // Too long version ID
    assert!(VersionId::parse("ver_000000000000000000000000000000001").is_err());
}
