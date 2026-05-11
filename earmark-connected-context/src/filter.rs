use earmark_index::ObjectSummary;
use std::collections::BTreeMap;

pub fn object_summary_matches_standing(
    row: &ObjectSummary,
    standing_filters: &BTreeMap<String, Vec<String>>,
) -> bool {
    standing_filters.iter().all(|(dimension, allowed)| {
        if allowed.is_empty() {
            return true;
        }
        let current = match dimension.as_str() {
            "epistemic" => &row.standing_epistemic,
            "review" => &row.standing_review,
            "process" => &row.standing_process,
            "kernel:epistemic" => &row.standing_epistemic,
            "kernel:review" => &row.standing_review,
            "kernel:process" => &row.standing_process,
            _ => return true, // Unknown dimensions pass for forward compatibility
        };
        allowed.iter().any(|candidate| candidate == current)
    })
}

pub fn object_summary_matches_classes(row: &ObjectSummary, allowed_classes: &[String]) -> bool {
    if allowed_classes.is_empty() {
        return true;
    }
    let Some(current_class) = &row.class else {
        return false;
    };
    allowed_classes.iter().any(|c| c == current_class)
}

pub fn object_summary_admissible(
    row: &ObjectSummary,
    classes: &[String],
    standing_filters: &BTreeMap<String, Vec<String>>,
) -> bool {
    object_summary_matches_classes(row, classes)
        && object_summary_matches_standing(row, standing_filters)
}

pub fn relation_type_admissible(relation_type: &str, allowed_relation_types: &[String]) -> bool {
    if allowed_relation_types.is_empty() {
        return true;
    }
    allowed_relation_types.iter().any(|r| r == relation_type)
}
