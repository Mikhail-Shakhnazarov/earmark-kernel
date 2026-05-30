/*
 * Copyright (c) 2026 Mikhail Shakhnazarov.
 * Dual-licensed under AGPL-3.0-or-later or commercial terms.
 */

pub fn unwrap_hard_wrapped_prose(input: &str) -> String {
    let mut output = Vec::new();
    let mut paragraph = Vec::new();
    let mut in_code_fence = false;

    for raw_line in input.lines() {
        let line = raw_line.trim_end_matches('\r');
        let trimmed = line.trim();

        if is_code_fence_line(trimmed) {
            flush_paragraph(&mut output, &mut paragraph);
            output.push(line.to_string());
            in_code_fence = !in_code_fence;
            continue;
        }

        if in_code_fence {
            output.push(line.to_string());
            continue;
        }

        if trimmed.is_empty() {
            flush_paragraph(&mut output, &mut paragraph);
            output.push(String::new());
            continue;
        }

        if is_structural_markdown_line(line) {
            flush_paragraph(&mut output, &mut paragraph);
            output.push(line.to_string());
            continue;
        }

        paragraph.push(trimmed.to_string());
    }

    flush_paragraph(&mut output, &mut paragraph);

    let mut rendered = output.join("\n");
    if input.ends_with('\n') {
        rendered.push('\n');
    }
    rendered
}

fn flush_paragraph(output: &mut Vec<String>, paragraph: &mut Vec<String>) {
    if paragraph.is_empty() {
        return;
    }

    output.push(paragraph.join(" "));
    paragraph.clear();
}

fn is_code_fence_line(trimmed: &str) -> bool {
    trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

fn is_structural_markdown_line(line: &str) -> bool {
    let trimmed_start = line.trim_start();

    if line.starts_with("    ") || line.starts_with('\t') {
        return true;
    }

    if trimmed_start.starts_with('#')
        || trimmed_start.starts_with('>')
        || trimmed_start.starts_with('|')
        || trimmed_start.starts_with("<!--")
        || trimmed_start.starts_with('<')
        || trimmed_start == "---"
        || trimmed_start == "***"
        || trimmed_start == "___"
        || trimmed_start.starts_with("- ")
        || trimmed_start.starts_with("* ")
        || trimmed_start.starts_with("+ ")
    {
        return true;
    }

    if trimmed_start.starts_with('[') && trimmed_start.contains("]:") {
        return true;
    }

    let digits = trimmed_start
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .count();
    if digits > 0 {
        let rest = &trimmed_start[digits..];
        if rest.starts_with(". ") || rest.starts_with(") ") {
            return true;
        }
    }

    // Heuristic: if a line is extremely short but doesn't look like prose continuation,
    // we might want to keep it structural, but for now we follow the CLI logic.

    false
}
