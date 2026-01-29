use crate::readme::extract_marker_block_lines;
use crate::readme_badges::parse_badge_line_optional;

#[derive(Debug)]
pub struct RemovalOutcome {
    pub remaining: Vec<String>,
    pub removed: usize,
    pub id_hits: usize,
    pub removed_ids: Vec<String>,
    pub removed_kinds: std::collections::HashMap<String, usize>,
    pub missing_ids: Vec<String>,
}

pub fn remove_block_lines_by_id_kind(
    content: &str,
    ids: &[String],
    kinds: &[String],
    strict: bool,
) -> anyhow::Result<RemovalOutcome> {
    let lines = extract_marker_block_lines(content)?;
    let id_set = ids
        .iter()
        .map(|s| s.trim().to_string())
        .collect::<std::collections::HashSet<_>>();
    let kind_set = kinds
        .iter()
        .map(|s| s.trim().to_string())
        .collect::<std::collections::HashSet<_>>();
    let mut remaining = Vec::new();
    let mut removed = 0;
    let mut id_hits = 0;
    let mut removed_ids = Vec::new();
    let mut removed_kinds = std::collections::HashMap::new();

    let mut in_code_fence = false;
    for line in lines {
        if is_code_fence(&line) {
            in_code_fence = !in_code_fence;
            remaining.push(line);
            continue;
        }
        if in_code_fence {
            remaining.push(line);
            continue;
        }
        let parsed = parse_badge_line_optional(&line);
        let id_candidate = parsed
            .as_ref()
            .map(|badge| badge.id.clone())
            .unwrap_or_else(|| format!("unknown:{}", hash_line(&line)));
        let kind_candidate = parsed
            .as_ref()
            .map(|badge| badge.kind.clone())
            .unwrap_or_else(|| "unknown".to_string());
        let remove_by_id = !id_set.is_empty() && id_set.contains(&id_candidate);
        let remove_by_kind = !kind_set.is_empty() && kind_set.contains(&kind_candidate);
        if remove_by_id || remove_by_kind {
            if remove_by_id {
                id_hits += 1;
            }
            removed_ids.push(id_candidate.clone());
            *removed_kinds.entry(kind_candidate).or_insert(0) += 1;
            removed += 1;
            continue;
        }
        remaining.push(line);
    }

    let missing_ids = if id_set.is_empty() {
        Vec::new()
    } else {
        id_set
            .iter()
            .filter(|id| !removed_ids.contains(*id))
            .cloned()
            .collect()
    };

    if strict && !id_set.is_empty() && id_hits == 0 {
        anyhow::bail!("id_not_found");
    }

    Ok(RemovalOutcome {
        remaining,
        removed,
        id_hits,
        removed_ids,
        removed_kinds,
        missing_ids,
    })
}

fn is_code_fence(line: &str) -> bool {
    line.trim_start().starts_with("```")
}

fn hash_line(line: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    line.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
