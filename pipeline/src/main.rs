use pipeline::cli_parser::{FileArgs, ListenArgs, RunArgs};
use pipeline::create_app;
use pipeline::config::AppConfig;
use pipeline::{listen, recv_file, run, send_file};

use clap::{App, AppSettings, Arg, SubCommand};

fn main() {
    let app = create_app!();
    let args = app.get_matches();

    let default_config_path = "./.config/config.toml".to_string();
    let config_path = args
        .get_one("config")
        .unwrap_or(&default_config_path);

    let raw_config_string = std::fs::read_to_string(config_path).expect("Missing `config.toml` file.");
    let app_config: AppConfig = toml::from_str(raw_config_string.as_str()).expect("Failed to parse `config.toml` file.");

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
