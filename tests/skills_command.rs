use std::process::Command;

const EXPECTED_SKILL: &str = include_str!("../.agents/skills/bdg/SKILL.md");

#[test]
fn skills_command_prints_bundled_skill() {
    let output = Command::new(env!("CARGO_BIN_EXE_bdg"))
        .arg("skills")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), EXPECTED_SKILL);
    assert!(output.stderr.is_empty());
}

#[test]
fn bundled_skill_has_required_metadata() {
    assert!(EXPECTED_SKILL.starts_with("---\n"));
    assert!(EXPECTED_SKILL.contains("\nname: bdg\n"));
    assert!(EXPECTED_SKILL.contains("\ndescription: "));
    assert!(EXPECTED_SKILL.contains("`bdg add`"));
    assert!(EXPECTED_SKILL.contains("`bdg list`"));
    assert!(EXPECTED_SKILL.contains("`bdg remove`"));
    assert!(EXPECTED_SKILL.contains("`bdg skills`"));
}
