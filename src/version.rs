use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct VersionInfo {
    pub raw: String,
    pub version_format: String,
    pub calver_scheme: Option<String>,
    pub calver_parts: Option<serde_json::Value>,
    pub modifier: Option<String>,
    pub semver_parts: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct VersionOptions {
    pub allow_yy_calver: bool,
    pub year_min: i32,
    pub year_max: i32,
}

pub fn classify_version(raw: &str, options: &VersionOptions) -> VersionInfo {
    let trimmed = raw.trim();
    let (core, modifier) = split_modifier(trimmed);
    if core.contains('+') {
        return semver_or_unknown(trimmed);
    }
    if core.contains('.') && core.contains('-') {
        return semver_or_unknown(trimmed);
    }
    if let Some(info) = classify_calver(core, modifier.as_deref(), options) {
        return info;
    }
    semver_or_unknown(trimmed)
}

fn semver_or_unknown(raw: &str) -> VersionInfo {
    match semver::Version::parse(raw) {
        Ok(version) => VersionInfo {
            raw: raw.to_string(),
            version_format: "semver".to_string(),
            calver_scheme: None,
            calver_parts: None,
            modifier: None,
            semver_parts: Some(serde_json::json!({
                "major": version.major,
                "minor": version.minor,
                "patch": version.patch,
                "pre": if version.pre.is_empty() { None } else { Some(version.pre.to_string()) },
                "build": if version.build.is_empty() { None } else { Some(version.build.to_string()) },
            })),
        },
        Err(_) => VersionInfo {
            raw: raw.to_string(),
            version_format: "unknown".to_string(),
            calver_scheme: None,
            calver_parts: None,
            modifier: None,
            semver_parts: None,
        },
    }
}

fn classify_calver(
    core: &str,
    modifier: Option<&str>,
    options: &VersionOptions,
) -> Option<VersionInfo> {
    if let Some((year, month)) = parse_year_month_yyyy(core, '.', options) {
        return Some(VersionInfo {
            raw: core.to_string(),
            version_format: "calver".to_string(),
            calver_scheme: Some("YYYY.MM".to_string()),
            calver_parts: Some(serde_json::json!({ "year": year, "month": month })),
            modifier: modifier.map(|m| m.to_string()),
            semver_parts: None,
        });
    }
    if let Some((year, month, micro)) = parse_year_month_micro_yyyy(core, '.', options) {
        return Some(VersionInfo {
            raw: core.to_string(),
            version_format: "calver".to_string(),
            calver_scheme: Some("YYYY.MM.MICRO".to_string()),
            calver_parts: Some(serde_json::json!({ "year": year, "month": month, "micro": micro })),
            modifier: modifier.map(|m| m.to_string()),
            semver_parts: None,
        });
    }
    if let Some((year, month, day)) = parse_year_month_day(core, '-', options) {
        return Some(VersionInfo {
            raw: core.to_string(),
            version_format: "calver".to_string(),
            calver_scheme: Some("YYYY-MM-DD".to_string()),
            calver_parts: Some(serde_json::json!({ "year": year, "month": month, "day": day })),
            modifier: modifier.map(|m| m.to_string()),
            semver_parts: None,
        });
    }
    if let Some(info) = parse_yyyymmdd(core, options) {
        return Some(VersionInfo {
            raw: core.to_string(),
            version_format: "calver".to_string(),
            calver_scheme: Some(info.0),
            calver_parts: Some(info.1),
            modifier: modifier.map(|m| m.to_string()),
            semver_parts: None,
        });
    }
    if options.allow_yy_calver {
        if let Some((year, month)) = parse_year_month_yy(core, '.') {
            let full_year = 2000 + year;
            return Some(VersionInfo {
                raw: core.to_string(),
                version_format: "calver".to_string(),
                calver_scheme: Some("YY.MM".to_string()),
                calver_parts: Some(serde_json::json!({ "year": full_year, "month": month })),
                modifier: modifier.map(|m| m.to_string()),
                semver_parts: None,
            });
        }
        if let Some((year, month, micro)) = parse_year_month_micro_yy(core, '.') {
            let full_year = 2000 + year;
            return Some(VersionInfo {
                raw: core.to_string(),
                version_format: "calver".to_string(),
                calver_scheme: Some("YY.MM.MICRO".to_string()),
                calver_parts: Some(
                    serde_json::json!({ "year": full_year, "month": month, "micro": micro }),
                ),
                modifier: modifier.map(|m| m.to_string()),
                semver_parts: None,
            });
        }
    }
    None
}

fn split_modifier(raw: &str) -> (&str, Option<String>) {
    if let Some(idx) = raw.rfind('-') {
        let (core, suffix) = raw.split_at(idx);
        if suffix.len() > 1 {
            let candidate = &suffix[1..];
            if candidate.chars().any(|c| c.is_ascii_alphabetic()) {
                return (core, Some(candidate.to_string()));
            }
        }
    }
    let mut last_digit = None;
    for (idx, ch) in raw.char_indices() {
        if ch.is_ascii_digit() {
            last_digit = Some(idx);
        }
    }
    if let Some(idx) = last_digit {
        let tail = &raw[idx + 1..];
        if !tail.is_empty() && tail.chars().any(|c| c.is_ascii_alphabetic()) {
            return (&raw[..=idx], Some(tail.to_string()));
        }
    }
    (raw, None)
}

fn parse_year_month_yyyy(core: &str, sep: char, options: &VersionOptions) -> Option<(u32, u32)> {
    let parts: Vec<&str> = core.split(sep).collect();
    if parts.len() != 2 {
        return None;
    }
    if sep == '.' && parts[0].len() == 4 {
        let year = parts[0].parse::<u32>().ok()?;
        let month = parts[1].parse::<u32>().ok()?;
        if year < options.year_min as u32
            || year > options.year_max as u32
            || !(1..=12).contains(&month)
        {
            return None;
        }
        return Some((year, month));
    }
    None
}

fn parse_year_month_yy(core: &str, sep: char) -> Option<(u32, u32)> {
    let parts: Vec<&str> = core.split(sep).collect();
    if parts.len() != 2 {
        return None;
    }
    if sep == '.' && parts[0].len() == 2 {
        let year = parts[0].parse::<u32>().ok()?;
        let month = parts[1].parse::<u32>().ok()?;
        if !(1..=12).contains(&month) || year > 99 {
            return None;
        }
        return Some((year, month));
    }
    None
}

fn parse_year_month_micro_yy(core: &str, sep: char) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = core.split(sep).collect();
    if parts.len() != 3 {
        return None;
    }
    if sep == '.' && parts[0].len() == 2 {
        let year = parts[0].parse::<u32>().ok()?;
        let month = parts[1].parse::<u32>().ok()?;
        let micro = parts[2].parse::<u32>().ok()?;
        if !(1..=12).contains(&month) || year > 99 {
            return None;
        }
        return Some((year, month, micro));
    }
    None
}

fn parse_year_month_micro_yyyy(
    core: &str,
    sep: char,
    options: &VersionOptions,
) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = core.split(sep).collect();
    if parts.len() != 3 {
        return None;
    }
    if sep == '.' && parts[0].len() == 4 {
        let year = parts[0].parse::<u32>().ok()?;
        let month = parts[1].parse::<u32>().ok()?;
        let micro = parts[2].parse::<u32>().ok()?;
        if year < options.year_min as u32
            || year > options.year_max as u32
            || !(1..=12).contains(&month)
        {
            return None;
        }
        return Some((year, month, micro));
    }
    None
}

fn parse_year_month_day(
    core: &str,
    sep: char,
    options: &VersionOptions,
) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = core.split(sep).collect();
    if parts.len() != 3 {
        return None;
    }
    if sep == '-' && parts[0].len() == 4 {
        let year = parts[0].parse::<u32>().ok()?;
        let month = parts[1].parse::<u32>().ok()?;
        let day = parts[2].parse::<u32>().ok()?;
        if year < options.year_min as u32
            || year > options.year_max as u32
            || !(1..=12).contains(&month)
            || !(1..=31).contains(&day)
        {
            return None;
        }
        return Some((year, month, day));
    }
    None
}

fn parse_yyyymmdd(core: &str, options: &VersionOptions) -> Option<(String, serde_json::Value)> {
    let (date, micro) = if let Some((date, micro)) = core.split_once('.') {
        (date, Some(micro))
    } else {
        (core, None)
    };
    if date.len() != 8 || !date.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let year = date[0..4].parse::<u32>().ok()?;
    let month = date[4..6].parse::<u32>().ok()?;
    let day = date[6..8].parse::<u32>().ok()?;
    if year < options.year_min as u32
        || year > options.year_max as u32
        || !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
    {
        return None;
    }
    if let Some(micro_str) = micro {
        let micro = micro_str.parse::<u32>().ok()?;
        return Some((
            "YYYYMMDD.MICRO".to_string(),
            serde_json::json!({ "year": year, "month": month, "day": day, "micro": micro }),
        ));
    }
    Some((
        "YYYYMMDD".to_string(),
        serde_json::json!({ "year": year, "month": month, "day": day }),
    ))
}
