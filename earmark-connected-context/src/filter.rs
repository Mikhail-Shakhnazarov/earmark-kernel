use earmark_index::ObjectSummary;
use std::collections::BTreeMap;

pub fn object_summary_matches_standing(
    row: &ObjectSummary,
    standing_filters: &BTreeMap<String, Vec<String>>,
) -> bool {
    if standing_filters.is_empty() {
        return true;
    }
    standing_filters.iter().all(|(dimension, allowed)| {
        if allowed.is_empty() {
            return true;
        }
        let current = match dimension.as_str() {
            "epistemic" | "kernel:epistemic" => row
                .standing
                .get("kernel:epistemic")
                .map(String::as_str)
                .unwrap_or(&row.standing_epistemic),
            "review" | "kernel:review" => row
                .standing
                .get("kernel:review")
                .map(String::as_str)
                .unwrap_or(&row.standing_review),
            "process" | "kernel:process" => row
                .standing
                .get("kernel:process")
                .map(String::as_str)
                .unwrap_or(&row.standing_process),
            other => match row.standing.get(other) {
                Some(v) => v.as_str(),
                None => return false,
            },
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
