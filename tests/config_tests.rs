use themalingadingdong::config::{HueOverrides, ThemeConfig};

#[test]
fn test_default_config() {
    let config = ThemeConfig::default();
    assert!(config.theme.name.is_empty());
    assert!(config.colors.background.is_none());
}

#[test]
fn test_parse_toml() {
    let toml_str = r##"
[theme]
name = "Test Theme"
author = "Test Author"

[colors]
background = "#1a1a2e"
foreground = "#eaeaea"

[curves.lightness]
type = "sigmoid"
strength = 2.0

[contrast]
target = 75.0
extended = 60.0
"##;

    let config: ThemeConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(config.theme.name, "Test Theme");
    assert_eq!(config.colors.background, Some("#1a1a2e".to_string()));
}

#[test]
fn test_hue_overrides_roundtrip() {
    let overrides = HueOverrides {
        base08: Some(25.0),
        base0d: Some(220.0),
        ..Default::default()
    };

    let array = overrides.to_array();
    let restored = HueOverrides::from_array(array);

    assert_eq!(restored.base08, Some(25.0));
    assert_eq!(restored.base0d, Some(220.0));
    assert_eq!(restored.base09, None);
}
