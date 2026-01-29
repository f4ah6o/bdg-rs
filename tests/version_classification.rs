use bdg::version::{classify_version, VersionOptions};

#[test]
fn calver_formats() {
    let info = classify_version("2026.01", &options(false, 2000, 2199));
    assert_eq!(info.version_format, "calver");
    assert_eq!(info.calver_scheme.as_deref(), Some("YYYY.MM"));

    let info = classify_version("2026.01.3", &options(false, 2000, 2199));
    assert_eq!(info.version_format, "calver");
    assert_eq!(info.calver_scheme.as_deref(), Some("YYYY.MM.MICRO"));

    let info = classify_version("2026-01-29", &options(false, 2000, 2199));
    assert_eq!(info.version_format, "calver");
    assert_eq!(info.calver_scheme.as_deref(), Some("YYYY-MM-DD"));

    let info = classify_version("20260129", &options(false, 2000, 2199));
    assert_eq!(info.version_format, "calver");
    assert_eq!(info.calver_scheme.as_deref(), Some("YYYYMMDD"));

    let info = classify_version("20260129.1", &options(false, 2000, 2199));
    assert_eq!(info.version_format, "calver");
    assert_eq!(info.calver_scheme.as_deref(), Some("YYYYMMDD.MICRO"));
}

#[test]
fn calver_invalid_ranges_are_unknown() {
    let info = classify_version("2026.13", &options(false, 2000, 2199));
    assert_eq!(info.version_format, "unknown");

    let info = classify_version("2026-00-10", &options(false, 2000, 2199));
    assert_eq!(info.version_format, "unknown");
}

#[test]
fn yy_micro_only_with_allow() {
    let info = classify_version("26.01.3", &options(false, 2000, 2199));
    assert_eq!(info.version_format, "unknown");
}

#[test]
fn semver_formats() {
    let info = classify_version("1.2.3", &options(false, 2000, 2199));
    assert_eq!(info.version_format, "semver");

    let info = classify_version("1.2.3-alpha.1", &options(false, 2000, 2199));
    assert_eq!(info.version_format, "semver");

    let info = classify_version("1.2.3+build.5", &options(false, 2000, 2199));
    assert_eq!(info.version_format, "semver");
}

#[test]
fn calver_semver_conflict() {
    let info = classify_version("2.6.1", &options(false, 2000, 2199));
    assert_eq!(info.version_format, "semver");

    let info = classify_version("26.01", &options(false, 2000, 2199));
    assert_eq!(info.version_format, "unknown");

    let info = classify_version("26.01", &options(true, 2000, 2199));
    assert_eq!(info.version_format, "calver");
    assert_eq!(info.calver_scheme.as_deref(), Some("YY.MM"));

    let info = classify_version("26.01.3", &options(true, 2000, 2199));
    assert_eq!(info.version_format, "calver");
    assert_eq!(info.calver_scheme.as_deref(), Some("YY.MM.MICRO"));
}

#[test]
fn year_range_limits_calver() {
    let info = classify_version("2026.01", &options(false, 2027, 2199));
    assert_eq!(info.version_format, "unknown");
}

fn options(allow_yy: bool, year_min: i32, year_max: i32) -> VersionOptions {
    VersionOptions {
        allow_yy_calver: allow_yy,
        year_min,
        year_max,
    }
}
