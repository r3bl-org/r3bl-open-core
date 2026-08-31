// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

# Examples & Patterns for `check-test-coverage`

This document provides real-world audit examples and pattern comparisons to guide test coverage reviews under the zero test-bloat directive.

---

## Example 1: Auditing Enum & Conversion Types (`args.rs`)

### Target Code (`tui/src/core/script/env_source/args.rs`)

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum BaseEnv {
    #[default]
    Inherit,
    Explicit(EnvMap),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputKind {
    File(PathBuf),
    InlineCommand(String),
}

impl From<(&Option<PathBuf>, &Option<String>)> for InputKind {
    fn from(tuple: (&Option<PathBuf>, &Option<String>)) -> InputKind {
        match tuple {
            (Some(file), None) => InputKind::File(file.clone()),
            (None, Some(cmd)) => InputKind::InlineCommand(cmd.clone()),
            _ => unreachable!("Guaranteed by caller passing mutually exclusive input options"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum OutputFormat {
    Fish,
    Powershell,
    Json,
    Dotenv,
}
```

### Coverage & Bloat Audit

| Component | Code Type | Branches / Paths | Audit Result | Action Required |
| :--- | :--- | :--- | :--- | :--- |
| `BaseEnv` | Derive | `#[derive(Default)]` | Upstream compiler derive | **Do NOT test** (testing `BaseEnv::default()` tests the compiler) |
| `OutputFormat` | Derive | `#[derive(Display, EnumString)]` | Upstream `strum` derive | **Do NOT test** (testing `to_string()` tests `strum`) |
| `InputKind::from` (Path 1) | Custom Logic | `(Some(file), None)` | Covered in unit test | None (Covered by `test_tuple_to_input_kind_file`) |
| `InputKind::from` (Path 2) | Custom Logic | `(None, Some(cmd))` | Covered in unit test | None (Covered by `test_tuple_to_input_kind_command`) |

### Conclusion
**Verdict: Sufficient.** All custom code branches are tested. No dependency bloat tests present.

---

## Example 2: Good vs Bad Test Practices

### Bad: Testing the Standard Library and Third-Party Crates (Test Bloat)

```rust
// ❌ BAD: This tests std and strum, not our custom logic.
#[test]
fn test_output_format_strum_display() {
    assert_eq!(OutputFormat::Fish.to_string(), "fish");
    assert_eq!(OutputFormat::Powershell.to_string(), "powershell");
}

// ❌ BAD: This tests the Rust compiler's derive macro.
#[test]
fn test_base_env_default() {
    assert_eq!(BaseEnv::default(), BaseEnv::Inherit);
}

// ❌ BAD: This tests std::collections::HashMap, not our code.
#[test]
fn test_env_map_insert_and_get() {
    let mut map = EnvMap::new();
    map.insert("KEY".into(), "VAL".into());
    assert_eq!(map.get("KEY"), Some(&"VAL".into()));
}
```

### Good: Branch-Targeted Testing of Custom Logic

```rust
// ✅ GOOD: Tests our custom parsing and fallback logic.
#[test]
fn test_parse_environment_block_with_comments_and_empty_lines() {
    let input = "# Comment line\n\nKEY=VALUE\nEMPTY=\n";
    let parsed = parse_env_block(input);
    assert_eq!(parsed.get("KEY"), Some(&"VALUE".to_string()));
    assert_eq!(parsed.get("EMPTY"), Some(&"".to_string()));
    assert_eq!(parsed.len(), 2);
}

// ✅ GOOD: Tests our custom diff calculation state machine.
#[test]
fn test_diff_identifies_added_modified_and_removed_variables() {
    let initial = create_env([("STAYS", "1"), ("MODIFIED", "old"), ("REMOVED", "bye")]);
    let current = create_env([("STAYS", "1"), ("MODIFIED", "new"), ("ADDED", "hello")]);

    let diff = EnvDiff::compute(&initial, &current);
    assert_eq!(diff.added.get("ADDED"), Some(&"hello".to_string()));
    assert_eq!(diff.modified.get("MODIFIED"), Some(&"new".to_string()));
    assert_eq!(diff.removed, vec!["REMOVED".to_string()]);
}
```
