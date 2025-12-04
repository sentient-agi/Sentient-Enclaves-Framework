# Enclave Init System

A robust, systemd-inspired init system designed specifically for AWS Nitro Enclaves and similar isolated environments. Written in Rust for safety, performance, and reliability.

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Architecture](#architecture)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Configuration Reference](#configuration-reference)
- [Service Files](#service-files)
- [CLI Reference](#cli-reference)
- [Usage Guide](#usage-guide)
- [Advanced Topics](#advanced-topics)
- [Troubleshooting](#troubleshooting)
- [Development](#development)
- [FAQ](#faq)
- [License](#license)

---

## Overview

The Enclave Init System is a minimal, production-ready init system (PID 1) designed to run inside secure enclaves. It provides process supervision, automatic service restarts, comprehensive logging, and a control interface for managing services at runtime.

### Key Characteristics

- **Minimal footprint**: Small binary size optimized for enclave environments
- **Reliable**: Written in Rust with comprehensive error handling
- **Non-crashing**: All errors are logged but never crash the init system
- **Service supervision**: Automatic process monitoring and restart policies
- **Runtime control**: Manage services without restarting the enclave
- **Persistent logging**: Per-service log files with automatic rotation
- **Configurable**: YAML-based configuration for all aspects of the system

---

## Features

### Core Features

- **Process Management**
  - PID 1 functionality (reaping zombie processes)
  - Process supervision and monitoring
  - Automatic service restarts based on policy
  - Graceful shutdown handling

- **Service Management**
  - Systemd-style service file format (TOML)
  - Support for multiple services
  - Per-service environment variables
  - Working directory configuration
  - Restart policies: `no`, `always`, `on-failure`, `on-success`
  - Configurable restart delays

- **Logging**
  - Per-service log files
  - Automatic log rotation based on size
  - Configurable retention (number of rotated files)
  - Timestamp prefixes
  - In-memory log cache for quick access

- **Runtime Control**
  - Unix domain socket-based IPC
  - CLI tool (`initctl`) for service management
  - Start, stop, restart services
  - View service status and logs
  - System-wide operations (reboot, shutdown)

- **Enclave Integration**
  - VSOCK heartbeat support for AWS Nitro Enclaves
  - NSM (Nitro Secure Module) driver loading
  - Configurable pivot root for filesystem isolation

- **Filesystem Initialization**
  - Automatic mounting of essential filesystems
  - `/proc`, `/sys`, `/dev`, `/tmp`, `/run` setup
  - cgroups initialization
  - Device node creation
  - Symlink management

---

## Architecture

### System Components

```
┌─────────────────────────────────────────────────────┐
│                    Enclave Host                     │
└─────────────────────────────────────────────────────┘
                         │
                    [VSOCK 9000]
                         │
┌─────────────────────────────────────────────────────┐
│                 Enclave Environment                 │
│                                                     │
│  ┌───────────────────────────────────────────┐      │
│  │              init (PID 1)                 │      │
│  │                                           │      │
│  │  - Signal Handling                        │      │
│  │  - Process Supervision                    │      │
│  │  - Filesystem Initialization              │      │
│  │  - Service Management                     │      │
│  │  - Control Socket (/run/init.sock)        │      │
│  └───────────────────────────────────────────┘      │
│           │              │              │           │
│     ┌─────┴─────┐  ┌─────┴─────┐  ┌─────┴─────┐     │
│     │ Service A │  │ Service B │  │ Service C │     │
│     └───────────┘  └───────────┘  └───────────┘     │
│                                                     │
│  ┌───────────────────────────────────────────┐      │
│  │              initctl                      │      │
│  │  (CLI tool for runtime control)           │      │
│  └───────────────────────────────────────────┘      │
│                                                     │
│  Filesystem Layout:                                 │
│  /etc/init.yaml        - Init configuration         │
│  /service/*.service    - Service definitions        │
│  /log/*.log            - Service logs               │
└─────────────────────────────────────────────────────┘
```

### Process Flow

```
┌──────────────┐
│ Init Startup │
└──────┬───────┘
       │
       ├─> Load Configuration (/etc/init.yaml)
       ├─> Setup Signal Handlers
       ├─> Initialize Filesystems
       ├─> Load NSM Driver (optional)
       ├─> Send VSOCK Heartbeat (optional)
       ├─> Perform Pivot Root (optional)
       ├─> Load Service Definitions
       ├─> Start All Services
       ├─> Start Control Socket Server
       │
       └─> ┌────────────────┐
           │   Main Loop    │
           └────────┬───────┘
                    │
                    ├─> Check SIGCHLD → Reap Children
                    ├─> Check SIGTERM/SIGINT → Shutdown
                    ├─> Restart Dead Services (per policy)
                    ├─> Handle Control Socket Requests
                    │
                    └─> [Loop continues]
```

---

## Installation

### Prerequisites

- Rust 1.91.0 or later
- Linux operating system (designed for enclaves)
- Standard build tools (gcc, make, etc.)

### Building from Source

```bash
# Clone the repository
git clone https://github.com/sentient-agi/Sentient-Enclaves-Framework.git
cd enclave-init

# Build release binaries
cargo build --release

# Binaries will be in target/release/
ls -lh target/release/init
ls -lh target/release/initctl
```

### Cross-Compilation for Enclaves

For musl-based static binaries (recommended for enclaves):

```bash
# Install musl target
rustup target add x86_64-unknown-linux-musl

# Build static binaries
cargo build --release --target x86_64-unknown-linux-musl

# Verify static linking
ldd target/x86_64-unknown-linux-musl/release/init
# Should output: "not a dynamic executable"
```

### Installation in Enclave Image

```bash
# Copy binaries to enclave filesystem
cp target/release/init /path/to/enclave/rootfs/sbin/init
cp target/release/initctl /path/to/enclave/rootfs/usr/bin/initctl

# Set proper permissions
chmod 755 /path/to/enclave/rootfs/sbin/init
chmod 755 /path/to/enclave/rootfs/usr/bin/initctl

# Create required directories
mkdir -p /path/to/enclave/rootfs/etc
mkdir -p /path/to/enclave/rootfs/service
mkdir -p /path/to/enclave/rootfs/log
```

---

## Quick Start

### 1. Create Configuration File

Create `/etc/init.yaml`:

```yaml
service_dir: /service
log_dir: /log
socket_path: /run/init.sock
max_log_size: 10485760  # 10 MB
max_log_files: 5

environment:
  TZ: UTC
  LANG: en_US.UTF-8

vsock:
  enabled: true
  cid: 3
  port: 9000

pivot_root: true
pivot_root_dir: /rootfs
```

### 2. Create Service Files

Create `/service/webapp.service`:

```toml
ExecStart = "/usr/bin/python3 /app/server.py"
Environment = [
    "PORT=8080",
    "LOG_LEVEL=info"
]
Restart = "always"
RestartSec = 5
WorkingDirectory = "/app"
```

Create `/service/worker.service`:

```toml
ExecStart = "/usr/bin/node /app/worker.js"
Environment = [
    "NODE_ENV=production"
]
Restart = "on-failure"
RestartSec = 10
```

### 3. Start Init System

The init system starts automatically as PID 1 when the enclave boots:

```bash
# Inside the enclave, init is already running as PID 1
ps aux | grep init
# root         1  0.0  0.1  12345  6789 ?        Ss   00:00   0:00 /sbin/init
```

### 4. Manage Services

```bash
# List all services
initctl list

# Check service status
initctl status webapp

# View logs
initctl logs webapp -n 100

# Restart a service
initctl restart webapp

# Stop a service
initctl stop worker

# Start a service
initctl start worker
```

---

## Configuration Reference

### Init Configuration File (`/etc/init.yaml`)

The main configuration file for the init system.

#### Location

- Default: `/etc/init.yaml`
- If not found, uses built-in defaults

#### Complete Example

```yaml
# Service directory containing .service files
service_dir: /service

# Directory for service log files
log_dir: /log

# Path to the control socket
socket_path: /run/init.sock

# Maximum size of a single log file in bytes (10 MB)
max_log_size: 10485760

# Maximum number of rotated log files to keep
max_log_files: 5

# Environment variables for the init system
# These are inherited by all services
environment:
  TZ: UTC
  LANG: en_US.UTF-8
  HOME: /root
  TERM: linux

# VSOCK configuration for AWS Nitro Enclaves
vsock:
  # Enable VSOCK heartbeat
  enabled: true
  # VSOCK Context ID (CID)
  cid: 3
  # VSOCK port number
  port: 9000

# Path to the NSM (Nitro Secure Module) driver
# Set to null to disable NSM driver loading
nsm_driver_path: nsm.ko

# Perform pivot root operation
pivot_root: true

# Source directory for pivot root
pivot_root_dir: /rootfs
```

#### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `service_dir` | string | `/service` | Directory containing service definition files |
| `log_dir` | string | `/log` | Directory where service logs are stored |
| `socket_path` | string | `/run/init.sock` | Unix domain socket path for IPC |
| `max_log_size` | integer | `10485760` | Maximum log file size in bytes before rotation |
| `max_log_files` | integer | `5` | Number of rotated log files to retain |
| `environment` | map | `{}` | Key-value pairs of environment variables |
| `vsock.enabled` | boolean | `true` | Enable VSOCK heartbeat to host |
| `vsock.cid` | integer | `3` | VSOCK Context ID |
| `vsock.port` | integer | `9000` | VSOCK port number |
| `nsm_driver_path` | string/null | `"nsm.ko"` | Path to NSM driver or null to disable |
| `pivot_root` | boolean | `true` | Perform pivot root operation on startup |
| `pivot_root_dir` | string | `/rootfs` | Source directory for pivot root |

#### Environment Variables

Environment variables defined in the configuration are:
1. Set for the init process itself
2. Inherited by all child processes (services)
3. Can be overridden by per-service environment variables

---

## Service Files

Service files define how individual services should be run and managed.

### File Format

- **Format**: TOML
- **Extension**: `.service`
- **Location**: Directory specified by `service_dir` in init configuration (default: `/service`)

### Complete Example

```toml
# Command to execute (required)
ExecStart = "/usr/bin/myapp --config /etc/myapp.conf --port 8080"

# Environment variables for this service
Environment = [
    "LOG_LEVEL=debug",
    "DATABASE_URL=postgresql://localhost/mydb",
    "API_KEY=secret123",
    "MAX_CONNECTIONS=100"
]

# Restart policy (optional, default: "no")
# Options: "no", "always", "on-failure", "on-success"
Restart = "always"

# Delay in seconds before restarting (optional, default: 5)
RestartSec = 10

# Working directory for the service (optional)
WorkingDirectory = "/var/lib/myapp"
```

### Service Configuration Options

| Option | Type | Required | Default | Description |
|--------|------|----------|---------|-------------|
| `ExecStart` | string | **Yes** | - | Command line to execute |
| `Environment` | array | No | `[]` | List of environment variables |
| `Restart` | string | No | `"no"` | Restart policy |
| `RestartSec` | integer | No | `5` | Seconds to wait before restart |
| `WorkingDirectory` | string | No | - | Working directory for the process |

### Restart Policies

#### `no`
Never restart the service automatically.

```toml
Restart = "no"
```

**Use case**: One-shot tasks, services that should not restart

#### `always`
Always restart the service regardless of exit status.

```toml
Restart = "always"
RestartSec = 5
```

**Use case**: Critical services that must always run (web servers, databases)

#### `on-failure`
Restart only if the service exits with a non-zero exit code.

```toml
Restart = "on-failure"
RestartSec = 10
```

**Use case**: Services that may exit successfully but should restart on errors

#### `on-success`
Restart only if the service exits with exit code 0.

```toml
Restart = "on-success"
RestartSec = 5
```

**Use case**: Periodic tasks that need to run continuously when successful

### Environment Variables

Services inherit environment variables in this order (later sources override earlier):

1. **Init system environment** (from `init.yaml`)
2. **Default PATH**: `PATH=/sbin:/usr/sbin:/bin:/usr/bin`
3. **Service-specific environment** (from service file)

Example:

```toml
Environment = [
    "PORT=8080",
    "LOG_LEVEL=info",
    "DATABASE_URL=postgres://db:5432/myapp",
    "REDIS_URL=redis://cache:6379",
    "API_TIMEOUT=30"
]
```

### Command Line Parsing

The `ExecStart` command supports:
- Simple commands: `"/usr/bin/app"`
- Arguments: `"/usr/bin/app --config /etc/app.conf"`
- Quoted arguments: `"/usr/bin/app --name \"My App\""`

**Note**: Does not support shell features like pipes, redirects, or variable expansion. Use a shell wrapper script if needed:

```toml
ExecStart = "/bin/sh /opt/app/start.sh"
```

### Service Examples

#### Web Server

```toml
ExecStart = "/usr/bin/python3 -m http.server 8080"
Environment = [
    "PYTHONUNBUFFERED=1"
]
Restart = "always"
RestartSec = 5
WorkingDirectory = "/var/www"
```

#### Database

```toml
ExecStart = "/usr/bin/postgres -D /var/lib/postgresql/data"
Environment = [
    "POSTGRES_PASSWORD=secret",
    "POSTGRES_DB=myapp"
]
Restart = "always"
RestartSec = 10
WorkingDirectory = "/var/lib/postgresql"
```

#### Background Worker

```toml
ExecStart = "/usr/bin/celery -A myapp worker --loglevel=info"
Environment = [
    "CELERY_BROKER_URL=redis://localhost:6379/0",
    "CELERY_RESULT_BACKEND=redis://localhost:6379/0"
]
Restart = "on-failure"
RestartSec = 15
WorkingDirectory = "/app"
```

#### Monitoring Agent

```toml
ExecStart = "/usr/bin/prometheus --config.file=/etc/prometheus/prometheus.yml"
Environment = [
    "GOMAXPROCS=2"
]
Restart = "always"
RestartSec = 5
```

---

## CLI Reference

### `initctl` - Init Control Tool

Command-line interface for managing the init system and services.

#### Synopsis

```bash
initctl [OPTIONS] <COMMAND>
```

#### Global Options

| Option | Short | Environment Variable | Default | Description |
|--------|-------|---------------------|---------|-------------|
| `--socket <PATH>` | `-s` | `INIT_SOCKET` | `/run/init.sock` | Path to init control socket |
| `--help` | `-h` | - | - | Show help information |
| `--version` | `-V` | - | - | Show version information |

#### Commands

### `list`

List all services with their current status.

**Syntax:**
```bash
initctl list
```

**Output:**
```
NAME                      ACTIVE     RESTART         RESTARTS
-------------------------------------------------------------
webapp                    active     always          3
worker                    inactive   on-failure      0
monitor                   active     always          1
```

**Columns:**
- `NAME`: Service name (from filename without .service extension)
- `ACTIVE`: `active` (running) or `inactive` (not running)
- `RESTART`: Restart policy
- `RESTARTS`: Number of times the service has been restarted

---

### `status`

Show detailed status of a specific service.

**Syntax:**
```bash
initctl status <SERVICE>
```

**Example:**
```bash
initctl status webapp
```

**Output:**
```
Service: webapp
  Status: active (running)
  PID: 1234
  Command: /usr/bin/python3 /app/server.py
  Working Directory: /app
  Restart Policy: always
  Restart Delay: 5s
  Restart Count: 3
  Last Exit Code: 0
```

**Exit Codes:**
- `0`: Success
- `1`: Service not found or error

---

### `start`

Start a stopped service.

**Syntax:**
```bash
initctl start <SERVICE>
```

**Example:**
```bash
initctl start webapp
```

**Output:**
```
✓ Service 'webapp' started
```

**Notes:**
- Service must be in stopped state
- Error if service is already running
- Clears manual stop flag (allows automatic restarts)

---

### `stop`

Stop a running service.

**Syntax:**
```bash
initctl stop <SERVICE>
```

**Example:**
```bash
initctl stop webapp
```

**Output:**
```
✓ Service 'webapp' stop signal sent
```

**Behavior:**
- Sends SIGTERM to the process
- Sets manual stop flag (prevents automatic restart)
- Process has up to 5 seconds to shutdown gracefully
- After 5 seconds, SIGKILL is sent during system shutdown

---

### `restart`

Restart a service (stop if running, then start).

**Syntax:**
```bash
initctl restart <SERVICE>
```

**Example:**
```bash
initctl restart webapp
```

**Output:**
```
✓ Service 'webapp' restarted
```

**Behavior:**
- If running: sends SIGTERM, waits 500ms, then starts
- If stopped: just starts the service
- Clears manual stop flag

---

### `logs`

Display logs for a service.

**Syntax:**
```bash
initctl logs [OPTIONS] <SERVICE>
```

**Options:**
| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--lines <N>` | `-n` | `50` | Number of lines to display |

**Examples:**
```bash
# Show last 50 lines (default)
initctl logs webapp

# Show last 100 lines
initctl logs webapp -n 100

# Show last 500 lines
initctl logs webapp --lines 500
```

**Output:**
```
[2024-01-15 10:30:45] Starting service webapp
[2024-01-15 10:30:45] Service webapp started with PID 1234
[2024-01-15 10:30:46] Listening on port 8080
[2024-01-15 10:35:22] Request from 10.0.1.5
[2024-01-15 10:35:23] Response 200 OK
```

**Log Format:**
- Each line prefixed with `[YYYY-MM-DD HH:MM:SS]`
- Logs are read from `/log/<service>.log`
- Shows rotated logs if requested lines exceed current log file

---

### `logs-clear`

Clear all logs for a service.

**Syntax:**
```bash
initctl logs-clear <SERVICE>
```

**Example:**
```bash
initctl logs-clear webapp
```

**Output:**
```
✓ Logs cleared for service 'webapp'
```

**Behavior:**
- Removes current log file
- Removes all rotated log files
- Creates new empty log file
- Service continues logging to new file

---

### `system-status`

Show overall system status and statistics.

**Syntax:**
```bash
initctl system-status
```

**Output:**
```
System Status
  Uptime: 2d 5h 32m 15s
  Services: 12 total, 10 active
  Service Directory: /service
  Log Directory: /log
```

**Information Displayed:**
- System uptime since init started
- Total number of services configured
- Number of currently active (running) services
- Configured service directory
- Configured log directory

---

### `reboot`

Reboot the system (enclave).

**Syntax:**
```bash
initctl reboot
```

**Output:**
```
✓ System reboot initiated
```

**Behavior:**
1. Sends SIGTERM to all running services
2. Waits 5 seconds for graceful shutdown
3. Sends SIGKILL to any remaining processes
4. Calls `reboot(RB_AUTOBOOT)` system call
5. Enclave restarts

**Warning:** This will terminate all services and restart the enclave.

---

### `shutdown`

Shutdown the system (enclave).

**Syntax:**
```bash
initctl shutdown
```

**Output:**
```
✓ System shutdown initiated
```

**Behavior:**
- Same as `reboot` but may not restart depending on enclave configuration
- Performs graceful shutdown of all services

---

### `ping`

Test connectivity to the init system.

**Syntax:**
```bash
initctl ping
```

**Output:**
```
✓ Pong - init system is responsive
```

**Use Cases:**
- Health checking
- Verify init system is running
- Test control socket connectivity

---

## Usage Guide

### Basic Operations

#### Starting the Init System

The init system is designed to run as PID 1 and starts automatically when the enclave boots. It's typically configured in your enclave's boot process:

```bash
# In enclave's init script or kernel command line
exec /sbin/init
```

#### Checking Service Status

```bash
# List all services
initctl list

# Check specific service
initctl status myapp

# Check if init is responsive
initctl ping
```

#### Managing Services

```bash
# Start a service
initctl start myapp

# Stop a service
initctl stop myapp

# Restart a service
initctl restart myapp
```

#### Viewing Logs

```bash
# View recent logs
initctl logs myapp

# View more logs
initctl logs myapp -n 200

# Monitor logs in real-time (requires tail)
tail -f /log/myapp.log

# Clear old logs
initctl logs-clear myapp
```

### Advanced Operations

#### Hot-Reloading Service Configuration

To update a service configuration without restarting the enclave:

```bash
# 1. Update the service file
vim /service/myapp.service

# 2. Restart the service to apply changes
initctl restart myapp
```

**Note:** Init system does not automatically reload service files. Services must be restarted to apply configuration changes.

#### Adding New Services at Runtime

```bash
# 1. Create new service file
cat > /service/newapp.service << EOF
ExecStart = "/usr/bin/newapp"
Restart = "always"
RestartSec = 5
EOF

# 2. Restart init system (requires enclave restart)
initctl reboot
```

**Note:** New service files are only loaded at init startup. Adding a new service requires an enclave restart.

#### Debugging Service Issues

```bash
# 1. Check service status
initctl status myapp

# 2. View logs
initctl logs myapp -n 100

# 3. Check exit code
# Look for "Last Exit Code" in status output

# 4. Try starting manually for debugging
# Stop the service first
initctl stop myapp

# Start in a shell for debugging
/usr/bin/myapp --verbose

# Once fixed, restart via init
initctl start myapp
```

#### Log Rotation Management

Logs automatically rotate when they exceed `max_log_size`. To manually manage logs:

```bash
# Check log file sizes
du -h /log/*.log

# View rotated logs
ls -lh /log/myapp.log*

# Clear logs if needed
initctl logs-clear myapp

# Or manually remove old rotations
rm /log/myapp.log.{3,4,5}
```

#### Service Dependencies

The init system does not have built-in dependency management. To handle service dependencies:

**Option 1: Use a wrapper script**

```bash
#!/bin/sh
# /opt/myapp/start-with-deps.sh

# Wait for database to be ready
until pg_isready -h localhost; do
    sleep 1
done

# Start the application
exec /usr/bin/myapp
```

Service file:
```toml
ExecStart = "/opt/myapp/start-with-deps.sh"
Restart = "always"
```

**Option 2: Use different restart delays**

```toml
# database.service - starts first
Restart = "always"
RestartSec = 5

# app.service - waits longer to start
Restart = "always"
RestartSec = 15
```

#### Resource Management

Monitor service resource usage:

```bash
# View all processes
ps aux

# View service PID
initctl status myapp | grep PID

# Monitor specific service
top -p $(initctl status myapp | grep PID | awk '{print $2}')

# Check memory usage
cat /proc/$(initctl status myapp | grep PID | awk '{print $2}')/status
```

### System Administration

#### Backup and Restore

**Backup service configurations:**
```bash
# Backup all service files
tar -czf services-backup.tar.gz /service/

# Backup init configuration
cp /etc/init.yaml /backup/init.yaml
```

**Restore service configurations:**
```bash
# Restore service files
tar -xzf services-backup.tar.gz -C /

# Restart init to load services
initctl reboot
```

#### Log Management Strategy

**Recommended log management:**

1. **Adjust log rotation settings** in `/etc/init.yaml`:
```yaml
max_log_size: 52428800  # 50 MB
max_log_files: 10       # Keep 10 rotations
```

2. **Periodically archive old logs**:
```bash
#!/bin/sh
# archive-logs.sh
tar -czf /archive/logs-$(date +%Y%m%d).tar.gz /log/*.log.*
find /log -name "*.log.*" -delete
```

3. **Monitor log directory size**:
```bash
du -sh /log
```

#### Graceful Shutdown

To safely shutdown the enclave:

```bash
# Option 1: Using initctl
initctl shutdown

# Option 2: Send signal to init
kill -TERM 1

# Option 3: System command (if available)
shutdown -h now
```

All methods will:
1. Stop all services gracefully (SIGTERM)
2. Wait 5 seconds
3. Force kill remaining processes (SIGKILL)
4. Shutdown the system

---

## Advanced Topics

### Signal Handling

The init system handles signals as follows:

| Signal | Behavior |
|--------|----------|
| `SIGCHLD` | Reap zombie processes, check for service exits |
| `SIGTERM` | Initiate graceful shutdown |
| `SIGINT` | Initiate graceful shutdown |
| Others | Blocked in init process |

**Child Process Signals:**
- All signals are unblocked in child processes
- Services receive signals directly
- Services can handle signals for graceful shutdown

### Process Lifecycle

```
Service Start:
  ┌─────────────┐
  │ Start Cmd   │
  └──────┬──────┘
         │
  ┌──────▼──────┐
  │    Fork     │
  └──────┬──────┘
         │
    ┌────┴────┐
    │ Parent  │ Child
    │         │
    │    ┌────▼────┐
    │    │ Setsid  │
    │    ├─────────┤
    │    │ Setpgid │
    │    ├─────────┤
    │    │ Chdir   │
    │    ├─────────┤
    │    │ Exec    │
    │    └────┬────┘
    │         │
    │    ┌────▼────┐
    │    │ Running │
    │    └────┬────┘
    │         │
    │    ┌────▼────┐
    │    │  Exit   │
    │    └────┬────┘
    │         │
    └────┬────┘
         │
  ┌──────▼──────┐
  │ SIGCHLD     │
  └──────┬──────┘
         │
  ┌──────▼──────┐
  │ Wait/Reap   │
  └──────┬──────┘
         │
  ┌──────▼──────┐
  │ Check Policy│
  └──────┬──────┘
         │
    Restart?
    Yes ├─┐
    No  │ │
        │ └──> [Wait RestartSec] ──> [Start Cmd]
        │
  ┌─────▼─────┐
  │    Done   │
  └───────────┘
```

### Filesystem Initialization Details

The init system sets up the following filesystems:

#### Essential Mounts

| Path | Type | Flags | Purpose |
|------|------|-------|---------|
| `/proc` | proc | `nodev,nosuid,noexec` | Process information |
| `/sys` | sysfs | `nodev,nosuid,noexec` | Kernel objects |
| `/dev` | devtmpfs | `nosuid,noexec` | Device nodes |
| `/dev/pts` | devpts | `nosuid,noexec` | Pseudo-terminals |
| `/dev/shm` | tmpfs | `nodev,nosuid,noexec` | Shared memory |
| `/tmp` | tmpfs | `nodev,nosuid,noexec` | Temporary files |
| `/run` | tmpfs | `nodev,nosuid,noexec` | Runtime data |
| `/sys/fs/cgroup` | tmpfs | `nodev,nosuid,noexec` | Cgroup root |

#### Symlinks Created

| Link | Target | Purpose |
|------|--------|---------|
| `/dev/fd` | `/proc/self/fd` | File descriptors |
| `/dev/stdin` | `/proc/self/fd/0` | Standard input |
| `/dev/stdout` | `/proc/self/fd/1` | Standard output |
| `/dev/stderr` | `/proc/self/fd/2` | Standard error |

#### Cgroups

The init system automatically mounts all enabled cgroup controllers under `/sys/fs/cgroup/`.

### VSOCK Integration

For AWS Nitro Enclaves, the init system can send a heartbeat to the host:

**Configuration:**
```yaml
vsock:
  enabled: true
  cid: 3        # Parent CID
  port: 9000    # Communication port
```

**Behavior:**
1. Connects to VSOCK address (CID:3, Port:9000)
2. Sends heartbeat byte (0xB7)
3. Waits for response
4. Verifies response matches heartbeat
5. Closes connection

**Use Cases:**
- Signal to host that enclave is ready
- Health checking from host
- Synchronization point for enclave startup

### NSM Driver Loading

For AWS Nitro Enclaves with NSM (Nitro Secure Module):

**Configuration:**
```yaml
nsm_driver_path: nsm.ko
```

**Behavior:**
1. Opens driver file
2. Calls `finit_module()` syscall
3. Loads driver into kernel
4. Deletes driver file
5. Driver available at `/dev/nsm`

**Disabling:**
```yaml
nsm_driver_path: null
```

### Pivot Root Operation

The init system can perform a pivot root to switch the root filesystem:

**Configuration:**
```yaml
pivot_root: true
pivot_root_dir: /rootfs
```

**Sequence:**
1. Bind mount `/rootfs` to itself
2. Change directory to `/rootfs`
3. Move mount to `/`
4. Chroot to current directory
5. Change to `/`
6. Re-initialize `/dev`, `/proc`, etc.

**Use Case:**
- Switching from initramfs to real root filesystem
- Filesystem isolation

### IPC Protocol

Communication between `initctl` and `init` uses JSON over Unix domain sockets.

**Request Format:**
```json
{
  "ServiceStatus": {
    "name": "webapp"
  }
}
```

**Response Format:**
```json
{
  "ServiceStatus": {
    "status": {
      "name": "webapp",
      "active": true,
      "pid": 1234,
      "restart_policy": "always",
      "restart_count": 3,
      "restart_sec": 5,
      "exit_status": null,
      "exec_start": "/usr/bin/python3 /app/server.py",
      "working_directory": "/app"
    }
  }
}
```

**Request Types:**
- `ListServices`
- `ServiceStatus { name }`
- `ServiceStart { name }`
- `ServiceStop { name }`
- `ServiceRestart { name }`
- `ServiceLogs { name, lines }`
- `ServiceLogsClear { name }`
- `SystemStatus`
- `SystemReboot`
- `SystemShutdown`
- `Ping`

---

## Troubleshooting

### Common Issues

#### Init System Not Starting

**Symptom:** Enclave fails to boot or hangs

**Solutions:**
1. Check if init binary is executable:
   ```bash
   ls -l /sbin/init
   # Should be: -rwxr-xr-x
   ```

2. Verify init is PID 1:
   ```bash
   ps aux | grep init
   # Should show PID 1
   ```

3. Check kernel command line:
   ```bash
   cat /proc/cmdline
   # Should have: init=/sbin/init
   ```

#### Service Won't Start

**Symptom:** Service shows as inactive, won't start

**Debugging:**
```bash
# Check service status
initctl status myapp

# View logs
initctl logs myapp

# Check service file syntax
cat /service/myapp.service

# Try running command manually
/usr/bin/myapp
```

**Common Causes:**
- Incorrect `ExecStart` path
- Missing executable permissions
- Missing dependencies
- Invalid working directory
- Environment variable issues

#### Service Keeps Restarting

**Symptom:** Service restarts repeatedly

**Debugging:**
```bash
# Check restart policy
initctl status myapp | grep "Restart Policy"

# View logs for error messages
initctl logs myapp -n 200

# Check exit codes
initctl status myapp | grep "Last Exit Code"
```

**Solutions:**
1. Fix application errors causing crashes
2. Adjust restart policy:
   ```toml
   Restart = "on-failure"
   RestartSec = 30  # Longer delay
   ```
3. Check for port conflicts or resource issues

#### Cannot Connect to Control Socket

**Symptom:** `initctl: Failed to connect to init socket`

**Solutions:**
```bash
# Check if socket exists
ls -l /run/init.sock

# Check if init is running
ps aux | grep init

# Check socket path in config
cat /etc/init.yaml | grep socket_path

# Use correct socket path
initctl -s /run/init.sock list
```

#### Logs Not Appearing

**Symptom:** `initctl logs` shows no output

**Debugging:**
```bash
# Check log directory exists
ls -ld /log

# Check log file
ls -l /log/myapp.log

# Check permissions
ls -l /log

# View log file directly
cat /log/myapp.log
```

**Solutions:**
1. Ensure log directory exists and is writable
2. Check `log_dir` in `/etc/init.yaml`
3. Verify service is actually writing to stdout/stderr

#### Service Shows Wrong Status

**Symptom:** Service shows as active but is not running

**Debugging:**
```bash
# Check actual process
ps aux | grep myapp

# Check PID from status
initctl status myapp | grep PID

# Verify process exists
kill -0 <PID>
```

**Solutions:**
- Wait a moment for status to update
- Service may have just crashed (check logs)
- Restart init system if status is stale

### Debug Mode

To enable debug logging, add to your init configuration:

```yaml
environment:
  RUST_LOG: debug
```

Then check logs:
```bash
# View all init system logs
journalctl -u init

# Or check console output
dmesg | grep init
```

### Performance Issues

#### High CPU Usage

```bash
# Check which process is using CPU
top

# Check service CPU usage
ps aux --sort=-%cpu | head

# Reduce restart frequency if service is crash-looping
# Edit service file:
RestartSec = 60  # Wait longer between restarts
```

#### High Memory Usage

```bash
# Check memory usage
free -h

# Check per-service memory
ps aux --sort=-%mem | head

# Check for memory leaks in services
# Monitor over time:
watch -n 5 'ps aux | grep myapp'
```

#### Too Many Log Files

```bash
# Check log directory size
du -sh /log

# Adjust rotation settings in /etc/init.yaml:
max_log_size: 5242880    # 5 MB (smaller files)
max_log_files: 3         # Fewer rotations

# Or clear old logs
initctl logs-clear myapp
```

---

## Development

### Building from Source

```bash
# Clone repository
git clone https://github.com/sentient-agi/Sentient-Enclaves-Framework.git
cd enclave-init

# Build debug version
cargo build

# Build release version
cargo build --release

# Run tests
cargo test

# Check code
cargo clippy

# Format code
cargo fmt
```

### Project Structure

```
enclave-init/
├── Cargo.toml              # Rust dependencies and build config
├── src/
│   ├── main.rs            # Init system (PID 1)
│   ├── initctl.rs         # CLI control tool
│   ├── protocol.rs        # IPC protocol definitions
│   ├── config.rs          # Configuration loading
│   └── logger.rs          # Logging implementation
├── examples/
│   ├── init.yaml          # Example init configuration
│   └── services/          # Example service files
├── tests/
│   └── integration/       # Integration tests
└── README.md              # This file
```

### Testing

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test '*'

# Test specific module
cargo test config

# Test with output
cargo test -- --nocapture
```

### Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Run `cargo fmt` and `cargo clippy`
6. Submit a pull request

### Code Style

- Follow Rust standard style (enforced by `rustfmt`)
- Use `clippy` for linting
- Add documentation comments for public APIs
- Write tests for new features

---

## FAQ

### General Questions

**Q: Can I use this outside of AWS Nitro Enclaves?**

A: Yes! While designed for enclaves, it works in any Linux environment. Disable enclave-specific features:
```yaml
vsock:
  enabled: false
nsm_driver_path: null
pivot_root: false
```

**Q: Does it support systemd service files?**

A: No, it uses TOML format which is simpler to parse. However, basic directives (`ExecStart`, `Restart`, etc.) are similar to systemd.

**Q: Can services run as different users?**

A: Not currently. All services run as the same user as init (typically root). This is by design for enclave environments.

**Q: How do I run multiple instances of the same service?**

A: Create separate service files:
```
/service/worker-1.service
/service/worker-2.service
/service/worker-3.service
```

**Q: Can I reload configuration without restarting?**

A: Service file changes require restarting the specific service. Init configuration changes require an enclave reboot.

### Service Management

**Q: How do I prevent a service from starting at boot?**

A: Remove or rename the service file:
```bash
mv /service/myapp.service /service/myapp.service.disabled
```

**Q: Can services communicate with each other?**

A: Yes, through normal IPC mechanisms (sockets, pipes, shared memory, etc.). The init system doesn't impose restrictions.

**Q: How do I ensure one service starts before another?**

A: Use wrapper scripts or delays. See [Service Dependencies](#service-dependencies) section.

**Q: What happens if all services exit?**

A: Init continues running, waiting for commands via control socket. It will never exit voluntarily.

### Logging

**Q: Can I send logs to syslog?**

A: Services can log to syslog if configured. Init system logs to files only.

**Q: How long are logs kept?**

A: Based on rotation settings. With default config (10MB × 5 files), up to 50MB per service.

**Q: Can I export logs to external storage?**

A: Yes, periodically copy `/log/` directory or setup a service that forwards logs.

**Q: Are logs persistent across reboots?**

A: Only if `/log` is on persistent storage. In enclaves, usually stored in memory and lost on reboot.

### Performance

**Q: What's the overhead of the init system?**

A: Minimal. Init uses <10MB RAM typically and negligible CPU when idle.

**Q: How many services can it manage?**

A: Tested with 100+ services. No hard limit, but consider resource constraints.

**Q: Does it support cgroups resource limits?**

A: Init mounts cgroups but doesn't configure limits. You can configure limits manually or via service wrapper scripts.

### Security

**Q: Is the control socket secured?**

A: It uses Unix domain socket permissions. Only users with access to the socket can control services.

**Q: Can services escape the enclave?**

A: No, enclave isolation is enforced by the hypervisor, not init.

**Q: Does it validate service files?**

A: Basic validation only. Malformed files are logged and skipped.

---

## Appendix

### Complete Configuration Examples

#### Minimal Configuration

```yaml
# /etc/init.yaml - Bare minimum
service_dir: /service
log_dir: /log
```

#### Production Configuration

```yaml
# /etc/init.yaml - Production settings
service_dir: /service
log_dir: /var/log/services
socket_path: /run/init.sock

max_log_size: 52428800    # 50 MB
max_log_files: 10

environment:
  TZ: UTC
  LANG: en_US.UTF-8
  HOME: /root
  PATH: /usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

vsock:
  enabled: true
  cid: 3
  port: 9000

nsm_driver_path: /lib/modules/nsm.ko
pivot_root: true
pivot_root_dir: /rootfs
```

#### Development Configuration

```yaml
# /etc/init.yaml - Development settings
service_dir: /opt/services
log_dir: /opt/logs
socket_path: /tmp/init.sock

max_log_size: 1048576     # 1 MB - rotate frequently
max_log_files: 3

environment:
  TZ: America/New_York
  DEBUG: "true"
  RUST_LOG: debug

vsock:
  enabled: false

nsm_driver_path: null
pivot_root: false
```

### Complete Service Examples

#### Python Web Application

```toml
# /service/webapp.service
ExecStart = "/usr/bin/python3 -m uvicorn app:main --host 0.0.0.0 --port 8080"
Environment = [
    "PYTHONUNBUFFERED=1",
    "DATABASE_URL=postgresql://localhost/myapp",
    "SECRET_KEY=change-me-in-production",
    "WORKERS=4"
]
Restart = "always"
RestartSec = 5
WorkingDirectory = "/app"
```

#### Node.js Microservice

```toml
# /service/api.service
ExecStart = "/usr/bin/node server.js"
Environment = [
    "NODE_ENV=production",
    "PORT=3000",
    "REDIS_URL=redis://localhost:6379",
    "LOG_LEVEL=info"
]
Restart = "always"
RestartSec = 10
WorkingDirectory = "/opt/api"
```

#### Rust Binary Service

```toml
# /service/processor.service
ExecStart = "/usr/local/bin/processor --config /etc/processor.toml"
Environment = [
    "RUST_LOG=info",
    "RUST_BACKTRACE=1"
]
Restart = "on-failure"
RestartSec = 15
WorkingDirectory = "/var/lib/processor"
```

#### Shell Script Service

```toml
# /service/monitor.service
ExecStart = "/bin/sh /opt/scripts/monitor.sh"
Environment = [
    "CHECK_INTERVAL=60",
    "ALERT_EMAIL=admin@example.com"
]
Restart = "always"
RestartSec = 5
```

#### Java Application

```toml
# /service/backend.service
ExecStart = "/usr/bin/java -jar /opt/backend/app.jar"
Environment = [
    "JAVA_OPTS=-Xmx2g -Xms512m",
    "SPRING_PROFILES_ACTIVE=production",
    "SERVER_PORT=8080"
]
Restart = "always"
RestartSec = 20
WorkingDirectory = "/opt/backend"
```

### Error Codes Reference

#### Init System Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Normal shutdown (not typical for init) |
| Other | System error (errno value) |

#### Service Exit Codes

| Code | Meaning | Restart Behavior |
|------|---------|------------------|
| 0 | Success | `always`, `on-success` |
| 1 | General error | `always`, `on-failure` |
| 2 | Misuse of shell builtin | `always`, `on-failure` |
| 126 | Command cannot execute | `always`, `on-failure` |
| 127 | Command not found | `always`, `on-failure` |
| 128+N | Fatal signal N | `always`, `on-failure` |
| 130 | Terminated by Ctrl+C | `always`, `on-failure` |
| 143 | Terminated by SIGTERM | `always`, `on-failure` |
| 137 | Killed by SIGKILL | `always`, `on-failure` |

### Signal Reference

#### Signals to Init Process

| Signal | Action | Description |
|--------|--------|-------------|
| SIGCHLD | Handle | Reap zombie child processes |
| SIGTERM | Shutdown | Graceful shutdown sequence |
| SIGINT | Shutdown | Graceful shutdown (Ctrl+C) |
| Others | Blocked | Ignored by init process |

#### Signals to Service Processes

Services receive all signals normally. Common signals:

| Signal | Number | Default Action | Use Case |
|--------|--------|----------------|----------|
| SIGTERM | 15 | Terminate | Graceful shutdown |
| SIGKILL | 9 | Kill | Force termination |
| SIGINT | 2 | Terminate | Interrupt (Ctrl+C) |
| SIGHUP | 1 | Terminate | Reload configuration |
| SIGUSR1 | 10 | Terminate | User-defined |
| SIGUSR2 | 12 | Terminate | User-defined |

### Filesystem Layout

Typical enclave filesystem layout:

```
/
├── sbin/
│   └── init                    # Init binary (PID 1)
├── usr/
│   └── bin/
│       └── initctl            # Control tool
├── etc/
│   ├── init.yaml              # Init configuration
│   └── ...
├── service/                   # Service definitions
│   ├── webapp.service
│   ├── worker.service
│   └── monitor.service
├── log/                       # Service logs
│   ├── webapp.log
│   ├── webapp.log.1
│   ├── worker.log
│   └── monitor.log
├── run/
│   └── init.sock             # Control socket
├── proc/                      # Process information
├── sys/                       # Kernel objects
├── dev/                       # Device nodes
└── tmp/                       # Temporary files
```

### Performance Tuning

#### Reducing Init System Overhead

```yaml
# Minimize logging
max_log_size: 1048576         # 1 MB
max_log_files: 2

# Reduce check frequency (in code, not configurable)
# Main loop sleeps 100ms between checks
```

#### Optimizing Service Restarts

```toml
# For services that start quickly
RestartSec = 1

# For services that need time to clean up
RestartSec = 30

# For services that rarely fail
Restart = "on-failure"
RestartSec = 60
```

#### Log Management

```bash
# Compress old logs
find /log -name "*.log.*" -exec gzip {} \;

# Archive and remove old logs
tar -czf logs-archive-$(date +%Y%m%d).tar.gz /log/*.gz
find /log -name "*.gz" -delete

# Limit log directory size with cron
du -s /log | awk '{if($1>102400)system("find /log -name \"*.log.*\" -delete")}'
```

---

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE-APACHE) file for details.

---

## Support

For issues, questions, or contributions:

- **Issue Tracker**: [GitHub Issues](https://github.com/sentient-agi/Sentient-Enclaves-Framework/issues)
- **Discussions**: [GitHub Discussions](https://github.com/sentient-agi/Sentient-Enclaves-Framework/discussions)
- **Email**: Sentient Enclaves Team <sentient-enclaves-team@sentient.xyz>

---

## Changelog

### Version 0.4.0 (Alpha State Release)

- Init system with process supervision
- Service management with restart policies
- File-based logging with rotation
- CLI control tool (`initctl`)
- YAML configuration support
- VSOCK integration for Nitro Enclaves
- NSM driver loading
- Comprehensive documentation

---

