use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

fn cmd() -> Command {
    cargo_bin_cmd!("themalingadingdong")
}

#[test]
fn test_cli_generates_yaml() {
    cmd()
        .args([
            "--background",
            "#1a1a2e",
            "--foreground",
            "#eaeaea",
            "--name",
            "Test Scheme",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("system: base24"))
        .stdout(predicate::str::contains("name: Test Scheme"))
        .stdout(predicate::str::contains("palette:"))
        .stdout(predicate::str::contains("base00:"))
        .stdout(predicate::str::contains("base07:"));
}

#[test]
fn test_cli_includes_all_base24_colors() {
    let output = cmd()
        .args([
            "--background",
            "#000000",
            "--foreground",
            "#ffffff",
            "--name",
            "Full Palette",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).unwrap();

    // Check base00-base0F
    for i in 0..16 {
        let key = format!("base0{:X}:", i);
        assert!(stdout.contains(&key), "Output should contain {}", key);
    }

    // Check base10-base17
    for i in 0..8 {
        let key = format!("base1{}:", i);
        assert!(stdout.contains(&key), "Output should contain {}", key);
    }
}

#[test]
fn test_cli_no_adjust_with_low_contrast_fails() {
    // Very low minimum contrast may not meet validation thresholds
    cmd()
        .args([
            "--background",
            "#1a1a2e",
            "--foreground",
            "#eaeaea",
            "--min-contrast",
            "30", // Very low contrast
            "--name",
            "Test",
            "--no-adjust",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Validation failed"));
}

#[test]
fn test_cli_high_contrast_passes_without_adjust() {
    // High minimum contrast on dark background should mostly pass
    cmd()
        .args([
            "--background",
            "#000000",
            "--foreground",
            "#ffffff",
            "--min-contrast",
            "90",
            "--accent-colorfulness",
            "10",
            "--name",
            "High Contrast",
        ])
        .assert()
        .success();
}

#[test]
fn test_cli_hue_override() {
    // Test overriding individual hues (e.g., make base08 pink instead of red)
    cmd()
        .args([
            "--background",
            "#1a1a2e",
            "--foreground",
            "#eaeaea",
            "--hue-08",
            "340", // Pink instead of default red (0°)
            "--name",
            "Custom Hue Theme",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("base08:")); // Should still have accents
}

#[test]
fn test_cli_invalid_hex_fails() {
    cmd()
        .args([
            "--background",
            "invalid",
            "--foreground",
            "#ffffff",
            "--name",
            "Test",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid background color"));
}

#[test]
fn test_cli_slug_generation() {
    cmd()
        .args([
            "--background",
            "#1a1a2e",
            "--foreground",
            "#eaeaea",
            "--name",
            "My Cool Theme",
        ])
        .assert()
        .success()
        // Slug now includes variant suffix
        .stdout(predicate::str::contains("slug: my-cool-theme-dark"));
}

#[test]
fn test_cli_dark_variant_detected() {
    cmd()
        .args([
            "--background",
            "#1a1a2e", // Dark background
            "--foreground",
            "#eaeaea",
            "--name",
            "Dark Theme",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("variant: dark"));
}

#[test]
fn test_cli_light_variant_detected() {
    cmd()
        .args([
            "--background",
            "#fafafa", // Light background
            "--foreground",
            "#1a1a1a",
            "--name",
            "Light Theme",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("variant: light"));
}

#[test]
fn test_cli_variant_dark() {
    cmd()
        .args([
            "--background",
            "#1a1a2e",
            "--foreground",
            "#eaeaea",
            "--variant",
            "dark",
            "--name",
            "Dark Test",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("variant: dark"));
}

#[test]
fn test_cli_variant_light() {
    // Even with dark background, --variant light forces light output
    cmd()
        .args([
            "--background",
            "#1a1a2e",
            "--foreground",
            "#eaeaea",
            "--variant",
            "light",
            "--name",
            "Light Test",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("variant: light"));
}

#[test]
fn test_cli_variant_both_requires_output() {
    cmd()
        .args([
            "--background",
            "#1a1a2e",
            "--foreground",
            "#eaeaea",
            "--variant",
            "both",
            "--name",
            "Test",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--variant both requires --output"));
}
