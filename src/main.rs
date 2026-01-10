//! CLI entry point for themalingadingdong.

use std::path::Path;

use clap::{CommandFactory, Parser};
use color_eyre::eyre::{Result, WrapErr, bail, eyre};
use tinted_builder::SchemeVariant;
use tracing::{info, warn};

use themalingadingdong::cli::{Cli, OutputFormat, VariantArg};
use themalingadingdong::config::{load_config, validate_config};
use themalingadingdong::generate::generate_for_variant;
use themalingadingdong::import::import_scheme;
use themalingadingdong::logging::init_logging;
use themalingadingdong::tui;
use themalingadingdong::validation::{validate, validate_with_warnings};

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    // Handle shell completions early (before logging setup)
    if let Some(shell) = cli.completions {
        clap_complete::generate(
            shell,
            &mut Cli::command(),
            "themalingadingdong",
            &mut std::io::stdout(),
        );
        return Ok(());
    }

    let _log_guard = init_logging(cli.log_file.as_deref(), Some(&cli.log_level));

    info!(version = env!("CARGO_PKG_VERSION"), "started");

    // Launch TUI only if --interactive is specified
    if cli.interactive {
        return tui::run(&cli);
    }

    // Handle --input without TUI: validate and output the imported scheme
    if let Some(ref input_path) = cli.input {
        let import_result = import_scheme(input_path)
            .wrap_err_with(|| format!("Failed to import {}", input_path.display()))?;

        let scheme = &import_result.scheme;
        let results = validate(scheme);

        // Print validation results
        eprintln!("Imported: {} by {}", scheme.name, scheme.author);
        eprintln!("Variant: {:?}", scheme.variant);
        eprintln!();

        // Print required validations
        eprintln!("Required contrast checks:");
        for result in &results.required {
            let status = if result.passes { "PASS" } else { "FAIL" };
            eprintln!(
                "  {} on {}: Lc {:5.1} (min {:5.1}) [{}]",
                result.pair.foreground,
                result.pair.background,
                result.contrast.abs(),
                result.pair.threshold.min_lc,
                status
            );
        }

        // Print reference validations (informational)
        if !results.reference.is_empty() {
            eprintln!();
            eprintln!("Reference contrast (informational):");
            for result in &results.reference {
                let status = if result.passes { "pass" } else { "low" };
                eprintln!(
                    "  {} on {}: Lc {:5.1} ({})",
                    result.pair.foreground,
                    result.pair.background,
                    result.contrast.abs(),
                    status
                );
            }
        }

        let pass_count = results.required.iter().filter(|r| r.passes).count();
        let total = results.required.len();
        eprintln!();
        eprintln!("Summary: {}/{} required checks pass", pass_count, total);

        // Output the scheme in requested format (unless --dry-run)
        if !cli.dry_run {
            let output_content = match cli.format {
                OutputFormat::Yaml => {
                    serde_yaml::to_string(scheme).wrap_err("Failed to serialize scheme to YAML")?
                }
                OutputFormat::Json => serde_json::to_string_pretty(scheme)
                    .wrap_err("Failed to serialize scheme to JSON")?,
            };

            if let Some(ref output_path) = cli.output {
                std::fs::write(output_path, &output_content)
                    .wrap_err_with(|| format!("Failed to write to {}", output_path.display()))?;
                eprintln!("Wrote scheme to {}", output_path.display());
            } else {
                print!("{output_content}");
            }
        }

        return Ok(());
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

        // Handle --dry-run: show validation results without generating output
        if cli.dry_run {
            let status = if warnings.is_empty() {
                "passed"
            } else {
                "passed with warnings"
            };
            eprintln!("Dry run: validation {} for '{}'", status, scheme.name);
            for warning in &warnings {
                eprintln!("  - {}", warning);
            }
            continue;
        }

        let output_content = match cli.format {
            OutputFormat::Yaml => {
                serde_yaml::to_string(&scheme).wrap_err("Failed to serialize scheme to YAML")?
            }
            OutputFormat::Json => serde_json::to_string_pretty(&scheme)
                .wrap_err("Failed to serialize scheme to JSON")?,
        };

        if let Some(ref base_path) = cli.output {
            let output_path = if matches!(cli.variant, VariantArg::Both) {
                variant_filename(base_path, &scheme.variant, cli.format)
            } else {
                base_path.clone()
            };

            info!(path = %output_path.display(), "wrote scheme");
            std::fs::write(&output_path, &output_content)
                .wrap_err_with(|| format!("Failed to write to {}", output_path.display()))?;
            eprintln!("Wrote scheme to {}", output_path.display());
        } else {
            print!("{output_content}");
        }
    }

    Ok(())
}

/// Generate output filename with variant suffix and format extension.
fn variant_filename(
    base_path: &Path,
    variant: &SchemeVariant,
    format: OutputFormat,
) -> std::path::PathBuf {
    let suffix = match variant {
        SchemeVariant::Dark => "-dark",
        SchemeVariant::Light => "-light",
        v => unreachable!("unsupported variant: {v:?}"),
    };

    let ext = match format {
        OutputFormat::Yaml => "yaml",
        OutputFormat::Json => "json",
    };

    let stem = base_path.file_stem().unwrap_or_default().to_string_lossy();
    let parent = base_path.parent().unwrap_or(Path::new(""));

    parent.join(format!("{stem}{suffix}.{ext}"))
}
