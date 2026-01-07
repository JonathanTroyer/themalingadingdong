//! CLI entry point for themalingadingdong.

use std::path::Path;

use clap::Parser;
use color_eyre::eyre::{Result, WrapErr, bail, eyre};
use tinted_builder::SchemeVariant;
use tracing::{info, warn};

use themalingadingdong::cli::{Cli, VariantArg};
use themalingadingdong::config::{load_config, validate_config};
use themalingadingdong::generate::generate_for_variant;
use themalingadingdong::logging::init_logging;
use themalingadingdong::tui;
use themalingadingdong::validation::validate_with_warnings;

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    let _log_guard = init_logging(cli.log_file.as_deref(), Some(&cli.log_level));

    info!(version = env!("CARGO_PKG_VERSION"), "started");

    if cli.interactive {
        return tui::run(&cli);
    }

    // Load configuration with Figment layering: defaults < TOML file < CLI args
    let theme_config = load_config(cli.config.as_deref(), &cli.to_config_overrides())
        .map_err(|e| eyre!("Configuration error: {}", e))?;

    // Validate required fields
    validate_config(&theme_config).map_err(|e| eyre!("{}", e))?;

    // Handle --save-config if specified
    if let Some(ref save_path) = cli.save_config {
        theme_config
            .save(save_path)
            .map_err(|e| eyre!("Failed to save config: {}", e))?;
        eprintln!("Saved configuration to {}", save_path.display());
    }

    // Convert to GenerateConfig
    let config = theme_config
        .to_generate_config()
        .map_err(|e| eyre!("Invalid configuration: {}", e))?;

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
        let result = generate_for_variant(&config, forced_variant);
        let scheme = result.scheme;

        if !result.warnings.is_empty() {
            eprintln!("Generation warnings:");
            for warning in &result.warnings {
                warn!(warning = %warning, "generation warning");
                eprintln!("  {warning}");
            }
        }

        let warnings = validate_with_warnings(&scheme);
        if !warnings.is_empty() {
            if cli.no_adjust {
                eprintln!("Validation failed for the following color pairs:");
                for warning in &warnings {
                    warn!(warning = %warning, "validation failure");
                    eprintln!("  {warning}");
                }
                bail!("Validation failed");
            }
            for warning in &warnings {
                warn!(warning = %warning, "validation warning");
                eprintln!("Warning: {warning}");
            }
        }

        let yaml = serde_yaml::to_string(&scheme).wrap_err("Failed to serialize scheme to YAML")?;

        if let Some(ref base_path) = cli.output {
            let output_path = if matches!(cli.variant, VariantArg::Both) {
                variant_filename(base_path, &scheme.variant)
            } else {
                base_path.clone()
            };

            info!(path = %output_path.display(), "wrote scheme");
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
