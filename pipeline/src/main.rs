use pipeline::cli_parser::{FileArgs, ListenArgs, RunArgs};
use pipeline::create_app;
use pipeline::{listen, recv_file, run, send_file};

use clap::{App, AppSettings, Arg, SubCommand};

fn main() {
    let app = create_app!();
    let args = app.get_matches();

    match args.subcommand() {
        Some(("listen", args)) => {
            let listen_args = ListenArgs::new_with(args).unwrap();
            listen(listen_args).unwrap();
        }
        Some(("run", args)) => {
            let run_args = RunArgs::new_with(args).unwrap();
            let rc = run(run_args).unwrap();
            std::process::exit(rc);
        }
        Some(("recv-file", args)) => {
            let subcmd_args = FileArgs::new_with(args).unwrap();
            recv_file(subcmd_args).unwrap();
        }
        Some(("send-file", args)) => {
            let subcmd_args = FileArgs::new_with(args).unwrap();
            send_file(subcmd_args).unwrap();
        }
        Some(_) | None => {}
    }
}
