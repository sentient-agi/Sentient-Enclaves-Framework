# Enclave Init System

A robust, systemd-inspired init system designed specifically for AWS Nitro Enclaves and similar isolated environments. Written in Rust for safety, performance, and reliability.

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Architecture](#architecture)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Configuration Reference](#configuration-reference)
  - [Init Configuration](#init-configuration-inityaml)
  - [Initctl Configuration](#initctl-configuration-initctlyaml)
- [Service Files](#service-files)
- [Service Dependencies](#service-dependencies)
- [Control Protocols](#control-protocols)
- [Process Management](#process-management)
- [Logs Streaming](#logs-streaming)
- [CLI Reference](#cli-reference)
- [Usage Guide](#usage-guide)
- [Advanced Topics](#advanced-topics)
- [Troubleshooting](#troubleshooting)
- [Development](#development)
- [FAQ](#faq)
- [Appendix](#appendix)

---

## Overview

The Enclave Init System is a minimal, production-ready init system (PID 1) designed to run inside secure enclaves. It provides process supervision, automatic service restarts, service dependency management, comprehensive logging, dual-protocol control interfaces (Unix socket and VSOCK), and system-wide process management capabilities.

### Key Characteristics

- **Minimal footprint**: Small binary size optimized for enclave environments
- **Reliable**: Written in Rust with comprehensive error handling
- **Non-crashing**: All errors are logged but never crash the init system
- **Service supervision**: Automatic process monitoring and restart policies
- **Dependency management**: Systemd-style service dependencies with startup ordering
- **Runtime control**: Manage services without restarting the enclave
- **Dual protocol support**: Control via Unix socket (local) or VSOCK (remote)
- **Process management**: List, monitor, and control all system processes
- **Enable/Disable**: Dynamic service activation control
- **Persistent logging**: Per-service log files with automatic rotation
- **Configurable**: YAML-based configuration for all aspects of the system
- **Flexible**: Configuration file path configurable via CLI and environment
- **Remote management**: Control enclave services from host via VSOCK

---

## Features

### Core Features

- **Process Management**
  - PID 1 functionality (reaping zombie processes)
  - Process supervision and monitoring
  - Automatic service restarts based on policy
  - Graceful shutdown handling
  - Signal handling (SIGTERM, SIGINT, SIGHUP, SIGCHLD)
  - System-wide process listing and monitoring
  - Process information (CPU, memory, state, command line)
  - Start, stop, restart, and signal processes
  - Track managed vs unmanaged processes

- **Service Management**
  - Systemd-style service file format (TOML)
  - Support for multiple services
  - Per-service environment variables
  - Working directory configuration
  - Restart policies: `no`, `always`, `on-failure`, `on-success`
  - Configurable restart delays
  - Enable/disable services at runtime
  - Service dependencies and ordering
  - Service-to-process mapping

- **Dependency Management**
  - `Before`: Specify services that should start after this one
  - `After`: Specify services that should start before this one
  - `Requires`: Hard dependencies (must exist and start first)
  - `RequiredBy`: Reverse dependency specification
  - Automatic topological sorting for startup order
  - Circular dependency detection

- **Dual Protocol Control**
  - **Unix Socket**: Local control interface for in-enclave management
  - **VSOCK**: Remote control interface for host-to-enclave management
  - Independent enable/disable for each protocol
  - Simultaneous listening on both protocols
  - Configurable CID and port for VSOCK
  - Protocol selection in client configuration

- **Logging**
  - Per-service log files
  - Automatic log rotation based on size
  - Configurable retention (number of rotated files)
  - Timestamp prefixes
  - In-memory log cache for quick access
  - View and clear logs via CLI

- **Runtime Control**
  - CLI tool (`initctl`) for service and process management
  - Start, stop, restart services
  - Enable, disable services
  - Reload configurations without restart
  - View service status and logs
  - List and monitor all processes
  - Start ad-hoc processes
  - Send signals to processes
  - System-wide operations (reload, reboot, shutdown)
  - Remote control from host via VSOCK

- **Enclave Integration**
  - VSOCK heartbeat support for AWS Nitro Enclaves
  - NSM (Nitro Secure Module) driver loading
  - Configurable pivot root for filesystem isolation
  - Host-to-enclave management via VSOCK control protocol

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
┌───────────────────────────────────────────────────┐
│                    Host System                    │
│                                                   │
│  ┌───────────────────────────────────────────┐    │
│  │         initctl (Host)                    │    │
│  │  - Service Management                     │    │
│  │  - Process Management                     │    │
│  │  (VSOCK Client)                           │    │
│  └───────────────┬───────────────────────────┘    │
│                  │                                │
│             [VSOCK CID:16 PORT:9001]              │
└──────────────────┼────────────────────────────────┘
                   │
                   │ VSOCK Connection
                   │
┌──────────────────┴────────────────────────────────┐
│                 Enclave Environment               │
│                                                   │
│  ┌───────────────────────────────────────────┐    │
│  │              init (PID 1)                 │    │
│  │                                           │    │
│  │  - Signal Handling (TERM/INT/HUP/CHLD)    │    │
│  │  - Process Supervision                    │    │
│  │  - Service Management                     │    │
│  │  - System Process Management              │    │
│  │  - Filesystem Initialization              │    │
│  │  - Dependency Resolution                  │    │
│  │  - Unix Socket (/run/init.sock)           │    │
│  │  - VSOCK Socket (CID:ANY PORT:9001)       │    │
│  └────────┬──────────────┬──────────────┬────┘    │
│           │              │              │         │
│     ┌─────┴─────┐  ┌─────┴─────┐  ┌─────┴─────┐   │
│     │ Service A │  │ Service B │  │ Service C │   │
│     │  (PID X)  │  │  (PID Y)  │  │  (PID Z)  │   │
│     │(depends B)│  │           │  │(after A,B)│   │
│     └───────────┘  └───────────┘  └───────────┘   │
│                                                   │
│  ┌───────────────────────────────────────────┐    │
│  │         initctl (Enclave)                 │    │
│  │  - Service Management                     │    │
│  │  - Process Management (ps commands)       │    │
│  │  (Unix Socket Client)                     │    │
│  └───────────────────────────────────────────┘    │
│                                                   │
│  Filesystem Layout:                               │
│  /etc/init.yaml              - Init configuration │
│  /etc/initctl.yaml           - Initctl config     │
│  /service/*.service          - Service files      │
│  /service/*.service.disabled - Disabled services  │
│  /log/*.log                  - Service logs       │
│  /proc/                      - Process info       │
└───────────────────────────────────────────────────┘
```

### Communication Protocols

```
┌─────────────────────────────────────────────────────┐
│                Control Protocols                    │
└─────────────────────────────────────────────────────┘

1. Unix Socket (Local Control)
   ┌──────────┐                    ┌──────────┐
   │ initctl  │ ──[Unix Socket]──> │  init    │
   │ (Local)  │ <───────────────── │  (PID 1) │
   └──────────┘                    └──────────┘
   Path: /run/init.sock
   Use: In-enclave management
   Commands: Service + Process management

2. VSOCK (Remote Control)
   ┌──────────┐                    ┌──────────┐
   │  initctl │ ──[VSOCK]────────> │   init   │
   │  (Host)  │ <───────────────── │ (Enclave)│
   └──────────┘                    └──────────┘
   CID: 16 (enclave), PORT: 9001
   Use: Host-to-enclave management
   Commands: Service + Process management

3. Both Protocols Simultaneously
   ┌──────────┐                    ┌──────────┐
   │ initctl  │ ──[Unix Socket]──> │          │
   │ (Local)  │                    │   init   │
   │          │                    │  (PID 1) │
   │ initctl  │ ──[VSOCK]────────> │          │
   │ (Host)   │                    │          │
   └──────────┘                    └──────────┘
```

### Process Management Flow

```
┌─────────────────────────────────────────────────────┐
│              Process Management Layer               │
└─────────────────────────────────────────────────────┘

Init System View of Processes:
┌─────────────────────────────────────────────────────┐
│  All System Processes                               │
│                                                     │
│  ┌──────────────────────────────────────────┐       │
│  │  Managed Services (tracked by init)      │       │
│  │  - webapp (PID 123) → Service            │       │
│  │  - database (PID 124) → Service          │       │
│  │  - worker (PID 125) → Service            │       │
│  └──────────────────────────────────────────┘       │
│                                                     │
│  ┌────────────────────────────────────────────┐     │
│  │  Unmanaged Processes                       │     │
│  │  - bash (PID 200) → User shell             │     │
│  │  - python (PID 201) → Ad-hoc process       │     │
│  │  - grep (PID 202) → Temporary              │     │
│  └────────────────────────────────────────────┘     │
│                                                     │
│  ┌────────────────────────────────────────────┐     │
│  │  System Processes                          │     │
│  │  - init (PID 1) → Init system              │     │
│  │  - kthreadd (PID 2) → Kernel threads       │     │
│  └────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────┘

Process Operations:
- List: Show all processes with details
- Status: Get detailed info for specific PID
- Start: Launch new ad-hoc process
- Stop: Send SIGTERM to process
- Kill: Send specific signal to process
- Restart: Restart managed service by PID
```

### Process Flow

```
┌──────────────┐
│ Init Startup │
└──────┬───────┘
       │
       ├─> Parse CLI Arguments (--config)
       ├─> Load Configuration (from file or env)
       ├─> Setup Signal Handlers (TERM/INT/HUP/CHLD)
       ├─> Initialize Filesystems
       ├─> Load NSM Driver (optional)
       ├─> Send VSOCK Heartbeat (optional)
       ├─> Perform Pivot Root (optional)
       ├─> Load Service Definitions
       ├─> Compute Dependency Order
       ├─> Validate Dependencies
       ├─> Start Enabled Services (in order)
       ├─> Start Unix Socket Server (if enabled)
       ├─> Start VSOCK Socket Server (if enabled)
       │
       └─> ┌────────────────┐
           │   Main Loop    │
           └────────┬───────┘
                    │
                    ├─> Check SIGCHLD → Reap Children
                    ├─> Check SIGTERM/SIGINT → Shutdown
                    ├─> Check SIGHUP → Reload Services
                    ├─> Restart Dead Services (per policy)
                    ├─> Handle Unix Socket Requests
                    │   ├─ Service commands
                    │   └─ Process commands
                    ├─> Handle VSOCK Requests
                    │   ├─ Service commands
                    │   └─ Process commands
                    │
                    └─> [Loop continues]
```

---

## Installation

### Prerequisites

- Rust 1.91.0 or later
- Linux operating system (designed for enclaves)
- Standard build tools (gcc, make, etc.)
- VSOCK kernel module (for VSOCK support)

### Building from Source

```bash
# Clone the repository
git clone https://github.com/sentient-agi/Sentient-Enclaves-Framework.git
cd Sentient-Enclaves-Framework/enclave-init/

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

# Create configuration files
touch /path/to/enclave/rootfs/etc/init.yaml
touch /path/to/enclave/rootfs/etc/initctl.yaml
```

### Installation on Host (for Remote Management)

```bash
# Copy initctl to host
cp target/release/initctl /usr/local/bin/initctl-enclave

# Create host configuration
mkdir -p /etc/enclave-init
cat > /etc/enclave-init/initctl.yaml << EOF
protocol: vsock
vsock_cid: 16  # Your enclave CID
vsock_port: 9001
EOF
```

---

## Quick Start

### 1. Create Init Configuration File

Create `/etc/init.yaml`:

```yaml
service_dir: /service
log_dir: /log

# Control socket configuration
control:
  # Unix socket (local control)
  unix_socket_enabled: true
  unix_socket_path: /run/init.sock

  # VSOCK (remote control from host)
  vsock_enabled: true
  vsock_cid: 4294967295  # VMADDR_CID_ANY (-1U)
  vsock_port: 9001

max_log_size: 10485760  # 10 MB
max_log_files: 5

environment:
  TZ: UTC
  LANG: en_US.UTF-8

# VSOCK heartbeat (different from control socket)
vsock:
  enabled: true
  cid: 3      # Parent CID
  port: 9000  # Heartbeat port

pivot_root: true
pivot_root_dir: /rootfs
```

### 2. Create Initctl Configuration File

**Inside Enclave** (`/etc/initctl.yaml`):

```yaml
# Use Unix socket for local control
protocol: unix
unix_socket_path: /run/init.sock
```

**On Host** (`/etc/initctl.yaml` or `/etc/enclave-init/initctl.yaml`):

```yaml
# Use VSOCK for remote control
protocol: vsock
vsock_cid: 16      # Enclave CID
vsock_port: 9001   # Control port
```

### 3. Create Service Files with Dependencies

Create `/service/database.service`:

```toml
ExecStart = "/usr/bin/postgres -D /var/lib/postgresql/data"
Environment = [
    "POSTGRES_PASSWORD=secret",
    "POSTGRES_DB=myapp"
]
Restart = "always"
RestartSec = 10
WorkingDirectory = "/var/lib/postgresql"
ServiceEnable = true

# This service should start before webapp
Before = ["webapp"]
```

Create `/service/webapp.service`:

```toml
ExecStart = "/usr/bin/python3 /app/server.py"
Environment = [
    "PORT=8080",
    "DATABASE_URL=postgresql://localhost/myapp",
    "LOG_LEVEL=info"
]
Restart = "always"
RestartSec = 5
WorkingDirectory = "/app"
ServiceEnable = true

# Start after database and require it
After = ["database"]
Requires = ["database"]
```

### 4. Start Init System

The init system starts automatically as PID 1 when the enclave boots:

```bash
# Inside the enclave, init is already running as PID 1
ps aux | grep init
# root         1  0.0  0.1  12345  6789 ?        Ss   00:00   0:00 /sbin/init

# With custom config path
/sbin/init --config /etc/my-init.yaml
# or via environment
INIT_CONFIG=/etc/my-init.yaml /sbin/init
```

### 5. Manage Services and Processes

**Inside Enclave (Unix Socket)**:

```bash
# Service management
initctl list
initctl status webapp
initctl logs webapp -n 100
initctl restart webapp

# Process management
initctl ps list
initctl ps status 123
initctl ps start /usr/bin/mycommand arg1 arg2
initctl ps stop 456
initctl ps kill 789 --signal 9
```

**From Host (VSOCK)**:

```bash
# Configure initctl to use VSOCK
export INITCTL_CONFIG=/etc/enclave-init/initctl.yaml

# Or use CLI options
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 list

# Service management from host
initctl status webapp
initctl restart webapp
initctl logs webapp -n 100

# Process management from host
initctl ps list
initctl ps status 123
initctl ps stop 456
```

---

## Configuration Reference

### Init Configuration (`init.yaml`)

The main configuration file for the init system.

#### Location

- Default: `/etc/init.yaml`
- Configurable via: `--config` flag or `INIT_CONFIG` environment variable
- If not found, uses built-in defaults

#### Complete Example

```yaml
# Service directory containing .service files
service_dir: /service

# Directory for service log files
log_dir: /log

# Control socket configuration
control:
  # Unix socket control interface
  unix_socket_enabled: true
  unix_socket_path: /run/init.sock

  # VSOCK control interface
  vsock_enabled: true
  # VMADDR_CID_ANY (4294967295 or -1U) listens on any CID
  # Or specify enclave's own CID (usually auto-assigned)
  vsock_cid: 4294967295
  vsock_port: 9001

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
  ENVIRONMENT: production

# VSOCK heartbeat configuration (for enclave readiness)
# This is separate from the control socket
vsock:
  # Enable VSOCK heartbeat to parent
  enabled: true
  # Parent CID (usually 3)
  cid: 3
  # Heartbeat port
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
| `control.unix_socket_enabled` | boolean | `true` | Enable Unix socket control interface |
| `control.unix_socket_path` | string | `/run/init.sock` | Unix domain socket path for local control |
| `control.vsock_enabled` | boolean | `false` | Enable VSOCK control interface |
| `control.vsock_cid` | integer | `3` | VSOCK CID to bind (use 4294967295 for ANY) |
| `control.vsock_port` | integer | `9001` | VSOCK port for control interface |
| `max_log_size` | integer | `10485760` | Maximum log file size in bytes before rotation |
| `max_log_files` | integer | `5` | Number of rotated log files to retain |
| `environment` | map | `{}` | Key-value pairs of environment variables |
| `vsock.enabled` | boolean | `true` | Enable VSOCK heartbeat to host |
| `vsock.cid` | integer | `3` | VSOCK CID for heartbeat (parent) |
| `vsock.port` | integer | `9000` | VSOCK port for heartbeat |
| `nsm_driver_path` | string/null | `"nsm.ko"` | Path to NSM driver or null to disable |
| `pivot_root` | boolean | `true` | Perform pivot root operation on startup |
| `pivot_root_dir` | string | `/rootfs` | Source directory for pivot root |

---

### Initctl Configuration (`initctl.yaml`)

Configuration file for the `initctl` CLI tool to specify how to connect to init.

#### Location

- Default: `/etc/initctl.yaml`
- Configurable via: `--config` flag or `INITCTL_CONFIG` environment variable
- If not found, uses built-in defaults (Unix socket)

#### Complete Example

**For In-Enclave Use**:
```yaml
# Protocol to use: "unix" or "vsock"
protocol: unix

# Unix socket configuration
unix_socket_path: /run/init.sock

# VSOCK configuration (not used when protocol is unix)
vsock_cid: 3
vsock_port: 9001
```

**For Host-to-Enclave Use**:
```yaml
# Protocol to use for host-to-enclave control
protocol: vsock

# Unix socket (not used when protocol is vsock)
unix_socket_path: /run/init.sock

# VSOCK configuration
vsock_cid: 16      # Enclave CID (assigned by hypervisor)
vsock_port: 9001   # Control port
```

#### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `protocol` | string | `unix` | Protocol to use: `unix` or `vsock` |
| `unix_socket_path` | string | `/run/init.sock` | Unix socket path (when protocol is unix) |
| `vsock_cid` | integer | `3` | VSOCK CID to connect to (when protocol is vsock) |
| `vsock_port` | integer | `9001` | VSOCK port to connect to (when protocol is vsock) |

---

## Service Files

Service files define how individual services should be run and managed.

### File Format

- **Format**: TOML
- **Extension**: `.service` (active) or `.service.disabled` (disabled)
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

# Enable the service (optional, default: true)
ServiceEnable = true

# Dependencies: Services that should start before this one
After = ["database", "cache"]

# Dependencies: Services that should start after this one
Before = ["monitor"]

# Hard dependencies: Required services (must exist and start)
Requires = ["database"]

# Reverse dependency specification
RequiredBy = ["monitor"]
```

### Service Configuration Options

| Option | Type | Required | Default | Description |
|--------|------|----------|---------|-------------|
| `ExecStart` | string | **Yes** | - | Command line to execute |
| `Environment` | array | No | `[]` | List of environment variables |
| `Restart` | string | No | `"no"` | Restart policy |
| `RestartSec` | integer | No | `5` | Seconds to wait before restart |
| `WorkingDirectory` | string | No | - | Working directory for the process |
| `ServiceEnable` | boolean | No | `true` | Enable service at startup |
| `Before` | array | No | `[]` | Services that should start after this |
| `After` | array | No | `[]` | Services that should start before this |
| `Requires` | array | No | `[]` | Required dependencies |
| `RequiredBy` | array | No | `[]` | Services that require this one |

---

## Service Dependencies

The init system supports systemd-style dependency management with automatic ordering.

### Dependency Types

#### `After`

Specifies services that should start **before** this service.

```toml
# webapp starts after database
[webapp.service]
After = ["database"]
```

**Behavior**:
- Soft dependency (database doesn't need to exist)
- If database exists, it starts first
- If database fails, webapp still starts
- Ordering only, not a hard requirement

#### `Before`

Specifies services that should start **after** this service.

```toml
# database starts before webapp
[database.service]
Before = ["webapp"]
```

#### `Requires`

Hard dependency. The required service must exist and start successfully.

```toml
# webapp requires database
[webapp.service]
Requires = ["database"]
After = ["database"]  # Usually combined with After
```

#### `RequiredBy`

Reverse of `Requires`. This service is required by others.

```toml
# database is required by webapp
[database.service]
RequiredBy = ["webapp"]
```

---

## Control Protocols

The init system supports two control protocols for managing services and processes.

### Unix Socket Protocol

**Purpose**: Local control within the enclave

**Configuration** (in `/etc/init.yaml`):
```yaml
control:
  unix_socket_enabled: true
  unix_socket_path: /run/init.sock
```

**Use Cases**:
- In-enclave service management
- In-enclave process management
- Local automation scripts
- Container-like environments

### VSOCK Protocol

**Purpose**: Remote control from host to enclave

**Configuration** (in `/etc/init.yaml`):
```yaml
control:
  vsock_enabled: true
  vsock_cid: 4294967295  # VMADDR_CID_ANY
  vsock_port: 9001
```

**Use Cases**:
- Host-to-enclave management
- Remote service control
- Remote process monitoring
- CI/CD pipelines managing enclave services
- Zero-downtime deployments

---

## Process Management

The init system provides comprehensive process management capabilities, allowing you to list, monitor, and control all processes in the system.

### Process Information

The init system tracks and displays the following information for each process:

| Field | Description |
|-------|-------------|
| **PID** | Process ID |
| **PPID** | Parent Process ID |
| **Name** | Process name (from /proc/[pid]/stat) |
| **Command** | Full command line with arguments |
| **State** | Process state (Running, Sleeping, Zombie, etc.) |
| **CPU%** | CPU usage percentage |
| **Memory** | Memory usage (RSS) in KB/MB/GB |
| **Start Time** | Process start time |
| **Managed** | Whether process is managed by init as a service |
| **Service** | Service name if managed |

### Process States

| State | Description |
|-------|-------------|
| Running | Currently executing |
| Sleeping | Interruptible sleep (waiting for event) |
| Disk sleep | Uninterruptible sleep (usually I/O) |
| Zombie | Terminated but not yet reaped |
| Stopped | Stopped by signal |
| Tracing | Being traced (e.g., by debugger) |
| Dead | Process is dead |
| Idle | Kernel idle thread |

### Process Management Commands

#### List All Processes

```bash
initctl ps list
```

**Output**:
```
PID      PPID     STATE        CPU%   MEM      MANAGED    SERVICE    COMMAND
----------------------------------------------------------------------------------------------------
1        0        Sleeping     0.1    12.5M    no         -          /sbin/init
123      1        Running      2.5    256M     yes        webapp     /usr/bin/python3 /app/server.py
124      1        Sleeping     1.2    512M     yes        database   /usr/bin/postgres -D /var/lib/p...
125      1        Sleeping     0.8    128M     yes        worker     /usr/bin/celery worker
200      1        Sleeping     0.0    8.2M     no         -          bash
201      200      Running      5.2    64M      no         -          python script.py
```

#### Get Process Status

```bash
initctl ps status <PID>
```

**Example**:
```bash
initctl ps status 123
```

**Output**:
```
Process: 123
  Name: python3
  Parent PID: 1
  State: Running
  Command: /usr/bin/python3 /app/server.py
  CPU: 2.5%
  Memory: 256M
  Start Time: 1234567890
  Managed by Init: yes
  Service: webapp
```

#### Start a Process

Start a new ad-hoc process (not managed as a service):

```bash
initctl ps start <COMMAND> [ARGS...] [--env KEY=VALUE]
```

**Examples**:
```bash
# Simple command
initctl ps start /usr/bin/python3 script.py

# Alias: run
initctl ps run /usr/bin/python3 script.py

# With arguments
initctl ps start /usr/bin/myapp --config /etc/myapp.conf --verbose

# With environment variables
initctl ps start /usr/bin/myapp -e LOG_LEVEL=debug -e PORT=8080
```

**Output**:
```
✓ Process started with PID 789 (PID: 789)
```

**Note**: Ad-hoc processes are not managed (no restart policy, not tracked as services).

#### Stop a Process

Send SIGTERM to a process:

```bash
initctl ps stop <PID>
```

**Example**:
```bash
initctl ps stop 789
```

**Output**:
```
✓ SIGTERM sent to process 789
```

#### Restart a Process

Restart a managed service by its PID:

```bash
initctl ps restart <PID>
```

**Example**:
```bash
initctl ps restart 123
```

**Output**:
```
✓ Service 'webapp' restarted
```

**Note**: Only works for processes managed by init as services. For ad-hoc processes, you'll get an error:
```
✗ Error: Process 789 is not managed by init, cannot restart
```

#### Kill a Process with Signal

Send a specific signal to a process:

```bash
initctl ps kill <PID> --signal <SIGNAL_NUMBER>
```

**Common Signals**:
| Signal | Number | Description |
|--------|--------|-------------|
| SIGHUP | 1 | Hangup (often used to reload config) |
| SIGINT | 2 | Interrupt (Ctrl+C) |
| SIGKILL | 9 | Kill (cannot be caught or ignored) |
| SIGTERM | 15 | Terminate (default, graceful) |

**Examples**:
```bash
# Send SIGTERM (default)
initctl ps kill 789 --signal 15

# Force kill
initctl ps kill 789 --signal 9

# Send SIGHUP (reload)
initctl ps kill 789 --signal 1

# Short form
initctl ps kill 789 -s 9
```

**Output**:
```
✓ Signal 9 sent to process 789
```

### Process Management from Host

All process management commands work remotely via VSOCK:

```bash
# List processes in enclave from host
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 ps list

# Get process status
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 ps status 123

# Stop process
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 ps stop 789

# Kill process
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 ps kill 789 --signal 9
```

### Managed vs Unmanaged Processes

**Managed Processes**:
- Started as services via service files
- Have restart policies
- Tracked by init
- Show service name in process list
- Can be restarted via `initctl ps restart <PID>`

**Unmanaged Processes**:
- Started ad-hoc via `initctl ps start`
- No restart policy
- Not tracked as services
- Show "-" for service name
- Cannot be restarted (only stopped/killed)

### Process Monitoring Example

Monitor processes in real-time:

```bash
#!/bin/bash
# monitor-processes.sh

while true; do
    clear
    echo "=== Enclave Processes - $(date) ==="
    initctl ps list
    sleep 5
done
```

Remote monitoring from host:

```bash
#!/bin/bash
# host-monitor.sh

INITCTL="initctl --protocol vsock --vsock-cid 16 --vsock-port 9001"

while true; do
    clear
    echo "=== Enclave Processes - $(date) ==="
    $INITCTL ps list
    echo ""
    echo "=== Services ==="
    $INITCTL list
    sleep 5
done
```

---

## Logs Streaming

Logs Streaming Feature for Enclave Apps Remote Debugging

### Summary of Changes

This implementation adds **real-time log streaming over VSock** from enclave services to the host. The feature enables operators to monitor service logs in real-time without polling, with logs flowing directly from inside the enclave VM to a listener on the host machine.

### Files Modified

| File | Changes |
|------|---------|
| `protocol.rs` | Added `ServiceLogsStream` and `ServiceLogsStreamStop` requests, `LogsStreamStarted` response |
| `logger.rs` | Added `LogSubscriber` trait and subscriber management to `ServiceLogger` |
| `main.rs` | Added `VsockLogStreamer` struct, `StreamerMap` type, streaming request handlers |
| `initctl.rs` | Added `LogsStream` command with VSock listener functionality |
| `Cargo.toml` | Added `ctrlc` dependency for graceful interrupt handling |

---

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              HOST SYSTEM                                │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                    initctl logs-stream                          │    │
│  │                                                                 │    │
│  │  1. Creates VSock listener on CID:2 PORT:9100                   │    │
│  │  2. Sends ServiceLogsStream request to enclave init             │    │
│  │  3. Accepts connection from enclave                             │    │
│  │  4. Receives log lines in real-time                             │    │
│  │  5. Outputs to stdout or file                                   │    │
│  └───────────────────────────┬─────────────────────────────────────┘    │
│                              │                                          │
│                              │ VSock Listener (CID:2 PORT:9100)         │
│                              ▼                                          │
│                        ┌───────────┐                                    │
│                        │  Accept   │ ◄── Incoming connection            │
│                        │  Logs     │     from enclave                   │
│                        └─────┬─────┘                                    │
│                              │                                          │
│         Control Request      │     Log Data Stream                      │
│      (ServiceLogsStream)     │     (newline-terminated)                 │
│              │               │                                          │
│              ▼               │                                          │
│  ┌─────────────────┐         │                                          │
│  │ VSOCK Control   │         │                                          │
│  │ CID:16 PORT:9001│         │                                          │
│  └────────┬────────┘         │                                          │
│           │                  │                                          │
└───────────┼──────────────────┼──────────────────────────────────────────┘
            │                  │
            │ VSock            │ VSock Log Stream
            │ Control          │
            ▼                  │
┌──────────────────────────────┼──────────────────────────────────────────┐
│           ENCLAVE VM         │                                          │
│                              │                                          │
│  ┌───────────────────────────┼──────────────────────────────────────┐   │
│  │                   init (PID 1)                                   │   │
│  │                           │                                      │   │
│  │  ┌────────────────────────┴─────────────────────────────────┐    │   │
│  │  │              ServiceLogsStream Handler                   │    │   │
│  │  │                                                          │    │   │
│  │  │  1. Receives request with target CID:2 PORT:9100         │    │   │
│  │  │  2. Creates VsockLogStreamer                             │    │   │
│  │  │  3. Connects to host listener                            │    │   │
│  │  │  4. Subscribes streamer to ServiceLogger                 │    │   │
│  │  │  5. Returns LogsStreamStarted response                   │    │   │
│  │  └──────────────────────────────────────────────────────────┘    │   │
│  │                                                                  │   │
│  │  ┌───────────────────────────────────────────────────────────┐   │   │
│  │  │                   VsockLogStreamer                        │   │   │
│  │  │                                                           │   │   │
│  │  │  - Implements LogSubscriber trait                         │   │   │
│  │  │  - Holds VSock connection to host                         │──────►
│  │  │  - on_log(): sends formatted line to host                 │   │   │
│  │  │  - is_active(): checks connection status                  │   │   │
│  │  └───────────────────────────────────────────────────────────┘   │   │
│  │                              ▲                                   │   │
│  │                              │ subscribe()                       │   │
│  │                              │                                   │   │
│  │  ┌───────────────────────────┴───────────────────────────────┐   │   │
│  │  │                   ServiceLogger                           │   │   │
│  │  │                                                           │   │   │
│  │  │  - Writes to log file                                     │   │   │
│  │  │  - Keeps in-memory buffer                                 │   │   │
│  │  │  - Notifies all subscribers on each log()                 │   │   │
│  │  └───────────────────────────────────────────────────────────┘   │   │
│  │                              ▲                                   │   │
│  │                              │ log("message")                    │   │
│  │                              │                                   │   │
│  └──────────────────────────────┼───────────────────────────────────┘   │
│                                 │                                       │
│  ┌──────────────────────────────┴────────────────────────────────────┐  │
│  │                        Service (webapp)                           │  │
│  │                                                                   │  │
│  │  - Runs application                                               │  │
│  │  - Outputs to stdout/stderr                                       │  │
│  │  - Init captures and logs via ServiceLogger                       │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

### Data Flow Sequence

```
Host (initctl)                    Enclave (init)                    Service
      │                                 │                               │
      │  1. Start listener              │                               │
      │     CID:2 PORT:9100             │                               │
      │                                 │                               │
      │  2. ServiceLogsStream ─────────►│                               │
      │     {name, cid:2, port:9100}    │                               │
      │                                 │                               │
      │                                 │  3. Create VsockLogStreamer   │
      │                                 │     Connect to CID:2:9100     │
      │◄────────────────────────────────│                               │
      │  (VSock connection established) │                               │
      │                                 │                               │
      │◄─────────────────────────────── │  4. LogsStreamStarted         │
      │                                 │     response                  │
      │                                 │                               │
      │                                 │                               │  5. Service
      │                                 │                               │     outputs
      │                                 │                               │     log line
      │                                 │◄──────────────────────────────│
      │                                 │  6. ServiceLogger.log()       │
      │                                 │                               │
      │                                 │  7. Notify subscribers        │
      │                                 │     (VsockLogStreamer)        │
      │                                 │                               │
      │◄────────────────────────────────│  8. Send log line             │
      │  "[timestamp] log message\n"    │     over VSock                │
      │                                 │                               │
      │  9. Output to stdout/file       │                               │
      │                                 │                               │
     ~~~                               ~~~                             ~~~
      │                                 │                               │
      │  (Ctrl+C pressed)               │                               │
      │                                 │                               │
      │  10. ServiceLogsStreamStop ────►│                               │
      │      {name}                     │                               │
      │                                 │  11. Stop streamer            │
      │                                 │      Close connection         │
      │◄─────────────────────────────── │                               │
      │  Success response               │                               │
      │                                 │                               │
      ▼                                 ▼                               ▼
```

---

### Logs Streaming Feature CLI Reference

#### New Command: `logs-stream`

Stream logs from a service in real-time via VSock.

```bash
initctl logs-stream <SERVICE> [OPTIONS]
```

#### Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--listen-cid <CID>` | | `2` | VSock CID to listen on (VMADDR_CID_HOST) |
| `--listen-port <PORT>` | | `9100` | VSock port for receiving logs |
| `--output <PATH>` | `-o` | stdout | Output file path |
| `--follow` | `-f` | false | Keep streaming until interrupted |

---

### Logs Streaming Feature CLI Examples

#### Basic Usage - Stream to stdout

```bash
# Stream webapp logs to console (follow mode)
initctl logs-stream webapp --follow

# Short form
initctl logs-stream webapp -f
```

#### Stream to File

```bash
# Stream logs to a file
initctl logs-stream webapp --output /var/log/enclave-webapp.log --follow

# Short form
initctl logs-stream webapp -o /var/log/enclave-webapp.log -f
```

#### Custom VSock Port

```bash
# Use custom port for log streaming
initctl logs-stream webapp --listen-port 9200 --follow
```

#### Remote Control via VSock (from host to enclave)

```bash
# Full example with VSOCK protocol to enclave
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 \
    logs-stream webapp --listen-port 9100 --follow
```

#### Using Configuration File

Create `/etc/initctl.yaml` on host:

```yaml
protocol: vsock
vsock_cid: 16
vsock_port: 9001
```

Then simply:

```bash
initctl logs-stream webapp --follow
```

#### Multiple Services (separate terminals)

```bash
# Terminal 1 - Stream webapp logs
initctl logs-stream webapp --listen-port 9100 -f

# Terminal 2 - Stream database logs
initctl logs-stream database --listen-port 9101 -f

# Terminal 3 - Stream worker logs
initctl logs-stream worker --listen-port 9102 -f
```

#### Background Streaming with Output to File

```bash
# Stream in background, save to file
nohup initctl logs-stream webapp -o /var/log/webapp.log -f &

# Check the log file
tail -f /var/log/webapp.log
```

#### Integration with Log Aggregation

```bash
# Pipe to external tool (e.g., jq for JSON logs)
initctl logs-stream webapp -f | jq '.'

# Send to syslog
initctl logs-stream webapp -f | logger -t enclave-webapp

# Send to remote log collector
initctl logs-stream webapp -f | nc logserver.example.com 514
```

---

### Protocol Changes

#### New Request Types

```rust
/// Request to initialize log streaming for a service
ServiceLogsStream {
    name: String,        // Service name
    vsock_cid: u32,      // Host CID to stream to
    vsock_port: u32,     // Host port to stream to
}

/// Stop streaming logs for a service
ServiceLogsStreamStop {
    name: String,        // Service name
}
```

#### New Response Type

```rust
/// Response for successful log streaming initialization
LogsStreamStarted {
    service: String,     // Service name
    vsock_cid: u32,      // Connected CID
    vsock_port: u32,     // Connected port
}
```

---

### Logger Subscriber Pattern

The `ServiceLogger` now supports a subscriber pattern for extensible log delivery:

```rust
/// Trait for log stream subscribers
pub trait LogSubscriber: Send + Sync {
    /// Called when a new log line is available
    fn on_log(&self, line: &str);

    /// Check if subscriber is still active
    fn is_active(&self) -> bool;
}
```

This allows:
- Multiple simultaneous log consumers
- Automatic cleanup of inactive subscribers
- Easy extension for future log delivery mechanisms

---

### Configuration

#### Init Configuration (`/etc/init.yaml`)

No changes required - log streaming uses the existing control socket configuration.

#### Initctl Configuration (`/etc/initctl.yaml`)

Example for host-side configuration:

```yaml
# Use VSOCK to connect to enclave
protocol: vsock
vsock_cid: 16      # Enclave CID
vsock_port: 9001   # Control port

# Unix socket (for in-enclave use)
unix_socket_path: /run/init.sock
```

---

### Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| `Service not found` | Invalid service name | Check service exists with `initctl list` |
| `Log streaming already active` | Stream already running | Stop existing stream first or use different port |
| `Failed to connect to VSock` | Enclave can't reach host | Check VSock configuration and ports |
| `No active log stream` | Stopping non-existent stream | Stream may have already stopped |

---

## CLI Reference

### `init` - Init System

Main init system binary that runs as PID 1.

#### Synopsis

```bash
init [OPTIONS]
```

#### Options

| Option | Short | Environment Variable | Default | Description |
|--------|-------|---------------------|---------|-------------|
| `--config <PATH>` | `-c` | `INIT_CONFIG` | `/etc/init.yaml` | Path to configuration file |
| `--help` | `-h` | - | - | Show help information |
| `--version` | `-V` | - | - | Show version information |

---

### `initctl` - Init Control Tool

Command-line interface for managing the init system, services, and processes.

#### Synopsis

```bash
initctl [OPTIONS] <COMMAND>
```

#### Global Options

| Option | Short | Environment Variable | Default | Description |
|--------|-------|---------------------|---------|-------------|
| `--config <PATH>` | `-c` | `INITCTL_CONFIG` | `/etc/initctl.yaml` | Path to initctl config file |
| `--protocol <PROTO>` | `-p` | - | From config | Protocol: `unix` or `vsock` |
| `--socket <PATH>` | `-s` | `INIT_SOCKET` | From config | Unix socket path |
| `--vsock-cid <CID>` | - | - | From config | VSOCK CID |
| `--vsock-port <PORT>` | - | - | From config | VSOCK port |
| `--help` | `-h` | - | - | Show help information |
| `--version` | `-V` | - | - | Show version information |

#### Commands

### Service Management Commands

#### `list`

List all services with their current status.

**Syntax:**
```bash
initctl list
```

**Output:**
```
NAME                      ENABLED    ACTIVE     RESTART         RESTARTS
---------------------------------------------------------------------------
webapp                    enabled    active     always          3
database                  enabled    active     always          1
worker                    enabled    inactive   on-failure      0
```

---

#### `status`

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
  Enabled: yes
  Status: active (running)
  PID: 1234
  Command: /usr/bin/python3 /app/server.py
  Working Directory: /app
  Restart Policy: always
  Restart Delay: 5s
  Restart Count: 3
  Last Exit Code: 0
  After: database, cache
  Requires: database
  Before: monitor
```

---

#### `start`

Start a stopped service.

**Syntax:**
```bash
initctl start <SERVICE>
```

---

#### `stop`

Stop a running service.

**Syntax:**
```bash
initctl stop <SERVICE>
```

---

#### `restart`

Restart a service.

**Syntax:**
```bash
initctl restart <SERVICE>
```

---

#### `enable`

Enable a disabled service.

**Syntax:**
```bash
initctl enable [OPTIONS] <SERVICE>
```

**Options:**
| Option | Description |
|--------|-------------|
| `--now` | Start the service immediately after enabling |

**Examples:**
```bash
initctl enable webapp
initctl enable --now webapp
```

---

#### `disable`

Disable a service.

**Syntax:**
```bash
initctl disable <SERVICE>
```

---

#### `logs`

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
initctl logs webapp
initctl logs webapp -n 100
```

---

#### `logs-clear`

Clear all logs for a service.

**Syntax:**
```bash
initctl logs-clear <SERVICE>
```

---

### Process Management Commands

#### `ps list`

List all processes in the system.

**Syntax:**
```bash
initctl ps list
```

**Output:**
```
PID      PPID     STATE        CPU%   MEM      MANAGED    SERVICE    COMMAND
----------------------------------------------------------------------------------------------------
1        0        Sleeping     0.1    12.5M    no         -          /sbin/init
123      1        Running      2.5    256M     yes        webapp     /usr/bin/python3 /app/server.py
124      1        Sleeping     1.2    512M     yes        database   /usr/bin/postgres -D /var/lib/...
```

---

#### `ps status`

Show status of a specific process.

**Syntax:**
```bash
initctl ps status <PID>
```

**Example:**
```bash
initctl ps status 123
```

**Output:**
```
Process: 123
  Name: python3
  Parent PID: 1
  State: Running
  Command: /usr/bin/python3 /app/server.py
  CPU: 2.5%
  Memory: 256M
  Start Time: 1234567890
  Managed by Init: yes
  Service: webapp
```

---

#### `ps start` / `ps run`

Start a new ad-hoc process.

**Syntax:**
```bash
initctl ps start <COMMAND> [ARGS...] [OPTIONS]
initctl ps run <COMMAND> [ARGS...]  # Alias
```

**Options:**
| Option | Short | Description |
|--------|-------|-------------|
| `--env <KEY=VALUE>` | `-e` | Set environment variable |

**Examples:**
```bash
# Simple command
initctl ps start /usr/bin/python3 script.py

# With arguments
initctl ps start /usr/bin/myapp --config /etc/myapp.conf

# With environment
initctl ps start /usr/bin/myapp -e LOG_LEVEL=debug -e PORT=8080

# Using run alias
initctl ps run /bin/bash script.sh
```

---

#### `ps stop`

Stop a process (send SIGTERM).

**Syntax:**
```bash
initctl ps stop <PID>
```

**Example:**
```bash
initctl ps stop 789
```

---

#### `ps restart`

Restart a managed service by PID.

**Syntax:**
```bash
initctl ps restart <PID>
```

**Example:**
```bash
initctl ps restart 123
```

**Note**: Only works for processes managed as services.

---

#### `ps kill`

Send a signal to a process.

**Syntax:**
```bash
initctl ps kill <PID> [OPTIONS]
```

**Options:**
| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--signal <N>` | `-s` | `15` | Signal number to send |

**Examples:**
```bash
# Send SIGTERM (default)
initctl ps kill 789

# Force kill
initctl ps kill 789 --signal 9

# Send SIGHUP
initctl ps kill 789 -s 1
```

---

### System Management Commands

#### `system-status`

Show overall system status and statistics.

**Syntax:**
```bash
initctl system-status
```

**Output:**
```
System Status
  Uptime: 2d 5h 32m 15s
  Services: 12 total, 10 enabled, 9 active
  Processes: 45 total
  Service Directory: /service
  Log Directory: /log
```

---

#### `reload`

Reload service configurations.

**Syntax:**
```bash
initctl reload
```

---

#### `reboot`

Reboot the system.

**Syntax:**
```bash
initctl reboot
```

---

#### `shutdown`

Shutdown the system.

**Syntax:**
```bash
initctl shutdown
```

---

#### `ping`

Test connectivity to init.

**Syntax:**
```bash
initctl ping
```

**Output:**
```
✓ Pong - init system is responsive
```

---

## Usage Guide

### Basic Operations

#### Managing Services Locally

```bash
# List services
initctl list

# Start/stop/restart
initctl start myapp
initctl stop myapp
initctl restart myapp

# Enable/disable
initctl enable myapp
initctl disable myapp

# View logs
initctl logs myapp -n 100
```

#### Managing Processes Locally

```bash
# List all processes
initctl ps list

# Get process info
initctl ps status 123

# Start ad-hoc process
initctl ps start /usr/bin/python3 script.py

# Stop process
initctl ps stop 789

# Kill process
initctl ps kill 789 --signal 9
```

#### Managing from Host via VSOCK

```bash
# Setup
export INITCTL_CONFIG=/etc/enclave-init/initctl.yaml

# Service management
initctl list
initctl status webapp
initctl restart webapp

# Process management
initctl ps list
initctl ps status 123
initctl ps stop 789
```

### Advanced Operations

#### Remote Process Monitoring

```bash
#!/bin/bash
# monitor-enclave.sh - Monitor enclave from host

INITCTL="initctl --protocol vsock --vsock-cid 16 --vsock-port 9001"

echo "=== Services ==="
$INITCTL list

echo ""
echo "=== Top Processes by CPU ==="
$INITCTL ps list | sort -k4 -rn | head -10

echo ""
echo "=== Top Processes by Memory ==="
$INITCTL ps list | sort -k5 -rn | head -10
```

#### Kill Runaway Processes

```bash
#!/bin/bash
# kill-high-cpu.sh - Kill processes using >90% CPU

INITCTL="initctl"
THRESHOLD=90.0

$INITCTL ps list | tail -n +3 | while read line; do
    PID=$(echo $line | awk '{print $1}')
    CPU=$(echo $line | awk '{print $4}')
    MANAGED=$(echo $line | awk '{print $6}')

    # Use bc for floating point comparison
    if (( $(echo "$CPU > $THRESHOLD" | bc -l) )); then
        if [ "$MANAGED" = "no" ]; then
            echo "Killing high CPU process: PID=$PID CPU=$CPU%"
            $INITCTL ps kill $PID --signal 9
        else
            echo "High CPU managed process: PID=$PID CPU=$CPU% (skipping)"
        fi
    fi
done
```

#### Start Debugging Session

```bash
# Start interactive shell in enclave
initctl ps start /bin/bash

# Or from host
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 ps start /bin/bash
```

#### Process Tree View

```bash
#!/bin/bash
# Show process tree

initctl ps list | awk '
BEGIN {
    print "Process Tree:"
}
NR > 2 {
    pid = $1
    ppid = $2
    cmd = ""
    for (i=8; i<=NF; i++) cmd = cmd $i " "
    processes[pid] = sprintf("  PID %s: %s", pid, cmd)
    children[ppid] = children[ppid] " " pid
}
END {
    # Print tree starting from PID 1
    print_tree(1, 0)
}

function print_tree(pid, level) {
    if (processes[pid]) {
        for (i=0; i<level; i++) printf "  "
        print processes[pid]
    }
    split(children[pid], kids, " ")
    for (k in kids) {
        if (kids[k] != "") print_tree(kids[k], level+1)
    }
}
'
```

---

## Advanced Topics

### Process Information Sources

The init system gathers process information from `/proc`:

#### `/proc/[pid]/stat`
- Process name
- State
- Parent PID
- CPU time (user + system)
- Start time

#### `/proc/[pid]/cmdline`
- Full command line with arguments

#### `/proc/[pid]/status`
- Memory usage (VmRSS)

### CPU Percentage Calculation

CPU percentage is calculated as:

```
cpu_percent = (total_cpu_time * 100) / (elapsed_time * HZ)
```

Where:
- `total_cpu_time` = user time + system time (from /proc/[pid]/stat)
- `elapsed_time` = system uptime - process start time
- `HZ` = kernel CONFIG_HZ (usually 100)

### Process States in /proc

| Code | State | Description |
|------|-------|-------------|
| R | Running | Currently executing or runnable |
| S | Sleeping | Interruptible sleep |
| D | Disk sleep | Uninterruptible sleep |
| Z | Zombie | Terminated, waiting for parent |
| T | Stopped | Stopped by signal |
| t | Tracing | Being traced |
| X/x | Dead | Process is dead |
| K | Wakekill | Wakekill state |
| W | Waking | Waking state |
| P | Parked | Parked state |
| I | Idle | Idle kernel thread |

### Signal Handling

**Init Process** (PID 1):
- Blocks all signals except SIGCHLD
- Handles SIGTERM/SIGINT for shutdown
- Handles SIGHUP for reload
- Handles SIGCHLD for reaping children

**Child Processes**:
- All signals unblocked
- Can handle signals normally
- Terminated children are reaped by init

**Process Management Signals**:
```rust
Signal::SIGHUP  => 1   // Hangup
Signal::SIGINT  => 2   // Interrupt
Signal::SIGKILL => 9   // Kill (uncatchable)
Signal::SIGTERM => 15  // Terminate (graceful)
```

### VSOCK Integration Details

**VSOCK CID Values**:
```rust
const VMADDR_CID_HYPERVISOR: u32 = 0;
const VMADDR_CID_LOCAL: u32 = 1;
const VMADDR_CID_HOST: u32 = 2;
const VMADDR_CID_ANY: u32 = 4294967295;  // -1U
```

**Finding Enclave CID**:
```bash
# Method 1: From host
nitro-cli describe-enclaves | jq '.[] | .EnclaveCID'

# Method 2: From enclave start
nitro-cli run-enclave --eif-path app.eif | jq '.EnclaveCID'
```

---

## Troubleshooting

### Process Management Issues

#### Cannot List Processes

**Symptom**: `initctl ps list` fails or shows no processes

**Solutions**:

1. Check if /proc is mounted:
```bash
mount | grep proc
```

2. Verify permissions:
```bash
ls -ld /proc
# Should be: dr-xr-xr-x
```

3. Check init is running:
```bash
ps aux | grep init
```

#### Process Status Shows Wrong Info

**Symptom**: CPU or memory values seem incorrect

**Possible Causes**:
- /proc filesystem not properly mounted
- Process state changing rapidly
- Kernel HZ value different than expected (100)

**Debug**:
```bash
# Check HZ value
getconf CLK_TCK

# Manually check process
cat /proc/123/stat
cat /proc/123/status
```

#### Cannot Start Process

**Symptom**: `initctl ps start` fails

**Solutions**:

1. Check command exists:
```bash
which mycommand
```

2. Check permissions:
```bash
ls -l /usr/bin/mycommand
```

3. Try running directly:
```bash
/usr/bin/mycommand
```

4. Check logs:
```bash
# Init system logs any fork/exec failures
dmesg | grep init
```

#### Cannot Kill Process

**Symptom**: `initctl ps kill` fails

**Solutions**:

1. Verify process exists:
```bash
initctl ps status <PID>
```

2. Check if process is in uninterruptible sleep:
```bash
initctl ps status <PID> | grep State
# If "Disk sleep", process cannot be killed until I/O completes
```

3. Try SIGKILL:
```bash
initctl ps kill <PID> --signal 9
```

#### Process Shows as Zombie

**Symptom**: Process state is "Zombie"

**Explanation**: Process has exited but not yet been reaped by parent.

**Normally**: Init automatically reaps zombies via SIGCHLD handler.

**If persists**:
```bash
# Check if parent is still alive
initctl ps status <ZOMBIE_PID>
# Note the PPID

# If parent is init (PID 1), zombie should be reaped automatically
# If zombie persists, init may be busy - wait a moment
```

### Remote Management Issues

#### Cannot Control Processes from Host

**Symptom**: Process commands from host fail

**Checklist**:
1. ✓ VSOCK enabled in init config
2. ✓ Correct enclave CID
3. ✓ Correct port (9001)
4. ✓ Enclave is running
5. ✓ initctl configured for VSOCK

**Test connectivity**:
```bash
# Test with ping
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 ping

# Test service list
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 list

# Test process list
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 ps list
```

---

## Development

### Building from Source

```bash
# Clone repository
git clone https://github.com/sentient-agi/Sentient-Enclaves-Framework.git
cd Sentient-Enclaves-Framework/enclave-init/

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
├── Cargo.toml                  # Rust dependencies and build config
├── src/
│   ├── main.rs                # Init system (PID 1)
│   ├── initctl.rs             # CLI control tool
│   ├── protocol.rs            # IPC protocol definitions
│   ├── config.rs              # Configuration loading
│   ├── logger.rs              # Logging implementation
│   ├── dependencies.rs        # Dependency resolution
│   └── process.rs             # Process management
├── examples/
│   ├── init.yaml              # Example init configuration
│   ├── initctl.yaml           # Initctl config examples
│   └── services/              # Example service files
├── tests/
│   └── integration/           # Integration tests
└── docs/
    └── README.md              # This file
```

### Testing Process Management

```bash
# Unit tests
cargo test process

# Integration test - list processes
cargo run --bin initctl -- ps list

# Integration test - start process
cargo run --bin initctl -- ps start /bin/sleep 10

# Test process killing
cargo run --bin initctl -- ps start /bin/sleep 100
# Get PID from output
cargo run --bin initctl -- ps kill <PID> --signal 15
```

---

## FAQ

### Process Management Questions

**Q: Can I manage processes that weren't started by init?**

A: Yes! `initctl ps list` shows all processes in the system, regardless of how they were started. You can stop, kill, or get status for any process.

**Q: What's the difference between managed and unmanaged processes?**

A:
- **Managed**: Started as services, have restart policies, tracked by init
- **Unmanaged**: All other processes (ad-hoc starts, user shells, etc.)

**Q: Can I restart unmanaged processes?**

A: No. `initctl ps restart` only works for processes managed as services. For unmanaged processes, you need to stop and start them manually.

**Q: Why does `initctl ps list` show system processes like `kthreadd`?**

A: The command shows ALL processes visible in `/proc`. This includes kernel threads and system processes.

**Q: How accurate are the CPU and memory values?**

A: They're snapshots from `/proc` at the time of the request. For precise monitoring, use dedicated tools or poll regularly.

**Q: Can I start services via `initctl ps start`?**

A: No. Use `initctl start <service>` for services. `initctl ps start` is for ad-hoc processes that aren't managed as services.

**Q: What happens to processes when I reboot?**

A: All processes are terminated (SIGTERM, then SIGKILL after timeout), then the system reboots.

**Q: Can I send custom signals via VSOCK?**

A: Yes! All process management commands work over VSOCK:
```bash
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 ps kill <PID> -s 9
```

**Q: Why can't I kill PID 1?**

A: PID 1 (init) has special protections in the kernel. It ignores most signals. Use `initctl shutdown` or `initctl reboot` instead.

**Q: How do I find the PID of a service?**

A: Use `initctl status <service>` which shows the PID, or use `initctl ps list` and look for the service name.

**Q: Can I start graphical applications?**

A: In an enclave environment, typically not. Enclaves usually don't have graphics. But you can start any command-line application.

---

## Appendix

### Complete Configuration Examples

#### Production Init Configuration

```yaml
# /etc/init.yaml - Production setup
service_dir: /service
log_dir: /var/log/services

# Enable both local and remote control
control:
  unix_socket_enabled: true
  unix_socket_path: /run/init.sock

  vsock_enabled: true
  vsock_cid: 4294967295  # VMADDR_CID_ANY
  vsock_port: 9001

max_log_size: 52428800    # 50 MB
max_log_files: 10

environment:
  TZ: UTC
  LANG: en_US.UTF-8
  ENVIRONMENT: production

# Heartbeat configuration
vsock:
  enabled: true
  cid: 3
  port: 9000

nsm_driver_path: /lib/modules/nsm.ko
pivot_root: true
pivot_root_dir: /rootfs
```

#### Host Initctl Configuration

```yaml
# /etc/enclave-init/initctl.yaml - Host configuration
protocol: vsock
unix_socket_path: /run/init.sock
vsock_cid: 16
vsock_port: 9001
```

### Command Reference Summary

#### Init Commands
```bash
init                                    # Default config
init --config /path/to/init.yaml       # Custom config
INIT_CONFIG=/path/to/init.yaml init    # Via environment
```

#### Service Management (Local)
```bash
initctl list                           # List services
initctl status <service>               # Service details
initctl start <service>                # Start service
initctl stop <service>                 # Stop service
initctl restart <service>              # Restart service
initctl enable <service>               # Enable service
initctl enable --now <service>         # Enable and start
initctl disable <service>              # Disable service
initctl logs <service>                 # View logs
initctl logs <service> -n 100          # View 100 lines
initctl logs-clear <service>           # Clear logs
```

#### Process Management (Local)
```bash
initctl ps list                        # List all processes
initctl ps status <PID>                # Process details
initctl ps start <cmd> [args]          # Start process
initctl ps run <cmd> [args]            # Alias for start
initctl ps stop <PID>                  # Stop process (SIGTERM)
initctl ps restart <PID>               # Restart managed service
initctl ps kill <PID> -s <N>           # Send signal
```

#### System Management
```bash
initctl system-status                  # System info
initctl reload                         # Reload configs
initctl reboot                         # Reboot system
initctl shutdown                       # Shutdown system
initctl ping                           # Test connection
```

#### Remote Management (from Host)
```bash
# Via config file
export INITCTL_CONFIG=/etc/enclave-init/initctl.yaml
initctl list
initctl ps list

# Via CLI options
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 list
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 ps list
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 ps status 123
```

### Process Information Fields

| Field | Source | Description |
|-------|--------|-------------|
| PID | /proc/[pid]/ | Process ID |
| PPID | /proc/[pid]/stat | Parent Process ID |
| Name | /proc/[pid]/stat | Process name (in parentheses) |
| Cmdline | /proc/[pid]/cmdline | Full command with args |
| State | /proc/[pid]/stat | Process state code |
| CPU% | /proc/[pid]/stat | Calculated from utime+stime |
| Memory | /proc/[pid]/status | VmRSS (Resident Set Size) |
| StartTime | /proc/[pid]/stat | Boot time + start ticks |
| Managed | Init tracking | If process is a service |
| Service | Init tracking | Service name if managed |

### Signal Reference

| Signal | Number | Description | Catchable |
|--------|--------|-------------|-----------|
| SIGHUP | 1 | Hangup | Yes |
| SIGINT | 2 | Interrupt | Yes |
| SIGQUIT | 3 | Quit | Yes |
| SIGKILL | 9 | Kill | No |
| SIGTERM | 15 | Terminate | Yes |
| SIGSTOP | 19 | Stop | No |
| SIGCONT | 18 | Continue | Yes |

### Example Scripts

#### Process Monitor Script

```bash
#!/bin/bash
# process-monitor.sh - Monitor high CPU/memory processes

THRESHOLD_CPU=80.0
THRESHOLD_MEM_MB=500

initctl ps list | tail -n +3 | while read line; do
    PID=$(echo $line | awk '{print $1}')
    CPU=$(echo $line | awk '{print $4}')
    MEM=$(echo $line | awk '{print $5}')
    MANAGED=$(echo $line | awk '{print $6}')
    SERVICE=$(echo $line | awk '{print $7}')
    CMD=$(echo $line | cut -d' ' -f8-)

    # Check CPU
    if (( $(echo "$CPU > $THRESHOLD_CPU" | bc -l) )); then
        echo "HIGH CPU: PID=$PID CPU=$CPU% MANAGED=$MANAGED SERVICE=$SERVICE"
        echo "  Command: $CMD"
    fi

    # Check memory (convert to MB for comparison)
    MEM_VAL=$(echo $MEM | sed 's/[^0-9.]//g')
    MEM_UNIT=$(echo $MEM | sed 's/[0-9.]//g')

    if [ "$MEM_UNIT" = "G" ]; then
        MEM_MB=$(echo "$MEM_VAL * 1024" | bc)
    elif [ "$MEM_UNIT" = "M" ]; then
        MEM_MB=$MEM_VAL
    else
        MEM_MB=$(echo "$MEM_VAL / 1024" | bc)
    fi

    if (( $(echo "$MEM_MB > $THRESHOLD_MEM_MB" | bc -l) )); then
        echo "HIGH MEM: PID=$PID MEM=$MEM MANAGED=$MANAGED SERVICE=$SERVICE"
        echo "  Command: $CMD"
    fi
done
```

#### Service Health Check

```bash
#!/bin/bash
# service-health.sh - Check service and process health

SERVICES=("webapp" "database" "worker")

for service in "${SERVICES[@]}"; do
    echo "Checking $service..."

    # Get service status
    STATUS=$(initctl status $service)

    # Check if active
    if echo "$STATUS" | grep -q "Status: active"; then
        # Get PID
        PID=$(echo "$STATUS" | grep "PID:" | awk '{print $2}')

        # Check process details
        PROC=$(initctl ps status $PID)
        CPU=$(echo "$PROC" | grep "CPU:" | awk '{print $2}')
        MEM=$(echo "$PROC" | grep "Memory:" | awk '{print $2}')

        echo "  ✓ Active (PID: $PID, CPU: $CPU, MEM: $MEM)"
    else
        echo "  ✗ Inactive"
    fi
done
```

---

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE-APACHE) file for details.

---

## Support

For issues, questions, or contributions:

- **Issue Tracker**: [GitHub Issues](https://github.com/sentient-agi/Sentient-Enclaves-Framework/issues)
- **Discussions**: [GitHub Discussions](https://github.com/sentient-agi/Sentient-Enclaves-Framework/discussions)
- **Documentation**: [GitHub Wiki](https://github.com/sentient-agi/Sentient-Enclaves-Framework/wiki)
- **Email**: Sentient Enclaves Team <sentient-enclaves-team@sentient.xyz>

---

## Changelog

### Version 0.8.0

**New Features:**
- Real-time log streaming over `VSock`
- `initctl logs-stream` command for continuous log monitoring
- `LogSubscriber` trait for extensible log delivery
- `VsockLogStreamer` for enclave-to-host streaming
- Automatic subscriber cleanup on disconnect
- Support for streaming to file or stdout
- Graceful interrupt handling (`Ctrl+C`)

**Protocol Changes:**
- Added `ServiceLogsStream` request
- Added `ServiceLogsStreamStop` request
- Added `LogsStreamStarted` response

### Version 0.7.0

**New Features:**
- Complete process management system
- `initctl ps list` - List all system processes
- `initctl ps status` - Get detailed process information
- `initctl ps start/run` - Start ad-hoc processes
- `initctl ps stop` - Stop processes
- `initctl ps restart` - Restart managed services by PID
- `initctl ps kill` - Send signals to processes
- Process information: PID, PPID, CPU%, memory, state, command
- Managed vs unmanaged process tracking
- Service-to-process mapping
- Remote process management via VSOCK

**Improvements:**
- Enhanced system status with total process count
- Process state parsing from /proc
- CPU and memory usage calculation
- Full command line display for processes

### Version 0.6.0

**New Features:**
- Dual protocol support: Unix socket and VSOCK
- VSOCK control interface for host-to-enclave management
- Separate initctl configuration file (`/etc/initctl.yaml`)
- Protocol selection in client (unix or vsock)
- CLI options to override protocol and connection parameters
- Simultaneous listening on both Unix socket and VSOCK
- Configurable VSOCK CID and port in init configuration
- Remote service management from host
- VMADDR_CID_ANY support for listening on any CID

**Configuration Changes:**
- Added `control` section in init.yaml with protocol settings
- New initctl.yaml configuration file for client
- `socket_path` renamed to `control.unix_socket_path`
- Added `control.vsock_enabled`, `control.vsock_cid`, `control.vsock_port`

**Breaking Changes:**
- Configuration structure changed (backward compatible with defaults)
- Socket path moved from root level to `control.unix_socket_path`

**Improvements:**
- Better separation of heartbeat and control protocols
- Enhanced remote management capabilities
- Improved documentation for VSOCK usage
- Added examples for host-to-enclave control

### Version 0.5.0 (Beta Stage Release)

**New Features:**
- Configurable init configuration file path via CLI (`--config`) and environment (`INIT_CONFIG`)
- Service dependency management (`Before`, `After`, `Requires`, `RequiredBy`)
- Automatic topological sorting for service startup order
- Circular dependency detection
- Enable/disable services at runtime
- `enable --now` to enable and start service immediately
- System reload command (`initctl reload`)
- SIGHUP signal handling for configuration reload

**Improvements:**
- Enhanced service status display with dependency information
- Better error messages for dependency issues
- Improved service loading with enabled/disabled state tracking
- Extended IPC protocol with enable/disable operations

**Bug Fixes:**
- Fixed service startup ordering issues
- Improved error handling in dependency resolution

### Version 0.4.0 (Alpha Stage Release)

**Features:**
- Init system with process supervision
- Service management with restart policies
- File-based logging with rotation
- CLI control tool (`initctl`)
- Unix socket control interface
- YAML configuration support
- VSOCK integration for Nitro Enclaves heartbeat
- NSM driver loading
- Comprehensive documentation

---
