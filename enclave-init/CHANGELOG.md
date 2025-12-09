# Enclave's Init System Changelog

# v0.1.0

Rust implementation of Enclave's Init System, rewritten from CLang based `init` for enclaves.

## Analysis of the C Code

CLang based `init` system for an enclaves (AWS Nitro Enclaves) that:
1. Sets up the filesystem hierarchy (mounting /proc, /sys, /dev, etc.)
2. Loads the NSM (Nitro Secure Module) driver
3. Signals readiness via VSOCK
4. Reads configuration from /env and /cmd files
5. Performs a pivot root operation
6. Launches a child process and reaps zombies

## Key Points of the Rust Implementation

1. **Error Handling**: Maintains the same error handling pattern as C (die on errors with errno), but uses Rust's `Result` types where appropriate.

2. **Memory Safety**: No manual memory management - Rust's ownership system handles it automatically.

3. **Type Safety**: Uses enums for operation types instead of C's union, providing type-safe variants.

4. **FFI**: Uses `nix` crate for most system calls (which provides safe wrappers), and `libc` directly only where necessary (like `execvpe`, `freopen`, `finit_module`).

5. **Constants**: All C `#define` constants are converted to Rust constants with appropriate types.

6. **Static Arrays**: The `OPS` array is defined as a static slice of `InitOp` enums, mimicking the C struct array.

7. **Signal Handling**: Uses `nix::sys::signal` for blocking signals.

8. **VSOCK Communication**: Uses `nix::sys::socket` with `VsockAddr` for the vsock connection.

9. **Process Management**: Fork/exec pattern is preserved with proper error handling.

The implementation follows the C code's logic step-by-step while leveraging Rust's safety guarantees and idiomatic patterns.

# v0.2.0

## Changes:

   - Makes error handling infallible, i.e. init will not die/panic on errors, but handle errors properly with log messages, thus init will exist during whole enclave's run-time.
   - Instead of `/env` and `/cmd` files introducing the service files (in a systemd fashion and format), placed in `/service` directory, containing command for running app and its environment variables for app run-time, and policy for app restart (like `Restart=always` and `RestartSec=5`), and makes signals handling for properly handling processes running by init (especially processes automatic restarting from init on process termination according to service files).

These changes are introducing a robust init system with proper error handling, systemd-style service files, and signal handling.

## Key Features

1. **Robust Error Handling**: All errors are logged but don't crash init. The system remains running throughout the enclave's lifetime.

2. **Service Management**:
   - Systemd-style TOML service files
   - Support for `ExecStart`, `Environment`, `WorkingDirectory`
   - Restart policies: `no`, `always`, `on-failure`, `on-success`
   - Configurable restart delay with `RestartSec`

3. **Signal Handling**:
   - `SIGCHLD`: Reaps zombie processes and triggers service restarts
   - `SIGTERM`/`SIGINT`: Graceful shutdown (SIGTERM then SIGKILL)
   - All signals handled properly without crashing init

4. **Process Supervision**:
   - Automatic restart based on policy
   - Tracking of exit codes and restart counts
   - Restart delay enforcement

5. **Logging**: Clear logging at INFO, WARN, ERROR, and DEBUG levels to stderr

The init system now runs forever, supervising services and handling all edge cases gracefully!

## Example of service file format:

**Example service file format (`/service/myapp.service`):**
```toml
ExecStart = "/usr/bin/myapp --config /etc/myapp.conf"
Environment = [
    "LOG_LEVEL=info",
    "DATABASE_URL=postgres://localhost/mydb"
]
Restart = "always"
RestartSec = 5
WorkingDirectory = "/var/lib/myapp"
```

# v0.3.0

Creation of a complete init system with a CLI tool for interaction - `initctl`. `initctl` tool uses UNIX domain sockets for IPC between the init process and the CLI tool.
This change introduce the ability to interact with enclave's init system through CLI (separate tool), i.e. start, stop, restart services, read status and logs of the service, etc.
And also introducing CLI sub-command to restart/reboot and shutdown system, i.e. enclave.

## Key Features

1. **Complete IPC System**: Unix domain socket for communication between init and initctl
2. **Service Management**: Full CRUD operations on services
3. **Process Supervision**: Automatic restarts with configurable policies
4. **Logging**: Per-service log collection (last 1000 lines in memory)
5. **System Control**: Reboot and shutdown commands
6. **Error Handling**: All operations handle errors gracefully without crashing init
7. **Signal Handling**: Proper signal handling for graceful shutdowns
8. **CLI Tool**: User-friendly command-line interface with colorized output

The init system now runs continuously, managing services and responding to control requests through the socket interface!

## Example of service files:

**Example service file (`/service/webapp.service`):**
```toml
ExecStart = "/usr/bin/python3 /app/server.py"
Environment = [
    "PORT=8080",
    "LOG_LEVEL=info",
    "DATABASE_URL=sqlite:///data/app.db"
]
Restart = "always"
RestartSec = 5
WorkingDirectory = "/app"
```

**Example service file (`/service/worker.service`):**
```toml
ExecStart = "/usr/bin/node /app/worker.js"
Environment = [
    "NODE_ENV=production",
    "QUEUE_URL=redis://localhost:6379"
]
Restart = "on-failure"
RestartSec = 10
WorkingDirectory = "/app"
```

## Usage Examples

```bash
# List all services
initctl list

# Check service status
initctl status webapp

# Start a service
initctl start webapp

# Stop a service
initctl stop webapp

# Restart a service
initctl restart webapp

# View service logs (last 50 lines by default)
initctl logs webapp

# View more log lines
initctl logs webapp -n 100

# Reboot the enclave
initctl reboot

# Shutdown the enclave
initctl shutdown

# Ping init system
initctl ping
```

# v0.4.0

Added configuration support for the init system and implement file-based logging for services:
   - Added support for configuration of init system itself (environment variables for init system can be set via config YAML file, `/service` services directory can be set in configuration file as well).
   - Made services logging into files in logs directory (for example, `/log` directory, set in the configuration file as well).

## Key Features

1. **Configuration File Support**: YAML configuration for all init settings
2. **Environment Variables**: Set init environment variables via config
3. **Configurable Paths**: Service directory, log directory, socket path, all configurable
4. **File-Based Logging**: Services log to individual files with automatic rotation
5. **Log Rotation**: Configurable max log size and number of rotated files
6. **VSOCK Configuration**: Enable/disable and configure VSOCK heartbeat
7. **Pivot Root Control**: Enable/disable pivot root via configuration
8. **System Status**: View uptime, service counts, and configuration
9. **Log Management**: View and clear service logs via CLI

The init system is now fully configurable and logs to persistent files!

## Example of init system configuration YAML file:

**Example configuration file (`/etc/init.yaml`):**
```yaml
# Service directory
service_dir: /service

# Log directory
log_dir: /log

# Control socket path
socket_path: /run/init.sock

# Maximum log file size (10 MB)
max_log_size: 10485760

# Maximum number of rotated log files to keep
max_log_files: 5

# Environment variables for init system
environment:
  TZ: UTC
  LANG: en_US.UTF-8
  HOME: /root

# VSOCK configuration
vsock:
  enabled: true
  cid: 3
  port: 9000

# NSM driver path (optional)
nsm_driver_path: nsm.ko

# Pivot root configuration
pivot_root: true
pivot_root_dir: /rootfs
```

## Usage Examples

```bash
# List all services
initctl list

# Check service status
initctl status webapp

# Start a service
initctl start webapp

# Stop a service
initctl stop webapp

# Restart a service
initctl restart webapp

# View service logs (last 50 lines by default)
initctl logs webapp

# View more log lines
initctl logs webapp -n 200

# Clear service logs
initctl logs-clear webapp

# Show system status
initctl system-status

# Reboot the enclave
initctl reboot

# Shutdown the enclave
initctl shutdown

# Ping init system
initctl ping

# Use custom socket path
initctl -s /custom/path/init.sock list
```

# v0.5.0

## Changes

Implementation of configuration file path options, service dependencies and services startup ordering, and enable/disable functionality for `initctl`, services, and service file configurations:
   - Passing the Init system actual configuration file path (`/etc/init.yaml` by default) via CLI options and environment variable.
   - Made possible to define service dependencies in service files and define services starting order in service files (as systemd options `Before=`, `After=`, `Requires=`, `RequiredBy=` in service files).
   - For `initctl` made CLI subcommands to enable/disable services (via `.disabled` file extension, when init does not handle `.disabled` services, and via `ServiceEnable=true/false` option in service file), and enable service and start it immediately (`enable --now`).

## Key Features

The Enclave's Init System implementation now includes:
1. CLI options for config file path (`--config` and `INIT_CONFIG` env var)
2. Service dependencies (`Before`, `After`, `Requires`, `RequiredBy`)
3. Topological sort for startup order
4. Enable/disable services via `.disabled` extension and `ServiceEnable` option
5. `enable --now` to enable and start immediately
6. `reload` command to reload configurations via SIGHUP

## Example of service files with dependencies and startup ordering:

**Example service file with dependencies (`/service/webapp.service`):**
```toml
ExecStart = "/usr/bin/python3 /app/server.py"
Environment = [
    "PORT=8080",
    "DATABASE_URL=postgresql://localhost/myapp"
]
Restart = "always"
RestartSec = 5
WorkingDirectory = "/app"
ServiceEnable = true

# Start after database is ready
After = ["database"]

# Required dependencies
Requires = ["database"]
```

**Example database service (`/service/database.service`):**
```toml
ExecStart = "/usr/bin/postgres -D /var/lib/postgresql/data"
Environment = [
    "POSTGRES_PASSWORD=secret"
]
Restart = "always"
RestartSec = 10
WorkingDirectory = "/var/lib/postgresql"
ServiceEnable = true

# Start before webapp
Before = ["webapp"]
```

## Usage Examples

```bash
# Start init with custom config
init --config /etc/my-init.yaml
# or
INIT_CONFIG=/etc/my-init.yaml init

# List services (shows enabled/disabled status)
initctl list

# Enable a service
initctl enable myapp

# Enable and start immediately
initctl enable --now myapp

# Disable a service (stops it first)
initctl disable myapp

# Reload service configurations
initctl reload

# Check service with dependencies
initctl status webapp
```

# v0.6.0

Added VSOCK support for the init control protocol:
   - Added support for init control protocol (`initctl`) over VSock as well, as it is implemented for Unix domain sockets now.
   - Added listening on dedicated `CID:PORT` for control protocol commands in a dedicated thread, as it is made for Unix sockets now.
   - Dedicated `CID` and `PORT` for control protocol are set in `init.yaml` as for Unix domain socket path as well.
   - The used protocol, VSock or Unix domain socket or both (listening both sockets, Unix domain socket and VSock), are set by configuration parameter in `init.yaml` as well.
   - Made `initctl.yaml` configuration file for `initctl` CLI client and include configuration for Unix domain socket and for VSock (`CID` and `PORT`), and parameter for which protocol will be used, over Unix domain socket or over VSock.

## Key Features

The Enclave's Init System implementation now supports:

1. **Dual protocol support**: Both Unix socket and VSOCK for control interface
2. **Configurable protocols**: Enable/disable each protocol independently
3. **VSOCK control listening**: Init listens on VSOCK for remote control
4. **Initctl configuration**: Separate config file for initctl with protocol selection
5. **CLI overrides**: Override protocol and connection parameters from command line
6. **Simultaneous listening**: Can listen on both Unix socket and VSOCK at the same time

## Examples of configuration YAML files for `init` system and `initctl`:

**Example `/etc/init.yaml`:**
```yaml
service_dir: /service
log_dir: /log

# Control socket configuration
control:
  # Enable Unix socket control interface
  unix_socket_enabled: true
  unix_socket_path: /run/init.sock

  # Enable VSOCK control interface
  vsock_enabled: true
  # Use VMADDR_CID_ANY (4294967295) to listen on any CID, or specific CID
  vsock_cid: 4294967295  # -1U / VMADDR_CID_ANY
  vsock_port: 9001

max_log_size: 10485760
max_log_files: 5

environment:
  TZ: UTC
  LANG: en_US.UTF-8

# VSOCK heartbeat configuration (different from control socket)
vsock:
  enabled: true
  cid: 3
  port: 9000

nsm_driver_path: nsm.ko
pivot_root: true
pivot_root_dir: /rootfs
```

**Example `/etc/initctl.yaml`:**
```yaml
# Protocol to use: "unix" or "vsock"
protocol: unix

# Unix socket configuration
unix_socket_path: /run/init.sock

# VSOCK configuration
vsock_cid: 3      # Parent CID for enclave access
vsock_port: 9001  # Control port
```

**Example `/etc/initctl.yaml` for host access:**
```yaml
# For host accessing enclave over VSOCK
protocol: vsock

# Unix socket (not used when protocol is vsock)
unix_socket_path: /run/init.sock

# VSOCK configuration
vsock_cid: 16     # Enclave CID
vsock_port: 9001  # Control port
```

## Usage examples

```bash
# Inside enclave (Unix socket)
initctl list

# From host (VSOCK)
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 list

# Or configure in /etc/initctl.yaml
initctl list
```

