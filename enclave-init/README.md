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
- [CLI Reference](#cli-reference)
- [Usage Guide](#usage-guide)
- [Advanced Topics](#advanced-topics)
- [Troubleshooting](#troubleshooting)
- [Development](#development)
- [FAQ](#faq)
- [Appendix](#appendix)

---

## Overview

The Enclave Init System is a minimal, production-ready init system (PID 1) designed to run inside secure enclaves. It provides process supervision, automatic service restarts, service dependency management, comprehensive logging, and dual-protocol control interfaces (Unix socket and VSOCK) for managing services at runtime.

### Key Characteristics

- **Minimal footprint**: Small binary size optimized for enclave environments
- **Reliable**: Written in Rust with comprehensive error handling
- **Non-crashing**: All errors are logged but never crash the init system
- **Service supervision**: Automatic process monitoring and restart policies
- **Dependency management**: Systemd-style service dependencies with startup ordering
- **Runtime control**: Manage services without restarting the enclave
- **Dual protocol support**: Control via Unix socket (local) or VSOCK (remote)
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

- **Service Management**
  - Systemd-style service file format (TOML)
  - Support for multiple services
  - Per-service environment variables
  - Working directory configuration
  - Restart policies: `no`, `always`, `on-failure`, `on-success`
  - Configurable restart delays
  - Enable/disable services at runtime
  - Service dependencies and ordering

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
  - CLI tool (`initctl`) for service management
  - Start, stop, restart services
  - Enable, disable services
  - Reload configurations without restart
  - View service status and logs
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
│  │  (VSOCK Client)                           │    │
│  └───────────────┬───────────────────────────┘    │
│                  │                                │
│             [VSOCK CID:16 PORT:9001]              │
└──────────────────┴────────────────────────────────┘
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
│  │  - Filesystem Initialization              │    │
│  │  - Service Management                     │    │
│  │  - Dependency Resolution                  │    │
│  │  - Unix Socket (/run/init.sock)           │    │
│  │  - VSOCK Socket (CID:ANY PORT:9001)       │    │
│  └───────────────────────────────────────────┘    │
│           │              │              │         │
│     ┌─────┴─────┐  ┌─────┴─────┐  ┌─────┴─────┐   │
│     │ Service A │  │ Service B │  │ Service C │   │
│     │(depends B)│  │           │  │(after A,B)│   │
│     └───────────┘  └───────────┘  └───────────┘   │
│                                                   │
│  ┌───────────────────────────────────────────┐    │
│  │         initctl (Enclave)                 │    │
│  │  (Unix Socket Client)                     │    │
│  └───────────────────────────────────────────┘    │
│                                                   │
│  Filesystem Layout:                               │
│  /etc/init.yaml              - Init configuration │
│  /etc/initctl.yaml           - Initctl config     │
│  /service/*.service          - Service files      │
│  /service/*.service.disabled - Disabled services  │
│  /log/*.log                  - Service logs       │
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

2. VSOCK (Remote Control)
   ┌──────────┐                    ┌──────────┐
   │ initctl  │ ──[VSOCK]────────> │   init   │
   │  (Host)  │ <───────────────── │ (Enclave)│
   └──────────┘                    └──────────┘
   CID: 16 (enclave), PORT: 9001
   Use: Host-to-enclave management

3. Both Protocols Simultaneously
   ┌──────────┐                    ┌──────────┐
   │ initctl  │ ──[Unix Socket]──> │          │
   │ (Local)  │                    │   init   │
   │          │                    │  (PID 1) │
   │ initctl  │ ──[VSOCK]────────> │          │
   │  (Host)  │                    │          │
   └──────────┘                    └──────────┘
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
                    ├─> Handle VSOCK Requests
                    │
                    └─> [Loop continues]
```

### Control Protocol Flow

```
Client Request Flow:
┌──────────────┐
│ initctl CLI  │
└──────┬───────┘
       │
       ├─> Load /etc/initctl.yaml
       ├─> Apply CLI overrides
       ├─> Determine protocol (unix/vsock)
       │
       ├─> [Unix Socket Path]
       │   │
       │   ├─> Connect to /run/init.sock
       │   ├─> Send JSON request
       │   ├─> Receive JSON response
       │   └─> Display result
       │
       └─> [VSOCK]
           │
           ├─> Connect to CID:PORT
           ├─> Send JSON request
           ├─> Receive JSON response
           └─> Display result

Server Thread (per protocol):
┌──────────────┐
│ Socket/VSOCK │
│   Listener   │
└──────┬───────┘
       │
       ├─> Accept connection
       ├─> Spawn handler thread
       │   │
       │   ├─> Receive request
       │   ├─> Parse JSON
       │   ├─> Handle request
       │   ├─> Serialize response
       │   └─> Send response
       │
       └─> [Loop for next connection]
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
git clone https://github.com/your-org/enclave-init.git
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

### 5. Manage Services

**Inside Enclave (Unix Socket)**:

```bash
# List all services
initctl list

# Check service status
initctl status webapp

# View logs
initctl logs webapp -n 100

# Restart a service
initctl restart webapp
```

**From Host (VSOCK)**:

```bash
# Configure initctl to use VSOCK
export INITCTL_CONFIG=/etc/enclave-init/initctl.yaml

# Or use CLI options
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 list

# Check service status from host
initctl status webapp

# Restart service from host
initctl restart webapp

# View logs from host
initctl logs webapp -n 100
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

#### Control Socket Configuration Details

**Unix Socket**:
- **Purpose**: Local control within enclave
- **Path**: Configurable (default: `/run/init.sock`)
- **Permissions**: Unix socket permissions apply
- **Use Case**: In-enclave service management

**VSOCK Control**:
- **Purpose**: Remote control from host
- **CID**: Use `4294967295` (VMADDR_CID_ANY) to listen on any CID
- **Port**: Configurable (default: `9001`)
- **Use Case**: Host-to-enclave service management

**Simultaneous Listening**:
```yaml
control:
  unix_socket_enabled: true
  unix_socket_path: /run/init.sock
  vsock_enabled: true
  vsock_cid: 4294967295
  vsock_port: 9001
```

#### VSOCK CID Values

| CID Value | Constant | Meaning |
|-----------|----------|---------|
| 0 | VMADDR_CID_HYPERVISOR | Hypervisor |
| 1 | VMADDR_CID_LOCAL | Local communication |
| 2 | VMADDR_CID_HOST | Host (deprecated, use 2) |
| 3+ | - | Guest/Enclave CID |
| 4294967295 | VMADDR_CID_ANY | Listen on any CID |

#### Configuration Loading Priority

1. **CLI argument**: `init --config /path/to/config.yaml` (highest priority)
2. **Environment variable**: `INIT_CONFIG=/path/to/config.yaml`
3. **Default path**: `/etc/init.yaml`
4. **Built-in defaults**: If no file found

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

#### CLI Overrides

All configuration options can be overridden via CLI arguments:

```bash
# Override protocol
initctl --protocol vsock list

# Override Unix socket path
initctl --socket /custom/path/init.sock list

# Override VSOCK parameters
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 list

# Override config file
initctl --config /path/to/initctl.yaml list
```

#### Environment Variables

| Variable | Description |
|----------|-------------|
| `INITCTL_CONFIG` | Path to initctl configuration file |
| `INIT_SOCKET` | Override Unix socket path (deprecated, use config) |

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

### Enable/Disable Mechanism

Services can be enabled or disabled in two ways:

#### 1. File Extension Method

- **Enabled**: File named `myservice.service`
- **Disabled**: File named `myservice.service.disabled`

```bash
# Disable by renaming
mv /service/myapp.service /service/myapp.service.disabled

# Enable by renaming back
mv /service/myapp.service.disabled /service/myapp.service

# Or use initctl (works locally or remotely)
initctl enable myapp
initctl disable myapp
```

#### 2. ServiceEnable Option

```toml
# In service file
ServiceEnable = true   # Enabled
ServiceEnable = false  # Disabled
```

**Note**: File extension takes precedence. A `.service.disabled` file will not be loaded regardless of `ServiceEnable` setting.

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

**Behavior**:
- Equivalent to webapp having `After = ["database"]`
- Reverse of `After`
- Multiple services can be specified

#### `Requires`

Hard dependency. The required service must exist and start successfully.

```toml
# webapp requires database
[webapp.service]
Requires = ["database"]
After = ["database"]  # Usually combined with After
```

**Behavior**:
- Database must exist (error if not)
- Database starts first (implies `After`)
- If database fails to start, webapp won't start
- Strong dependency relationship

#### `RequiredBy`

Reverse of `Requires`. This service is required by others.

```toml
# database is required by webapp
[database.service]
RequiredBy = ["webapp"]
```

**Behavior**:
- Informational/documentation purpose
- Doesn't affect startup order directly
- Useful for tracking reverse dependencies

### Dependency Resolution

#### Algorithm

The init system uses **Topological Sort (Kahn's Algorithm)** to determine startup order:

1. **Build dependency graph** from service files
2. **Validate** that all required dependencies exist
3. **Detect circular dependencies**
4. **Compute ordering** using topological sort
5. **Start services** in computed order

#### Example Dependency Chain

```toml
# redis.service
[redis.service]
ExecStart = "/usr/bin/redis-server"
Before = ["cache-warmer", "webapp"]

# cache-warmer.service
[cache-warmer.service]
ExecStart = "/usr/bin/cache-warmer"
After = ["redis"]
Before = ["webapp"]
Requires = ["redis"]

# database.service
[database.service]
ExecStart = "/usr/bin/postgres"
Before = ["webapp"]

# webapp.service
[webapp.service]
ExecStart = "/usr/bin/webapp"
After = ["database", "cache-warmer"]
Requires = ["database"]
```

**Computed startup order**:
1. `redis` (no dependencies)
2. `database` (no dependencies)
3. `cache-warmer` (after redis)
4. `webapp` (after database and cache-warmer)

### Circular Dependency Detection

The init system detects and reports circular dependencies:

```toml
# service-a.service
After = ["service-b"]

# service-b.service
After = ["service-a"]
```

**Result**: Error logged, services start in arbitrary order.

```
[ERROR] Failed to compute startup order: Circular dependency detected in service definitions
```

---

## Control Protocols

The init system supports two control protocols for managing services.

### Unix Socket Protocol

**Purpose**: Local control within the enclave

**Configuration** (in `/etc/init.yaml`):
```yaml
control:
  unix_socket_enabled: true
  unix_socket_path: /run/init.sock
```

**Advantages**:
- Fast local communication
- Standard Unix permissions for access control
- No network overhead
- Works without VSOCK support

**Use Cases**:
- In-enclave service management
- Local automation scripts
- Container-like environments

**Client Configuration** (in `/etc/initctl.yaml`):
```yaml
protocol: unix
unix_socket_path: /run/init.sock
```

**Usage**:
```bash
# Inside enclave
initctl list
initctl status webapp
initctl restart webapp
```

---

### VSOCK Protocol

**Purpose**: Remote control from host to enclave

**Configuration** (in `/etc/init.yaml`):
```yaml
control:
  vsock_enabled: true
  vsock_cid: 4294967295  # VMADDR_CID_ANY
  vsock_port: 9001
```

**CID Selection**:
- **VMADDR_CID_ANY (4294967295)**: Listen on any CID (recommended)
- **Specific CID**: Bind to enclave's assigned CID
- **Auto-assignment**: Enclave CID is usually auto-assigned by hypervisor

**Advantages**:
- Remote management from host
- No need to enter enclave
- Secure communication via VSOCK
- Multiple enclaves on same host can use different ports

**Use Cases**:
- Host-to-enclave management
- CI/CD pipelines managing enclave services
- Monitoring and orchestration from host
- Zero-downtime deployments

**Client Configuration** (in `/etc/initctl.yaml` on host):
```yaml
protocol: vsock
vsock_cid: 16      # Enclave's CID
vsock_port: 9001
```

**Finding Enclave CID**:
```bash
# On host, list running enclaves
nitro-cli describe-enclaves

# Output includes:
# "EnclaveCID": 16
```

**Usage from Host**:
```bash
# Configure initctl
export INITCTL_CONFIG=/etc/enclave-init/initctl.yaml

# Or use CLI options
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 list

# Manage services remotely
initctl status webapp
initctl restart webapp
initctl logs webapp -n 100

# Enable/disable services
initctl enable newservice --now
initctl disable oldservice
```

---

### Dual Protocol Mode

**Purpose**: Enable both Unix socket and VSOCK simultaneously

**Configuration** (in `/etc/init.yaml`):
```yaml
control:
  # Enable both protocols
  unix_socket_enabled: true
  unix_socket_path: /run/init.sock

  vsock_enabled: true
  vsock_cid: 4294967295
  vsock_port: 9001
```

**Benefits**:
- Local management via Unix socket
- Remote management via VSOCK
- Flexibility in access method
- Redundancy

**Use Cases**:
- Development: local testing + remote monitoring
- Production: in-enclave automation + host management
- Troubleshooting: access from multiple locations

**Usage**:
```bash
# Inside enclave (Unix socket)
initctl list

# From host (VSOCK)
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 list
```

---

### Protocol Comparison

| Feature | Unix Socket | VSOCK |
|---------|-------------|-------|
| **Location** | Local (in-enclave) | Remote (host-to-enclave) |
| **Performance** | Very fast | Fast (VSOCK overhead) |
| **Security** | File permissions | Enclave isolation |
| **Use Case** | In-enclave management | Host management |
| **Requires** | Unix filesystem | VSOCK kernel module |
| **Port** | File path | CID + Port number |
| **Concurrent** | Yes | Yes |
| **Overhead** | Minimal | Low |

---

### IPC Protocol Format

Both Unix socket and VSOCK use the same JSON-based protocol.

**Request Format**:
```json
{
  "ServiceStatus": {
    "name": "webapp"
  }
}
```

**Response Format**:
```json
{
  "ServiceStatus": {
    "status": {
      "name": "webapp",
      "enabled": true,
      "active": true,
      "pid": 1234,
      "restart_policy": "always",
      "restart_count": 3,
      "restart_sec": 5,
      "exit_status": null,
      "exec_start": "/usr/bin/python3 /app/server.py",
      "working_directory": "/app",
      "dependencies": {
        "before": ["monitor"],
        "after": ["database", "cache"],
        "requires": ["database"],
        "required_by": []
      }
    }
  }
}
```

**Request Types**:
- `ListServices`
- `ServiceStatus { name }`
- `ServiceStart { name }`
- `ServiceStop { name }`
- `ServiceRestart { name }`
- `ServiceEnable { name }`
- `ServiceDisable { name }`
- `ServiceLogs { name, lines }`
- `ServiceLogsClear { name }`
- `SystemStatus`
- `SystemReload`
- `SystemReboot`
- `SystemShutdown`
- `Ping`

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

#### Examples

```bash
# Start with default config
init

# Start with custom config
init --config /etc/my-init.yaml
init -c /etc/my-init.yaml

# Use environment variable
INIT_CONFIG=/etc/my-init.yaml init

# Show help
init --help

# Show version
init --version
```

---

### `initctl` - Init Control Tool

Command-line interface for managing the init system and services.

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

#### Usage Examples

**Local (Unix Socket)**:
```bash
# Use default config
initctl list

# Override socket path
initctl --socket /custom/init.sock list
initctl -s /run/init.sock list
```

**Remote (VSOCK)**:
```bash
# Use config file
export INITCTL_CONFIG=/etc/enclave-init/initctl.yaml
initctl list

# Override protocol
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 list

# Short form
initctl -p vsock --vsock-cid 16 --vsock-port 9001 status webapp
```

#### Commands

### `list`

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
monitor                   enabled    active     always          2
cache                     disabled   inactive   always          0
```

**Columns:**
- `NAME`: Service name (from filename without .service extension)
- `ENABLED`: `enabled` or `disabled`
- `ACTIVE`: `active` (running) or `inactive` (not running)
- `RESTART`: Restart policy
- `RESTARTS`: Number of times the service has been restarted

**Remote Usage**:
```bash
# From host
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 list
```

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

# From host
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 status webapp
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

### `start`

Start a stopped service.

**Syntax:**
```bash
initctl start <SERVICE>
```

**Examples:**
```bash
# Local
initctl start webapp

# From host
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 start webapp
```

**Output:**
```
✓ Service 'webapp' started
```

---

### `stop`

Stop a running service.

**Syntax:**
```bash
initctl stop <SERVICE>
```

**Examples:**
```bash
# Local
initctl stop webapp

# From host
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 stop webapp
```

---

### `restart`

Restart a service.

**Syntax:**
```bash
initctl restart <SERVICE>
```

**Examples:**
```bash
# Local
initctl restart webapp

# From host (useful for deployments)
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 restart webapp
```

---

### `enable`

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
# Local
initctl enable webapp
initctl enable --now webapp

# From host
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 enable --now newservice
```

---

### `disable`

Disable a service.

**Syntax:**
```bash
initctl disable <SERVICE>
```

**Examples:**
```bash
# Local
initctl disable webapp

# From host
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 disable oldservice
```

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
# Local
initctl logs webapp
initctl logs webapp -n 100

# From host
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 logs webapp -n 200
```

---

### `logs-clear`

Clear all logs for a service.

**Syntax:**
```bash
initctl logs-clear <SERVICE>
```

**Examples:**
```bash
# Local
initctl logs-clear webapp

# From host
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 logs-clear webapp
```

---

### `system-status`

Show overall system status and statistics.

**Syntax:**
```bash
initctl system-status
```

**Examples:**
```bash
# Local
initctl system-status

# From host
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 system-status
```

**Output:**
```
System Status
  Uptime: 2d 5h 32m 15s
  Services: 12 total, 10 enabled, 9 active
  Service Directory: /service
  Log Directory: /log
```

---

### `reload`

Reload service configurations without restarting the system.

**Syntax:**
```bash
initctl reload
```

**Examples:**
```bash
# Local
initctl reload

# From host (useful for applying config changes)
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 reload
```

---

### `reboot`

Reboot the system (enclave).

**Syntax:**
```bash
initctl reboot
```

**Examples:**
```bash
# Local
initctl reboot

# From host
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 reboot
```

---

### `shutdown`

Shutdown the system (enclave).

**Syntax:**
```bash
initctl shutdown
```

**Examples:**
```bash
# Local
initctl shutdown

# From host
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 shutdown
```

---

### `ping`

Test connectivity to the init system.

**Syntax:**
```bash
initctl ping
```

**Examples:**
```bash
# Local
initctl ping

# From host (test VSOCK connectivity)
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 ping
```

---

## Usage Guide

### Basic Operations

#### Starting the Init System

```bash
# Standard startup (uses default config)
exec /sbin/init

# With custom config path
exec /sbin/init --config /etc/my-init.yaml

# Using environment variable
export INIT_CONFIG=/etc/my-init.yaml
exec /sbin/init
```

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
```

#### Managing Services Remotely from Host

```bash
# Setup initctl config on host
cat > /etc/enclave-init/initctl.yaml << EOF
protocol: vsock
vsock_cid: 16
vsock_port: 9001
EOF

export INITCTL_CONFIG=/etc/enclave-init/initctl.yaml

# Now manage enclave services from host
initctl list
initctl status webapp
initctl restart webapp
initctl logs webapp -n 100
```

### Advanced Operations

#### Remote Service Deployment

```bash
#!/bin/bash
# deploy.sh - Deploy new service from host

ENCLAVE_CID=16
INITCTL="initctl --protocol vsock --vsock-cid $ENCLAVE_CID --vsock-port 9001"

# Copy new service file to enclave (via shared volume or other mechanism)
# Then enable and start

$INITCTL enable newservice --now
$INITCTL status newservice
```

#### Zero-Downtime Restart from Host

```bash
#!/bin/bash
# rolling-restart.sh - Restart services one by one

SERVICES=("webapp-1" "webapp-2" "webapp-3")
INITCTL="initctl --protocol vsock --vsock-cid 16 --vsock-port 9001"

for service in "${SERVICES[@]}"; do
    echo "Restarting $service..."
    $INITCTL restart $service
    sleep 10  # Wait for health check
done
```

#### Multi-Enclave Management

```bash
#!/bin/bash
# manage-enclaves.sh - Manage multiple enclaves

ENCLAVES=(16 17 18)

for cid in "${ENCLAVES[@]}"; do
    echo "=== Enclave CID: $cid ==="
    initctl --protocol vsock --vsock-cid $cid --vsock-port 9001 list
    echo
done
```

#### CI/CD Integration

```yaml
# .github/workflows/deploy.yml
name: Deploy to Enclave

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2

      - name: Deploy service
        run: |
          # Copy service file to enclave
          # ...

          # Reload and restart
          initctl --protocol vsock \
                  --vsock-cid ${{ secrets.ENCLAVE_CID }} \
                  --vsock-port 9001 \
                  reload

          initctl --protocol vsock \
                  --vsock-cid ${{ secrets.ENCLAVE_CID }} \
                  --vsock-port 9001 \
                  restart myapp
```

---

## Advanced Topics

### VSOCK Networking Details

**VSOCK (Virtual Socket)** provides communication between host and guest VMs/enclaves.

#### VSOCK Address Format

```
CID:PORT
```

- **CID** (Context ID): Identifies the VM/enclave
- **PORT**: Port number (like TCP port)

#### Special CID Values

```rust
const VMADDR_CID_HYPERVISOR: u32 = 0;
const VMADDR_CID_LOCAL: u32 = 1;
const VMADDR_CID_HOST: u32 = 2;
const VMADDR_CID_ANY: u32 = 4294967295;  // -1U
```

#### Finding Enclave CID

**Method 1: From host**:
```bash
nitro-cli describe-enclaves | jq '.[] | .EnclaveCID'
```

**Method 2: Inside enclave**:
```bash
# CID is assigned by hypervisor and can be found in kernel messages
dmesg | grep -i vsock
```

**Method 3: From parent process**:
```bash
# When starting enclave
nitro-cli run-enclave --eif-path app.eif | jq '.EnclaveCID'
```

#### VSOCK Ports

Ports 0-1023 typically require privileges. Use ports 1024+ for services.

**Recommended Port Allocation**:
- `9000`: VSOCK heartbeat
- `9001`: Init control protocol
- `9002+`: Application services

### Control Socket Security

#### Unix Socket Security

**File Permissions**:
```bash
# Check socket permissions
ls -l /run/init.sock
# srwxr-xr-x 1 root root 0 Jan 1 00:00 /run/init.sock

# Restrict access
chmod 600 /run/init.sock  # Only root
chmod 660 /run/init.sock  # Root and group
```

**Access Control**:
- Based on Unix file permissions
- Users need read/write access to socket
- Can use groups for shared access

#### VSOCK Security

**Isolation**:
- VSOCK communication is isolated per VM/enclave
- Hypervisor enforces isolation
- Host can connect to enclave, but enclaves can't connect to each other (by default)

**Port Security**:
- Only accessible via VSOCK (not network)
- Hypervisor mediates all connections
- Enclave must explicitly listen on port

**Best Practices**:
- Use specific CID in client config (not ANY)
- Document which ports are used
- Consider adding authentication layer for sensitive operations

### Multi-Protocol Access Patterns

#### Pattern 1: Local Development

```yaml
# /etc/init.yaml
control:
  unix_socket_enabled: true
  unix_socket_path: /run/init.sock
  vsock_enabled: false
```

Use Unix socket for fast local testing.

#### Pattern 2: Production Enclave

```yaml
# /etc/init.yaml
control:
  unix_socket_enabled: true
  unix_socket_path: /run/init.sock
  vsock_enabled: true
  vsock_cid: 4294967295
  vsock_port: 9001
```

Enable both for flexibility.

#### Pattern 3: Secure Enclave

```yaml
# /etc/init.yaml
control:
  unix_socket_enabled: false
  vsock_enabled: true
  vsock_cid: 4294967295
  vsock_port: 9001
```

VSOCK only for controlled host access.

#### Pattern 4: Monitoring Setup

```yaml
# Host monitoring tool config
protocol: vsock
vsock_cid: 16
vsock_port: 9001
```

Monitor enclave health from host:
```bash
#!/bin/bash
while true; do
    if ! initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 ping; then
        echo "Enclave not responding!"
        # Alert
    fi
    sleep 30
done
```

---

## Troubleshooting

### VSOCK Issues

#### VSOCK Not Available

**Symptom**: "Failed to create VSOCK socket"

**Solutions**:

1. Check if VSOCK kernel module is loaded:
```bash
lsmod | grep vsock
# Should show: vhost_vsock, vmw_vsock_virtio_transport, vsock
```

2. Load VSOCK module:
```bash
modprobe vhost_vsock
modprobe vsock
```

3. Check kernel config:
```bash
zcat /proc/config.gz | grep VSOCK
# Should have: CONFIG_VSOCK=y or =m
```

#### Cannot Connect from Host

**Symptom**: "Failed to connect to VSOCK"

**Debugging**:

1. Verify enclave CID:
```bash
nitro-cli describe-enclaves
```

2. Check if enclave is listening:
```bash
# Inside enclave
netstat -l | grep 9001
# Or check init logs
grep "VSOCK control socket listening" /var/log/init.log
```

3. Test with simple VSOCK tool:
```bash
# On host
vsock-socat - VSOCK-CONNECT:16:9001
```

4. Check firewall/security groups (shouldn't affect VSOCK but verify)

#### Wrong CID

**Symptom**: Connection refused or timeout

**Solution**:

1. Get correct CID:
```bash
nitro-cli describe-enclaves | jq '.[0].EnclaveCID'
```

2. Update initctl config:
```yaml
vsock_cid: 16  # Use actual CID
```

3. Or use command line:
```bash
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 ping
```

#### Port Already in Use

**Symptom**: "Failed to bind VSOCK socket"

**Solutions**:

1. Check if another service is using the port:
```bash
# Inside enclave
netstat -tulpn | grep 9001
```

2. Change port in config:
```yaml
control:
  vsock_port: 9002  # Use different port
```

3. Kill conflicting process:
```bash
kill <PID>
```

### Protocol Selection Issues

#### initctl Uses Wrong Protocol

**Symptom**: Commands fail with connection errors

**Debugging**:

1. Check current config:
```bash
cat /etc/initctl.yaml
```

2. Check what initctl is using:
```bash
initctl ping
# Look for debug output showing protocol
```

3. Override protocol explicitly:
```bash
initctl --protocol unix ping
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 ping
```

#### Config File Not Found

**Symptom**: "Config file not found, using defaults"

**Solutions**:

1. Create config file:
```bash
cat > /etc/initctl.yaml << EOF
protocol: unix
unix_socket_path: /run/init.sock
EOF
```

2. Use custom path:
```bash
initctl --config /path/to/config.yaml list
```

3. Set environment variable:
```bash
export INITCTL_CONFIG=/etc/enclave-init/initctl.yaml
initctl list
```

### Remote Management Issues

#### Cannot Manage from Host

**Symptom**: Commands from host fail

**Checklist**:

1. ✓ VSOCK enabled in init config
2. ✓ Correct enclave CID
3. ✓ Correct port
4. ✓ Enclave is running
5. ✓ VSOCK kernel modules loaded
6. ✓ initctl configured for VSOCK

**Debug Steps**:

```bash
# 1. Verify enclave is running
nitro-cli describe-enclaves

# 2. Test VSOCK connectivity
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 ping

# 3. Check init logs inside enclave
# (via console or shared volume)
cat /var/log/init.log | grep VSOCK

# 4. Enable debug output
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 list 2>&1
```

#### Timeout on Remote Commands

**Symptom**: Commands hang or timeout

**Possible Causes**:

1. **Init not responding**: Check if init process is alive
2. **VSOCK queue full**: Check enclave resource usage
3. **Network issue**: Verify VSOCK connection

**Solutions**:

```bash
# Check enclave health
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 ping

# Try simpler command
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 system-status

# Check enclave resources
nitro-cli describe-enclaves | jq '.[0].State'
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
```

### Building with VSOCK Support

```bash
# VSOCK support requires nix crate with socket features
cargo build --release

# Verify VSOCK symbols
nm target/release/init | grep vsock
```

### Testing VSOCK Locally

Without a real enclave, you can test VSOCK using vsock_loopback:

```bash
# Load loopback module
modprobe vsock_loopback

# Test with loopback CID (1)
initctl --protocol vsock --vsock-cid 1 --vsock-port 9001 ping
```

### Project Structure

```
enclave-init/
├── Cargo.toml                  # Dependencies
├── src/
│   ├── main.rs                # Init system
│   ├── initctl.rs             # CLI tool
│   ├── protocol.rs            # IPC protocol
│   ├── config.rs              # Configuration (init & initctl)
│   ├── logger.rs              # Logging
│   └── dependencies.rs        # Dependency resolution
├── examples/
│   ├── init.yaml              # Init config example
│   ├── initctl.yaml           # Initctl config examples
│   │   ├── local.yaml         # Unix socket config
│   │   └── remote.yaml        # VSOCK config
│   └── services/              # Service file examples
└── docs/
    └── README.md              # This file
```

---

### Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new features
5. Run `cargo fmt` and `cargo clippy`
6. Submit a pull request

### Code Style

- Follow Rust standard style (enforced by `rustfmt`)
- Use `clippy` for linting
- Add documentation comments for public APIs
- Write tests for new features
- Use meaningful variable names
- Keep functions focused and small

---

## FAQ

### Control Protocol Questions

**Q: Can I use both Unix socket and VSOCK at the same time?**

A: Yes! Enable both in `/etc/init.yaml`:
```yaml
control:
  unix_socket_enabled: true
  vsock_enabled: true
```

**Q: How do I control enclave from host?**

A: Configure initctl on host to use VSOCK:
```yaml
# /etc/initctl.yaml on host
protocol: vsock
vsock_cid: 16  # Your enclave CID
vsock_port: 9001
```

**Q: What's the difference between VSOCK heartbeat and control socket?**

A:
- **Heartbeat** (`vsock.enabled`, port 9000): One-time signal to host that enclave is ready
- **Control socket** (`control.vsock_enabled`, port 9001): Ongoing service management protocol

**Q: Can multiple clients connect simultaneously?**

A: Yes, both Unix socket and VSOCK support concurrent connections. Each connection is handled in a separate thread.

**Q: Which protocol is faster?**

A: Unix socket is slightly faster for local communication. VSOCK has minimal overhead for remote access. The difference is negligible for most use cases.

**Q: Can I disable control socket completely?**

A: You can disable both protocols, but then you can't manage services at runtime:
```yaml
control:
  unix_socket_enabled: false
  vsock_enabled: false
```

**Q: How do I secure the control socket?**

A:
- **Unix socket**: Use file permissions (`chmod 600`)
- **VSOCK**: Enclave isolation provides security; optionally add authentication layer

**Q: Can I change ports after enclave is running?**

A: No, you need to restart the enclave with new configuration.

**Q: What if I forget the enclave CID?**

A: Query from host:
```bash
nitro-cli describe-enclaves | jq '.[] | .EnclaveCID'
```

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

#### Enclave Initctl Configuration

```yaml
# /etc/initctl.yaml - Enclave configuration
protocol: unix
unix_socket_path: /run/init.sock
vsock_cid: 3
vsock_port: 9001
```

### Environment Variables Reference

| Variable | Scope | Description |
|----------|-------|-------------|
| `INIT_CONFIG` | init | Path to init configuration file |
| `INITCTL_CONFIG` | initctl | Path to initctl configuration file |
| `INIT_SOCKET` | initctl | Override Unix socket path (deprecated) |

### Port Allocation Recommendations

| Port | Protocol | Purpose |
|------|----------|---------|
| 9000 | VSOCK | Heartbeat (init → host) |
| 9001 | VSOCK | Control protocol (host → init) |
| 9002 | VSOCK | Application service 1 |
| 9003 | VSOCK | Application service 2 |
| ... | VSOCK | Additional services |

### Command Reference Summary

#### Init Commands
```bash
init                                    # Default config
init --config /path/to/init.yaml       # Custom config
INIT_CONFIG=/path/to/init.yaml init    # Via environment
```

#### Initctl Commands (Local)
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
initctl system-status                  # System info
initctl reload                         # Reload configs
initctl ping                           # Test connection
```

#### Initctl Commands (Remote from Host)
```bash
# Via config file
export INITCTL_CONFIG=/etc/enclave-init/initctl.yaml
initctl list

# Via CLI options
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 list
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 status webapp
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 restart webapp
initctl --protocol vsock --vsock-cid 16 --vsock-port 9001 logs webapp -n 100
```

### VSOCK Reference

#### Constants
```rust
VMADDR_CID_HYPERVISOR = 0       // Hypervisor
VMADDR_CID_LOCAL = 1            // Local communication
VMADDR_CID_HOST = 2             // Host (legacy)
VMADDR_CID_ANY = 4294967295     // Listen on any CID (-1U)
```

#### Finding CID
```bash
# Method 1: nitro-cli
nitro-cli describe-enclaves | jq '.[] | .EnclaveCID'

# Method 2: From start output
nitro-cli run-enclave --eif-path app.eif | jq '.EnclaveCID'

# Method 3: Inside enclave
dmesg | grep -i vsock
```

#### Testing VSOCK
```bash
# Test connectivity
initctl --protocol vsock --vsock-cid <CID> --vsock-port 9001 ping

# Debug connection
strace initctl --protocol vsock --vsock-cid <CID> --vsock-port 9001 ping
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

### Version 0.5.0 (Beta State Release)

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

### Version 0.4.0 (Alpha State Release)

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

