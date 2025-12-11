mod config;
mod protocol;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use config::{ControlProtocol, InitctlConfig};
use nix::sys::socket::{connect, recv, send, socket, AddressFamily, MsgFlags, SockFlag, SockType, UnixAddr, VsockAddr};
use nix::unistd::close;
use protocol::{Request, Response};

#[derive(Parser)]
#[command(name = "initctl")]
#[command(about = "Control tool for enclave init system", long_about = None)]
struct Cli {
    /// Path to initctl configuration file
    #[arg(short = 'c', long, env = "INITCTL_CONFIG", default_value = "/etc/initctl.yaml")]
    config: String,

    /// Override protocol (unix or vsock)
    #[arg(short, long)]
    protocol: Option<String>,

    /// Override Unix socket path
    #[arg(short = 's', long, env = "INIT_SOCKET")]
    socket: Option<String>,

    /// Override VSOCK CID
    #[arg(long)]
    vsock_cid: Option<u32>,

    /// Override VSOCK port
    #[arg(long)]
    vsock_port: Option<u32>,

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

    /// Enable a service
    Enable {
        /// Service name
        #[arg(value_name = "SERVICE")]
        name: String,

        /// Start the service immediately after enabling
        #[arg(long)]
        now: bool,
    },

    /// Disable a service
    Disable {
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

    /// Clear logs of a service
    LogsClear {
        /// Service name
        #[arg(value_name = "SERVICE")]
        name: String,
    },

    /// Process management commands
    #[command(subcommand)]
    Ps(PsCommands),

    /// Show system status
    SystemStatus,

    /// Reload service configurations
    Reload,

    /// Reboot the system
    Reboot,

    /// Shutdown the system
    Shutdown,

    /// Ping the init system
    Ping,
}

#[derive(Subcommand)]
enum PsCommands {
    /// List all processes
    List,

    /// Show status of a specific process
    Status {
        /// Process ID
        #[arg(value_name = "PID")]
        pid: i32,
    },

    /// Start a new process (alias: run)
    #[command(alias = "run")]
    Start {
        /// Command to execute
        #[arg(value_name = "COMMAND")]
        command: String,

        /// Arguments for the command
        #[arg(value_name = "ARGS")]
        args: Vec<String>,

        /// Environment variables (KEY=VALUE)
        #[arg(short, long)]
        env: Vec<String>,
    },

    /// Stop a process (send SIGTERM)
    Stop {
        /// Process ID
        #[arg(value_name = "PID")]
        pid: i32,
    },

    /// Restart a process (managed services only)
    Restart {
        /// Process ID
        #[arg(value_name = "PID")]
        pid: i32,
    },

    /// Send a signal to a process
    Kill {
        /// Process ID
        #[arg(value_name = "PID")]
        pid: i32,

        /// Signal number (default: 15/SIGTERM)
        #[arg(short, long, default_value = "15")]
        signal: i32,
    },
}

fn send_request_unix(socket_path: &str, request: Request) -> Result<Response> {
    let socket_fd = socket(
        AddressFamily::Unix,
        SockType::Stream,
        SockFlag::empty(),
        None,
    )
    .context("Failed to create Unix socket")?;

    let addr = UnixAddr::new(socket_path).context("Failed to create Unix socket address")?;

    connect(socket_fd, &addr).context("Failed to connect to Unix socket")?;

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

fn send_request_vsock(cid: u32, port: u32, request: Request) -> Result<Response> {
    let socket_fd = socket(
        AddressFamily::Vsock,
        SockType::Stream,
        SockFlag::empty(),
        None,
    )
    .context("Failed to create VSOCK socket")?;

    let addr = VsockAddr::new(cid, port);

    connect(socket_fd, &addr).context("Failed to connect to VSOCK")?;

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

fn format_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, minutes, seconds)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

fn format_memory(kb: u64) -> String {
    if kb >= 1024 * 1024 {
        format!("{:.1}G", kb as f64 / (1024.0 * 1024.0))
    } else if kb >= 1024 {
        format!("{:.1}M", kb as f64 / 1024.0)
    } else {
        format!("{}K", kb)
    }
}

fn format_state(state: &str) -> &str {
    match state {
        "R" => "Running",
        "S" => "Sleeping",
        "D" => "Disk sleep",
        "Z" => "Zombie",
        "T" => "Stopped",
        "t" => "Tracing",
        "X" => "Dead",
        "x" => "Dead",
        "K" => "Wakekill",
        "W" => "Waking",
        "P" => "Parked",
        "I" => "Idle",
        _ => state,
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load configuration
    let mut config = InitctlConfig::load_from(&cli.config).unwrap_or_default();

    // Apply CLI overrides
    if let Some(protocol_str) = &cli.protocol {
        config.protocol = match protocol_str.to_lowercase().as_str() {
            "unix" => ControlProtocol::Unix,
            "vsock" => ControlProtocol::Vsock,
            _ => {
                eprintln!("Invalid protocol '{}', use 'unix' or 'vsock'", protocol_str);
                std::process::exit(1);
            }
        };
    }

    if let Some(socket_path) = &cli.socket {
        config.unix_socket_path = socket_path.clone();
    }

    if let Some(cid) = cli.vsock_cid {
        config.vsock_cid = cid;
    }

    if let Some(port) = cli.vsock_port {
        config.vsock_port = port;
    }

    // Handle enable --now specially
    if let Commands::Enable { ref name, now } = cli.command {
        let enable_request = Request::ServiceEnable { name: name.clone() };
        let response = match config.protocol {
            ControlProtocol::Unix => send_request_unix(&config.unix_socket_path, enable_request)?,
            ControlProtocol::Vsock => send_request_vsock(config.vsock_cid, config.vsock_port, enable_request)?,
        };

        match response {
            Response::Success { message } => {
                println!("✓ {}", message);

                if now {
                    let start_request = Request::ServiceStart { name: name.clone() };
                    let start_response = match config.protocol {
                        ControlProtocol::Unix => send_request_unix(&config.unix_socket_path, start_request)?,
                        ControlProtocol::Vsock => send_request_vsock(config.vsock_cid, config.vsock_port, start_request)?,
                    };

                    match start_response {
                        Response::Success { message } => {
                            println!("✓ {}", message);
                        }
                        Response::Error { message } => {
                            eprintln!("✗ Error starting service: {}", message);
                            std::process::exit(1);
                        }
                        _ => {}
                    }
                }
            }
            Response::Error { message } => {
                eprintln!("✗ Error: {}", message);
                std::process::exit(1);
            }
            _ => {}
        }

        return Ok(());
    }

    let request = match &cli.command {
        Commands::List => Request::ListServices,
        Commands::Status { name } => Request::ServiceStatus { name: name.clone() },
        Commands::Start { name } => Request::ServiceStart { name: name.clone() },
        Commands::Stop { name } => Request::ServiceStop { name: name.clone() },
        Commands::Restart { name } => Request::ServiceRestart { name: name.clone() },
        Commands::Enable { name, .. } => Request::ServiceEnable { name: name.clone() },
        Commands::Disable { name } => Request::ServiceDisable { name: name.clone() },
        Commands::Logs { name, lines } => Request::ServiceLogs { name: name.clone(), lines: *lines },
        Commands::LogsClear { name } => Request::ServiceLogsClear { name: name.clone() },

        Commands::Ps(ps_cmd) => match ps_cmd {
            PsCommands::List => Request::ProcessList,
            PsCommands::Status { pid } => Request::ProcessStatus { pid: *pid },
            PsCommands::Start { command, args, env } => Request::ProcessStart {
                command: command.clone(),
                args: args.clone(),
                env: env.clone(),
            },
            PsCommands::Stop { pid } => Request::ProcessStop { pid: *pid },
            PsCommands::Restart { pid } => Request::ProcessRestart { pid: *pid },
            PsCommands::Kill { pid, signal } => Request::ProcessKill { pid: *pid, signal: *signal },
        },

        Commands::SystemStatus => Request::SystemStatus,
        Commands::Reload => Request::SystemReload,
        Commands::Reboot => Request::SystemReboot,
        Commands::Shutdown => Request::SystemShutdown,
        Commands::Ping => Request::Ping,
    };

    let response = match config.protocol {
        ControlProtocol::Unix => {
            send_request_unix(&config.unix_socket_path, request)?
        }
        ControlProtocol::Vsock => {
            send_request_vsock(config.vsock_cid, config.vsock_port, request)?
        }
    };

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
                println!("{:<25} {:<10} {:<10} {:<15} {:<10}", "NAME", "ENABLED", "ACTIVE", "RESTART", "RESTARTS");
                println!("{}", "-".repeat(75));
                for service in services {
                    println!(
                        "{:<25} {:<10} {:<10} {:<15} {:<10}",
                        service.name,
                        if service.enabled { "enabled" } else { "disabled" },
                        if service.active { "active" } else { "inactive" },
                        service.restart_policy,
                        service.restart_count
                    );
                }
            }
        }
        Response::ServiceStatus { status } => {
            println!("Service: {}", status.name);
            println!("  Enabled: {}", if status.enabled { "yes" } else { "no" });
            println!("  Status: {}", if status.active { "active (running)" } else { "inactive (dead)" });
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

            if !status.dependencies.before.is_empty() {
                println!("  Before: {}", status.dependencies.before.join(", "));
            }
            if !status.dependencies.after.is_empty() {
                println!("  After: {}", status.dependencies.after.join(", "));
            }
            if !status.dependencies.requires.is_empty() {
                println!("  Requires: {}", status.dependencies.requires.join(", "));
            }
            if !status.dependencies.required_by.is_empty() {
                println!("  Required By: {}", status.dependencies.required_by.join(", "));
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
        Response::ProcessList { processes } => {
            if processes.is_empty() {
                println!("No processes found");
            } else {
                println!("{:<8} {:<8} {:<12} {:<6} {:<8} {:<10} {:<10} {}",
                         "PID", "PPID", "STATE", "CPU%", "MEM", "MANAGED", "SERVICE", "COMMAND");
                println!("{}", "-".repeat(100));

                for process in processes {
                    let managed = if process.managed { "yes" } else { "no" };
                    let service = process.service_name.as_deref().unwrap_or("-");
                    let cmdline = if process.cmdline.len() > 50 {
                        format!("{}...", &process.cmdline[..47])
                    } else {
                        process.cmdline.clone()
                    };

                    println!("{:<8} {:<8} {:<12} {:<6.1} {:<8} {:<10} {:<10} {}",
                             process.pid,
                             process.ppid,
                             format_state(&process.state),
                             process.cpu_percent,
                             format_memory(process.memory_kb),
                             managed,
                             service,
                             cmdline);
                }
            }
        }
        Response::ProcessStatus { process } => {
            println!("Process: {}", process.pid);
            println!("  Name: {}", process.name);
            println!("  Parent PID: {}", process.ppid);
            println!("  State: {}", format_state(&process.state));
            println!("  Command: {}", process.cmdline);
            println!("  CPU: {:.1}%", process.cpu_percent);
            println!("  Memory: {}", format_memory(process.memory_kb));
            println!("  Start Time: {}", process.start_time);
            println!("  Managed by Init: {}", if process.managed { "yes" } else { "no" });
            if let Some(service_name) = process.service_name {
                println!("  Service: {}", service_name);
            }
        }
        Response::ProcessStarted { pid, message } => {
            println!("✓ {} (PID: {})", message, pid);
        }
        Response::SystemStatus { status } => {
            println!("System Status");
            println!("  Uptime: {}", format_uptime(status.uptime_secs));
            println!("  Services: {} total, {} enabled, {} active",
                     status.total_services, status.enabled_services, status.active_services);
            println!("  Processes: {} total", status.total_processes);
            println!("  Service Directory: {}", status.service_dir);
            println!("  Log Directory: {}", status.log_dir);
        }
        Response::Pong => {
            println!("✓ Pong - init system is responsive");
        }
    }

    Ok(())
}
