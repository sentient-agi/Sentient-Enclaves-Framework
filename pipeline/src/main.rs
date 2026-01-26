use std::process::exit;

use anyhow::{Context, Result};
use clap::{App, AppSettings, Arg, SubCommand};
use tracing::{debug, error, info};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use pipeline::cats::{GEORGE, PASCAL};
use pipeline::cli_parser::{CommandOutput, DirArgs, FileArgs, ListenArgs, RunArgs};
use pipeline::config::{self, AppConfig};
use pipeline::create_app;
use pipeline::{listen, recv_dir, recv_file, run, send_dir, send_file};

fn init_tracing() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();
}

fn main() -> Result<()> {
    init_tracing();

    debug!("Starting pipeline application");

    let app = create_app!();
    let args = app.get_matches();

    if args.contains_id("=(^\">,.•.,<\"^)=") {
        let output: CommandOutput = CommandOutput::new(String::from(GEORGE), String::new(), 0);
        info!(cat = "george", "Displaying George the cat");
        // Still print to stdout for the cat ASCII art
        print!("{}", output.stdout);
        exit(output.rc.unwrap_or_default());
    };

    if args.contains_id("=(^\",..,\"^)=") {
        let output: CommandOutput = CommandOutput::new(String::from(PASCAL), String::new(), 0);
        info!(cat = "pascal", "Displaying Pascal the cat");
        // Still print to stdout for the cat ASCII art
        print!("{}", output.stdout);
        exit(output.rc.unwrap_or_default());
    };

    let default_config_path = format!("./.config/{}.config.yaml", env!("CARGO_CRATE_NAME"));
    let config_path: &String = args.get_one("config").unwrap_or(&default_config_path);

    debug!(config_path = %config_path, "Loading configuration");

    // Load and set the runtime configuration
    let app_config: AppConfig = config::load_config_from_path(config_path)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Set the runtime config so that buffer sizes are available globally
    config::set_runtime_config(app_config.clone());

    info!(
        cid = app_config.cid,
        port = app_config.port,
        buf_file_io = app_config.buffers.buf_max_len_file_io,
        buf_file_path = app_config.buffers.buf_max_len_file_path,
        buf_cmd = app_config.buffers.buf_max_len_cmd,
        buf_cmd_io = app_config.buffers.buf_max_len_cmd_io,
        backlog = app_config.buffers.backlog,
        max_conn_attempts = app_config.buffers.max_connection_attempts,
        "Configuration loaded and applied"
    );

    match args.subcommand() {
        Some(("listen", sub_args)) => {
            debug!("Processing 'listen' subcommand");
            let listen_args = ListenArgs::new_with(sub_args)
                .map_err(|e| anyhow::anyhow!("Failed to parse listen arguments: {}", e))?;
            listen(listen_args, app_config)
                .map_err(|e| anyhow::anyhow!("Listen failed: {}", e))?;
        }
        Some(("run", sub_args)) => {
            debug!("Processing 'run' subcommand");
            let run_args = RunArgs::new_with(sub_args)
                .map_err(|e| anyhow::anyhow!("Failed to parse run arguments: {}", e))?;
            let rc = run(run_args, app_config)
                .map_err(|e| anyhow::anyhow!("Run failed: {}", e))?;
            info!(return_code = rc, "Command execution completed");
            std::process::exit(rc);
        }
        Some(("send-file", sub_args)) => {
            debug!("Processing 'send-file' subcommand");
            let subcmd_args = FileArgs::new_with(sub_args)
                .map_err(|e| anyhow::anyhow!("Failed to parse send-file arguments: {}", e))?;
            send_file(subcmd_args, app_config)
                .map_err(|e| anyhow::anyhow!("Send file failed: {}", e))?;
            info!("Send file completed successfully");
        }
        Some(("recv-file", sub_args)) => {
            debug!("Processing 'recv-file' subcommand");
            let subcmd_args = FileArgs::new_with(sub_args)
                .map_err(|e| anyhow::anyhow!("Failed to parse recv-file arguments: {}", e))?;
            recv_file(subcmd_args, app_config)
                .map_err(|e| anyhow::anyhow!("Receive file failed: {}", e))?;
            info!("Receive file completed successfully");
        }
        Some(("send-dir", sub_args)) => {
            debug!("Processing 'send-dir' subcommand");
            let subcmd_args = DirArgs::new_with(sub_args)
                .map_err(|e| anyhow::anyhow!("Failed to parse send-dir arguments: {}", e))?;
            send_dir(subcmd_args, app_config)
                .map_err(|e| anyhow::anyhow!("Send directory failed: {}", e))?;
            info!("Send directory completed successfully");
        }
        Some(("recv-dir", sub_args)) => {
            debug!("Processing 'recv-dir' subcommand");
            let subcmd_args = DirArgs::new_with(sub_args)
                .map_err(|e| anyhow::anyhow!("Failed to parse recv-dir arguments: {}", e))?;
            recv_dir(subcmd_args, app_config)
                .map_err(|e| anyhow::anyhow!("Receive directory failed: {}", e))?;
            info!("Receive directory completed successfully");
        }
        Some((cmd, _)) => {
            error!(command = %cmd, "Unknown subcommand");
        }
        None => {
            debug!("No subcommand provided");
        }
    }

    Ok(())
}
