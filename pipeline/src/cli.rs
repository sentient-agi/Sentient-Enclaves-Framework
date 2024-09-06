use log::error;

pub trait ExitGracefully<T, E> {
    fn ok_or_exit(self, message: &str) -> T;
}

impl<T, E: std::fmt::Debug> ExitGracefully<T, E> for Result<T, E> {
    fn ok_or_exit(self, message: &str) -> T {
        match self {
            Ok(val) => val,
            Err(err) => {
                error!("{:?}: {}", err, message);
                std::process::exit(1);
            }
        }
    }
}

#[macro_export]
macro_rules! create_app {
    () => {
        App::new("Pipeline Vsock Communication Protocol")
            .about("Pipeline vsock secure local channel communication protocol that provides remote control of enclave via running commands inside the enclave and provides bidirectional files transmission into/from encalve's FS.")
            .setting(AppSettings::ArgRequiredElseHelp)
            .version(env!("CARGO_PKG_VERSION"))
            .arg(
                Arg::with_name("config")
                .short('c')
                .long("config")
                .help("Configuration settings")
                .takes_value(true)
                .required(false),
            )
            .subcommand(
                SubCommand::with_name("listen")
                    .about("Listen on a given port")
                    .arg(
                        Arg::with_name("port")
                            .long("port")
                            .help("port")
                            .takes_value(true)
                            .required(true),
                    ),
            )
            .subcommand(
                SubCommand::with_name("run")
                    .about("Run a command inside the enclave")
                    .arg(
                        Arg::with_name("port")
                            .long("port")
                            .help("port")
                            .takes_value(true)
                            .required(true),
                    )
                    .arg(
                        Arg::with_name("cid")
                            .long("cid")
                            .help("cid")
                            .takes_value(true)
                            .required(true),
                    )
                    .arg(
                        Arg::with_name("command")
                            .long("command")
                            .help("command")
                            .takes_value(true)
                            .required(true),
                    )
                    .arg(
                        Arg::with_name("no-wait")
                            .long("no-wait")
                            .help("command execution pipeline won't wait the command's result returning")
                            .takes_value(false),
                    ),
            )
            .subcommand(
                SubCommand::with_name("recv-file")
                    .about("Receive a file from the enclave")
                    .arg(
                        Arg::with_name("port")
                            .long("port")
                            .help("port")
                            .takes_value(true)
                            .required(true),
                    )
                    .arg(
                        Arg::with_name("cid")
                            .long("cid")
                            .help("cid")
                            .takes_value(true)
                            .required(true),
                    )
                    .arg(
                        Arg::with_name("localpath")
                            .long("localpath")
                            .help("localpath")
                            .takes_value(true)
                            .required(true),
                    )
                    .arg(
                        Arg::with_name("remotepath")
                            .long("remotepath")
                            .help("remotepath")
                            .takes_value(true)
                            .required(true),
                    ),
            )
            .subcommand(
                SubCommand::with_name("send-file")
                    .about("Send a file to the enclave")
                    .arg(
                        Arg::with_name("port")
                            .long("port")
                            .help("port")
                            .takes_value(true)
                            .required(true),
                    )
                    .arg(
                        Arg::with_name("cid")
                            .long("cid")
                            .help("cid")
                            .takes_value(true)
                            .required(true),
                    )
                    .arg(
                        Arg::with_name("localpath")
                            .long("localpath")
                            .help("localpath")
                            .takes_value(true)
                            .required(true),
                    )
                    .arg(
                        Arg::with_name("remotepath")
                            .long("remotepath")
                            .help("remotepath")
                            .takes_value(true)
                            .required(true),
                    ),
            )
            .arg(
                Arg::with_name("=(^\">,.•.,<\"^)=")
                .short('g')
                .long("george")
                .help("Happy Easter from George the BSH cat!")
                .takes_value(false)
                .required(false),
            )
            .arg(
                Arg::with_name("=(^\",..,\"^)=")
                .short('p')
                .long("pascal")
                .help("Happy Easter from Pascal the BSH cat!")
                .takes_value(false)
                .required(false),
            )
    };
}
