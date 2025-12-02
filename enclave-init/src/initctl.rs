mod protocol;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use nix::sys::socket::{connect, recv, send, socket, AddressFamily, MsgFlags, SockFlag, SockType, UnixAddr};
use nix::unistd::close;
use protocol::{Request, Response, SOCKET_PATH};

#[derive(Parser)]
#[command(name = "initctl")]
#[command(about = "Control tool for enclave init system", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all services
    List,

    /// Show status of a service
    Status {
        /// Service name
        #[arg(value_name = "SERVICE")]
        name: String,
    },

    /// Start a service
    Start {
        /// Service name
        #[arg(value_name = "SERVICE")]
        name: String,
    },

    /// Stop a service
    Stop {
        /// Service name
        #[arg(value_name = "SERVICE")]
        name: String,
    },

    /// Restart a service
    Restart {
        /// Service name
        #[arg(value_name = "SERVICE")]
        name: String,
    },

    /// Show logs of a service
    Logs {
        /// Service name
        #[arg(value_name = "SERVICE")]
        name: String,

        /// Number of lines to show
        #[arg(short = 'n', long, default_value = "50")]
        lines: usize,
    },

    /// Reboot the system
    Reboot,

    /// Shutdown the system
    Shutdown,

    /// Ping the init system
    Ping,
}

fn send_request(request: Request) -> Result<Response> {
    let socket_fd = socket(
        AddressFamily::Unix,
        SockType::Stream,
        SockFlag::empty(),
        None,
    )
    .context("Failed to create socket")?;

    let addr = UnixAddr::new(SOCKET_PATH).context("Failed to create socket address")?;

    connect(socket_fd, &addr).context("Failed to connect to init socket")?;

    let request_data = serde_json::to_vec(&request).context("Failed to serialize request")?;

    send(socket_fd, &request_data, MsgFlags::empty())
        .context("Failed to send request")?;

    let mut buffer = vec![0u8; 65536];
    let n = recv(socket_fd, &mut buffer, MsgFlags::empty())
        .context("Failed to receive response")?;

    buffer.truncate(n);

    let response: Response = serde_json::from_slice(&buffer)
        .context("Failed to deserialize response")?;

    close(socket_fd).context("Failed to close socket")?;

    Ok(response)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let request = match cli.command {
        Commands::List => Request::ListServices,
        Commands::Status { name } => Request::ServiceStatus { name },
        Commands::Start { name } => Request::ServiceStart { name },
        Commands::Stop { name } => Request::ServiceStop { name },
        Commands::Restart { name } => Request::ServiceRestart { name },
        Commands::Logs { name, lines } => Request::ServiceLogs { name, lines },
        Commands::Reboot => Request::SystemReboot,
        Commands::Shutdown => Request::SystemShutdown,
        Commands::Ping => Request::Ping,
    };

    let response = send_request(request)?;

    match response {
        Response::Success { message } => {
            println!("✓ {}", message);
        }
        Response::Error { message } => {
            eprintln!("✗ Error: {}", message);
            std::process::exit(1);
        }
        Response::ServiceList { services } => {
            if services.is_empty() {
                println!("No services found");
            } else {
                println!("{:<20} {:<10} {:<15} {:<10}", "NAME", "ACTIVE", "RESTART", "RESTARTS");
                println!("{}", "-".repeat(60));
                for service in services {
                    println!(
                        "{:<20} {:<10} {:<15} {:<10}",
                        service.name,
                        if service.active { "active" } else { "inactive" },
                        service.restart_policy,
                        service.restart_count
                    );
                }
            }
        }
        Response::ServiceStatus { status } => {
            println!("Service: {}", status.name);
            println!("  Status: {}", if status.active { "active" } else { "inactive" });
            if let Some(pid) = status.pid {
                println!("  PID: {}", pid);
            }
            println!("  Command: {}", status.exec_start);
            if let Some(wd) = status.working_directory {
                println!("  Working Directory: {}", wd);
            }
            println!("  Restart Policy: {}", status.restart_policy);
            println!("  Restart Delay: {}s", status.restart_sec);
            println!("  Restart Count: {}", status.restart_count);
            if let Some(exit_code) = status.exit_status {
                println!("  Last Exit Code: {}", exit_code);
            }
        }
        Response::ServiceLogs { logs } => {
            if logs.is_empty() {
                println!("No logs available");
            } else {
                for log in logs {
                    println!("{}", log);
                }
            }
        }
        Response::Pong => {
            println!("Pong - init system is responsive");
        }
    }

    Ok(())
}
