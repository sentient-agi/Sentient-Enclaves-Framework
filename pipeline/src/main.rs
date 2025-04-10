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

    let raw_config_string = std::fs::read_to_string(config_path).expect(format!("Missing '{}' configuration file.", config_path).as_str());
    let app_config: AppConfig = toml::from_str(raw_config_string.as_str()).expect(format!("Failed to parse '{}' configuration file.", config_path).as_str());

    match args.subcommand() {
        Some(("listen", args)) => {
            let listen_args = ListenArgs::new_with(args).unwrap();
            listen(listen_args, app_config).unwrap();
        }
        Some(("run", args)) => {
            let run_args = RunArgs::new_with(args).unwrap();
            let rc = run(run_args, app_config).unwrap();
            std::process::exit(rc);
        }
        Some(("send-file", args)) => {
            let subcmd_args = FileArgs::new_with(args).unwrap();
            send_file(subcmd_args, app_config).unwrap();
        }
        Some(("recv-file", args)) => {
            let subcmd_args = FileArgs::new_with(args).unwrap();
            recv_file(subcmd_args, app_config).unwrap();
        }
        Some(_) | None => {}
    }
}
