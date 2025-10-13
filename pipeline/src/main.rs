use std::process::exit;
use pipeline::cli_parser::{CommandOutput, FileArgs, ListenArgs, RunArgs};
use pipeline::create_app;
use pipeline::config::AppConfig;
use pipeline::{listen, recv_file, run, send_file};
use pipeline::cats::{GEORGE, PASCAL};

use clap::{App, AppSettings, Arg, SubCommand};

fn main() {
    let app = create_app!();
    let args = app.get_matches();

    if args.contains_id("=(^\">,.•.,<\"^)=") {
        let output: CommandOutput = CommandOutput::new(String::from(GEORGE), String::new(), 0);
        print!("{}", output.stdout);
        eprint!("{}", output.stderr);
        exit(output.rc.unwrap_or_default());
    };
    if args.contains_id("=(^\",..,\"^)=") {
        let output: CommandOutput = CommandOutput::new(String::from(PASCAL), String::new(), 0);
        print!("{}", output.stdout);
        eprint!("{}", output.stderr);
        exit(output.rc.unwrap_or_default());
    };

    let default_config_path = format!("./.config/{}.config.toml", env!("CARGO_CRATE_NAME"));
    let config_path = args
        .get_one("config")
        .unwrap_or(&default_config_path);

    let raw_config_string = match std::fs::read_to_string(config_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read config file '{}': {}", config_path, e);
            exit(1);
        }
    };

    let app_config: AppConfig = match toml::from_str(raw_config_string.as_str()) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to parse config file '{}': {}", config_path, e);
            exit(1);
        }
    };

    match args.subcommand() {
        Some(("listen", args)) => {
            let listen_args = match ListenArgs::new_with(args) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("Invalid listen arguments: {}", e);
                    exit(1);
                }
            };
            if let Err(e) = listen(listen_args, app_config) {
                eprintln!("Listen error: {}", e);
                exit(1);
            }
        }
        Some(("run", args)) => {
            let run_args = match RunArgs::new_with(args) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("Invalid run arguments: {}", e);
                    exit(1);
                }
            };
            let rc = match run(run_args, app_config) {
                Ok(code) => code,
                Err(e) => {
                    eprintln!("Command execution failed: {}", e);
                    exit(1);
                }
            };
            std::process::exit(rc);
        }
        Some(("send-file", args)) => {
            let subcmd_args = match FileArgs::new_with(args) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("Invalid file arguments: {}", e);
                    exit(1);
                }
            };
            if let Err(e) = send_file(subcmd_args, app_config) {
                eprintln!("File send failed: {}", e);
                exit(1);
            }
        }
        Some(("recv-file", args)) => {
            let subcmd_args = match FileArgs::new_with(args) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("Invalid file arguments: {}", e);
                    exit(1);
                }
            };
            if let Err(e) = recv_file(subcmd_args, app_config) {
                eprintln!("File receive failed: {}", e);
                exit(1);
            }
        }
        Some(_) | None => {}
    }
}
