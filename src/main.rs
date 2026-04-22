mod cli;
mod config;
mod error;
mod fs_utils;
mod link_ops;
mod path_resolver;
mod workspace;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Commands};
use config::{AppConfig, Config};
use fs_utils::FsUtils;
use link_ops::{LinkRequest, LinkType, OnExists};
use path_resolver::PathResolver;
use spinners::{Spinner, Spinners};
use workspace::Workspace;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init { path, force } => {
            let workspace_path = match path {
                Some(p) => std::path::PathBuf::from(p),
                None => {
                    let config_path = Workspace::config_path()?;
                    if config_path.exists() && !*force {
                        anyhow::bail!("Config already exists. Use --force to reinitialize.");
                    }
                    std::path::PathBuf::from("D:/link-disk-workspace")
                }
            };

            if cli.verbose {
                println!("Initializing workspace at: {:?}", workspace_path);
            }

            Workspace::init(&workspace_path).context("Failed to initialize workspace")?;

            println!("Workspace initialized at: {:?}", workspace_path);
            println!("Config file: {:?}", Workspace::config_path()?);
        }

        Commands::Link { apps, all, dry_run, force } => {
            let config = load_config(&cli.config)?;

            let workspace_path = &config.workspace.path;
            if !workspace_path.exists() {
                if cli.verbose {
                    println!("Creating workspace directory: {:?}", workspace_path);
                }
                FsUtils::ensure_parent_exists(workspace_path)?;
            }

            let apps_to_link = resolve_apps(&config, apps, *all)?;

            if apps_to_link.is_empty() {
                println!("No apps to link. Configure apps in config.toml or use --all");
                return Ok(());
            }

            for app_id in apps_to_link {
                let app_config = config
                    .get_app(app_id)
                    .context("App not found in config")?;

                if cli.verbose {
                    println!("\nLinking app: {}", app_config.name);
                }

                link_app(&config, app_id, app_config, *dry_run, *force, cli.verbose)?;
            }
        }

        Commands::Unlink {
            apps,
            all,
            force,
            keep_files,
        } => {
            if !*force {
                println!("This will remove links and move files back. Use --force to confirm.");
                return Ok(());
            }

            let config = load_config(&cli.config)?;
            let apps_to_unlink = resolve_apps(&config, apps, *all)?;

            for app_id in apps_to_unlink {
                let app_config = config
                    .get_app(app_id)
                    .context("App not found in config")?;

                if cli.verbose {
                    println!("\nUnlinking app: {}", app_config.name);
                }

                unlink_app(&config, app_id, app_config, *keep_files, cli.verbose)?;
            }
        }

        Commands::List { app } => {
            let config = load_config(&cli.config)?;

            match app {
                Some(app_id) => {
                    if let Some(app_config) = config.get_app(app_id) {
                        print_app_links(app_config);
                    } else {
                        println!("App not found: {}", app_id);
                    }
                }
                None => {
                    for (_, app_config) in config.enabled_apps() {
                        print_app_links(app_config);
                        println!();
                    }
                }
            }
        }

        Commands::Status { apps } => {
            let config = load_config(&cli.config)?;
            let apps_to_check: Vec<&AppConfig> = if apps.is_empty() {
                config.enabled_apps().into_iter().map(|(_, c)| c).collect()
            } else {
                apps.iter().filter_map(|a| config.get_app(a)).collect()
            };

            for app_config in apps_to_check {
                check_app_status(&config, app_config);
            }
        }

        Commands::Repair { apps, force } => {
            let config = load_config(&cli.config)?;
            let apps_to_repair: Vec<String> = if apps.is_empty() {
                config
                    .enabled_apps()
                    .into_iter()
                    .map(|(n, _)| n.clone())
                    .collect()
            } else {
                apps.clone()
            };

            for app_id in &apps_to_repair {
                if let Some(app_config) = config.get_app(app_id) {
                    repair_app(&config, app_config, *force, cli.verbose)?;
                }
            }
        }
    }

    Ok(())
}

fn load_config(config_path: &Option<String>) -> Result<Config> {
    let path = match config_path {
        Some(p) => Workspace::expand_path(p),
        None => Workspace::config_path()?,
    };

    if !path.exists() {
        anyhow::bail!(
            "Config file not found: {:?}. Run 'link-disk init' first.",
            path
        );
    }

    Config::load(&path)
}

fn resolve_apps<'a>(config: &'a Config, apps: &'a [String], all: bool) -> Result<Vec<&'a String>> {
    if all || apps.is_empty() {
        Ok(config.enabled_apps().into_iter().map(|(n, _)| n).collect())
    } else {
        Ok(apps.iter().collect())
    }
}

fn link_app(
    config: &Config,
    app_id: &str,
    app_config: &AppConfig,
    dry_run: bool,
    force: bool,
    verbose: bool,
) -> Result<()> {
    let workspace_path = &config.workspace.path;

    for source in &app_config.sources {
        let source_path_str = PathResolver::expand(&source.source);
        let target_relative = format!("{}/{}", app_config.name, source.target);
        let target_path = Workspace::resolve_target(workspace_path, &target_relative);

        if verbose {
            println!("  Source: {:?}", source_path_str);
            println!("  Target: {:?}", target_path);
        }

        if dry_run {
            println!(
                "  [DRY RUN] Would link {:?} -> {:?}",
                source_path_str, target_path
            );
            continue;
        }

        let source_path = PathResolver::resolve_if_exists(&source.source)
            .unwrap_or_else(|| PathResolver::expand(&source.source).into());

        let request = LinkRequest {
            source: source_path.clone(),
            target: target_path.clone(),
            link_type: LinkType::from_str(&source.link_type),
            on_exists: OnExists::from_str(app_config.on_exists_strategy()),
            force,
        };

        let source_name = source
            .source
            .split('/')
            .next_back()
            .unwrap_or(&source.source);
        let display_name = &app_config.name;
        let mut sp = Spinner::new(Spinners::Dots12, format!("  Linking {}...", display_name));

        match link_ops::LinkOps::link(&request, verbose) {
            Ok(_) => {
                sp.stop();
                println!("  ✓ Linked: {} ({})", display_name, source_name);
            }
            Err(e) => {
                sp.stop();
                anyhow::bail!("Failed to link {}:{} - {}", app_id, source_path_str, e);
            }
        }
    }

    Ok(())
}

fn unlink_app(
    config: &Config,
    app_id: &str,
    app_config: &AppConfig,
    keep_files: bool,
    verbose: bool,
) -> Result<()> {
    let workspace_path = &config.workspace.path;

    for source in &app_config.sources {
        let source_path = PathResolver::resolve_if_exists(&source.source)
            .unwrap_or_else(|| PathResolver::expand(&source.source).into());

        let target_relative = format!("{}/{}", app_config.name, source.target);
        let target_path = Workspace::resolve_target(workspace_path, &target_relative);

        if verbose {
            println!("  Source: {:?}", source_path);
            println!("  Target: {:?}", target_path);
        }

        link_ops::LinkOps::unlink(&source_path, &target_path, keep_files, verbose)
            .with_context(|| format!("Failed to unlink {}:{}", app_id, source.source))?;
    }

    Ok(())
}

fn print_app_links(app_config: &AppConfig) {
    println!("App: {}", app_config.name);

    for source in &app_config.sources {
        println!("  {} -> {}", source.source, source.target);
    }
}

fn check_app_status(_config: &Config, app_config: &AppConfig) {
    println!("App: {}", app_config.name);

    for source in &app_config.sources {
        let source_path: std::path::PathBuf = PathResolver::expand(&source.source).into();
        let status = link_ops::LinkOps::check_status(
            &source_path,
            &std::path::PathBuf::from(&source.target),
        );

        let status_icon = match status {
            "linked" => "✓",
            "broken" => "✗",
            _ => "?",
        };

        println!("  {} {} -> {}", status_icon, source.source, status);
    }
}

fn repair_app(config: &Config, app_config: &AppConfig, force: bool, verbose: bool) -> Result<()> {
    let workspace_path = &config.workspace.path;

    for source in &app_config.sources {
        let source_path: std::path::PathBuf = PathResolver::expand(&source.source).into();
        let target_relative = format!("{}/{}", app_config.name, source.target);
        let target_path = Workspace::resolve_target(workspace_path, &target_relative);
        let status = link_ops::LinkOps::check_status(&source_path, &target_path);

        match status {
            "broken" => {
                if verbose {
                    println!("  Repairing broken link: {}", source.source);
                }

                FsUtils::remove_if_exists(&source_path, verbose)?;

                link_ops::LinkOps::unlink(&source_path, &target_path, false, verbose)?;
            }
            "target_only" => {
                if force {
                    if verbose {
                        println!("  Creating link for orphaned target: {}", source.source);
                    }

                    FsUtils::create_symlink(&target_path, &source_path)?;
                } else {
                    println!(
                        "  Target exists without link. Use --force to create link: {}",
                        source.source
                    );
                }
            }
            _ => {
                if verbose {
                    println!("  Skipping {} (status: {})", source.source, status);
                }
            }
        }
    }

    Ok(())
}
