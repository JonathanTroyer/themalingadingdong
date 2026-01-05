//! CLI entry point for themalingadingdong.

use std::path::Path;

use clap::Parser;
use color_eyre::eyre::{Result, WrapErr, bail, eyre};
use tinted_builder::SchemeVariant;

use themalingadingdong::cli::{Cli, VariantArg};
use themalingadingdong::generate::{GenerateConfig, generate_for_variant, parse_color};
use themalingadingdong::tui;
use themalingadingdong::validation::validate_with_warnings;

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    // Launch TUI if --interactive flag is set
    if cli.interactive {
        return tui::run(&cli);
    }

    // Parse input colors (required in non-interactive mode)
    let bg_str = cli
        .background
        .as_ref()
        .ok_or_else(|| eyre!("Background color is required"))?;
    let fg_str = cli
        .foreground
        .as_ref()
        .ok_or_else(|| eyre!("Foreground color is required"))?;
    let name_str = cli
        .name
        .as_ref()
        .ok_or_else(|| eyre!("Scheme name is required"))?;

    let background =
        parse_color(bg_str).map_err(|e| eyre!("Invalid background color '{}': {}", bg_str, e))?;

    let foreground =
        parse_color(fg_str).map_err(|e| eyre!("Invalid foreground color '{}': {}", fg_str, e))?;

    // Create generation config
    let config = GenerateConfig {
        background,
        foreground,
        hue_overrides: cli.hue_overrides(),
        min_contrast: cli.min_contrast,
        extended_min_contrast: cli.extended_min_contrast,
        max_lightness_adjustment: cli.max_lightness_adjustment,
        accent_colorfulness: cli.accent_colorfulness,
        extended_colorfulness: cli.extended_colorfulness,
        name: name_str.clone(),
        author: cli.author.clone(),
        interpolation: cli.interpolation_config(),
    };

    // Determine which variants to generate
    let variants_to_generate: Vec<Option<SchemeVariant>> = match cli.variant {
        VariantArg::Dark => vec![Some(SchemeVariant::Dark)],
        VariantArg::Light => vec![Some(SchemeVariant::Light)],
        VariantArg::Both => {
            if cli.output.is_none() {
                bail!("--variant both requires --output to specify base filename");
            }
            vec![Some(SchemeVariant::Dark), Some(SchemeVariant::Light)]
        }
        VariantArg::Auto => vec![None],
    };

    for forced_variant in variants_to_generate {
        // Generate the scheme
        let result = generate_for_variant(&config, forced_variant);
        let scheme = result.scheme;

        // Display generation warnings (hues that couldn't achieve target contrast)
        if !result.warnings.is_empty() {
            eprintln!("Generation warnings:");
            for warning in &result.warnings {
                eprintln!("  {warning}");
            }
        }

        // Validate
        let warnings = validate_with_warnings(&scheme);
        if !warnings.is_empty() {
            if cli.no_adjust {
                eprintln!("Validation failed for the following color pairs:");
                for warning in &warnings {
                    eprintln!("  {warning}");
                }
                bail!("Validation failed");
            }
            for warning in &warnings {
                eprintln!("Warning: {warning}");
            }
        }

        // Serialize to YAML using serde
        let yaml = serde_yaml::to_string(&scheme).wrap_err("Failed to serialize scheme to YAML")?;

        // Output
        if let Some(ref base_path) = cli.output {
            let output_path = if matches!(cli.variant, VariantArg::Both) {
                variant_filename(base_path, &scheme.variant)
            } else {
                base_path.clone()
            };

            std::fs::write(&output_path, &yaml)
                .wrap_err_with(|| format!("Failed to write to {}", output_path.display()))?;
            eprintln!("Wrote scheme to {}", output_path.display());
        } else {
            print!("{yaml}");
        }
    }

    Ok(())
}

/// Generate output filename with variant suffix.
fn variant_filename(base_path: &Path, variant: &SchemeVariant) -> std::path::PathBuf {
    let suffix = match variant {
        SchemeVariant::Dark => "-dark",
        SchemeVariant::Light => "-light",
        v => unimplemented!("unsupported variant: {v:?}"),
    };

    let stem = base_path.file_stem().unwrap_or_default().to_string_lossy();
    let ext = base_path.extension().unwrap_or_default().to_string_lossy();
    let parent = base_path.parent().unwrap_or(Path::new(""));

    if ext.is_empty() {
        parent.join(format!("{stem}{suffix}.yaml"))
    } else {
        parent.join(format!("{stem}{suffix}.{ext}"))
    }
}
