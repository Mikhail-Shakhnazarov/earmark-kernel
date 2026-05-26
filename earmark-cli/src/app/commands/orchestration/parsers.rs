use std::collections::BTreeMap;

pub fn parse_manifest_sections(text: &str) -> BTreeMap<String, String> {
    let mut sections = BTreeMap::new();
    let mut current_section = "_preamble".to_string();
    let mut current_content = Vec::new();

    for line in text.lines() {
        if line.starts_with("# ") || line.starts_with("## ") {
            if !current_content.is_empty() {
                sections.insert(
                    current_section.clone(),
                    current_content.join("\n").trim().to_string(),
                );
                current_content.clear();
            }
            current_section = line.trim_start_matches('#').trim().to_lowercase();
        } else {
            current_content.push(line);
        }
    }
    if !current_content.is_empty() {
        sections.insert(
            current_section,
            current_content.join("\n").trim().to_string(),
        );
    }
    sections
}

pub fn parse_header_pairs(text: &str) -> BTreeMap<String, String> {
    let mut pairs = BTreeMap::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut in_yaml = false;
    for line in lines {
        if line.trim() == "---" {
            in_yaml = !in_yaml;
            continue;
        }
        if in_yaml {
            if let Some((k, v)) = line.split_once(':') {
                pairs.insert(k.trim().to_lowercase(), v.trim().to_string());
            }
        }
    }
    pairs
}

pub fn parse_task_id_from_filename(filename: &str) -> Option<String> {
    filename.split('-').next().map(|s| s.to_string())
}

pub fn parse_attempt_from_filename(filename: &str) -> Option<usize> {
    filename.split('-').nth(1).and_then(|s| s.parse().ok())
}

pub fn parse_fenced_code_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current_block = Vec::new();
    let mut in_block = false;
    for line in text.lines() {
        if line.starts_with("```") {
            if in_block {
                blocks.push(current_block.join("\n"));
                current_block.clear();
                in_block = false;
            } else {
                in_block = true;
            }
        } else if in_block {
            current_block.push(line);
        }
    }
    blocks
}

pub fn parse_bullet_list(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.trim())
        .filter(|l| l.starts_with("- ") || l.starts_with("* "))
        .map(|l| l[2..].trim().to_string())
        .collect()
}

pub fn parse_files_changed(sections: &BTreeMap<String, String>) -> Vec<String> {
    if let Some(content) = sections
        .get("files changed")
        .or_else(|| sections.get("affected files"))
    {
        parse_bullet_list(content)
    } else {
        Vec::new()
    }
}
