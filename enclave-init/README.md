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
- [Service Dependencies](#service-dependencies)
- [CLI Reference](#cli-reference)
- [Usage Guide](#usage-guide)
- [Advanced Topics](#advanced-topics)
- [Troubleshooting](#troubleshooting)
- [Development](#development)
- [FAQ](#faq)
- [Appendix](#appendix)

---

## Overview

The Enclave Init System is a minimal, production-ready init system (PID 1) designed to run inside secure enclaves. It provides process supervision, automatic service restarts, service dependency management, comprehensive logging, and a control interface for managing services at runtime.

### Key Characteristics

- **Minimal footprint**: Small binary size optimized for enclave environments
- **Reliable**: Written in Rust with comprehensive error handling
- **Non-crashing**: All errors are logged but never crash the init system
- **Service supervision**: Automatic process monitoring and restart policies
- **Dependency management**: Systemd-style service dependencies with startup ordering
- **Runtime control**: Manage services without restarting the enclave
- **Enable/Disable**: Dynamic service activation control
- **Persistent logging**: Per-service log files with automatic rotation
- **Configurable**: YAML-based configuration for all aspects of the system
- **Flexible**: Configuration file path configurable via CLI and environment

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

- **Logging**
  - Per-service log files
  - Automatic log rotation based on size
  - Configurable retention (number of rotated files)
  - Timestamp prefixes
  - In-memory log cache for quick access
  - View and clear logs via CLI

- **Runtime Control**
  - Unix domain socket-based IPC
  - CLI tool (`initctl`) for service management
  - Start, stop, restart services
  - Enable, disable services
  - Reload configurations without restart
  - View service status and logs
  - System-wide operations (reload, reboot, shutdown)

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
┌───────────────────────────────────────────────────┐
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
│  │  - Control Socket (/run/init.sock)        │    │
│  └───────────────────────────────────────────┘    │
│           │              │              │         │
│     ┌─────┴─────┐  ┌─────┴─────┐  ┌─────┴─────┐   │
│     │ Service A │  │ Service B │  │ Service C │   │
│     │(depends B)│  │           │  │(after A,B)│   │
│     └───────────┘  └───────────┘  └───────────┘   │
│                                                   │
│  ┌───────────────────────────────────────────┐    │
│  │              initctl                      │    │
│  │  (CLI tool for runtime control)           │    │
│  └───────────────────────────────────────────┘    │
│                                                   │
│  Filesystem Layout:                               │
│  /etc/init.yaml              - Init configuration │
│  /service/*.service          - Service files      │
│  /service/*.service.disabled - Disabled services  │
│  /log/*.log                  - Service logs       │
└───────────────────────────────────────────────────┘
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
       ├─> Start Control Socket Server
       │
       └─> ┌────────────────┐
           │   Main Loop    │
           └────────┬───────┘
                    │
                    ├─> Check SIGCHLD → Reap Children
                    ├─> Check SIGTERM/SIGINT → Shutdown
                    ├─> Check SIGHUP → Reload Services
                    ├─> Restart Dead Services (per policy)
                    ├─> Handle Control Socket Requests
                    │
                    └─> [Loop continues]
```

### Dependency Resolution Flow

```
Service Definitions
        │
        ▼
┌────────────────┐
│  Parse Before  │
│  Parse After   │
│  Parse Requires│
└────────┬───────┘
         │
         ▼
┌────────────────────┐
│ Build Dependency   │
│ Graph              │
└────────┬───────────┘
         │
         ▼
┌────────────────────┐
│ Validate           │
│ Dependencies       │
│ (check existence)  │
└────────┬───────────┘
         │
         ▼
┌────────────────────┐
│ Detect Circular    │
│ Dependencies       │
└────────┬───────────┘
         │
         ▼
┌────────────────────┐
│ Topological Sort   │
│ (Kahn's Algorithm) │
└────────┬───────────┘
         │
         ▼
┌────────────────────┐
│ Startup Order List │
└────────────────────┘
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

### 2. Create Service Files with Dependencies

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

Create `/service/monitor.service`:

```toml
ExecStart = "/usr/bin/monitor --interval 60"
Environment = [
    "TARGETS=webapp,database"
]
Restart = "always"
RestartSec = 30
ServiceEnable = true

# Monitor should start after other services
After = ["database", "webapp"]
```

### 3. Start Init System

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

### 4. Manage Services

```bash
# List all services (shows enabled/disabled and active status)
initctl list

# Check service status (includes dependency information)
initctl status webapp

# View logs
initctl logs webapp -n 100

# Restart a service
initctl restart webapp

# Enable a service
initctl enable myapp

# Enable and start immediately
initctl enable --now myapp

# Disable a service (stops it first)
initctl disable myapp

# Reload all service configurations
initctl reload
```

---

## Configuration Reference

### Init Configuration File

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
  CUSTOM_VAR: value

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

#### Configuration Loading Priority

1. **CLI argument**: `init --config /path/to/config.yaml` (highest priority)
2. **Environment variable**: `INIT_CONFIG=/path/to/config.yaml`
3. **Default path**: `/etc/init.yaml`
4. **Built-in defaults**: If no file found

#### Environment Variables

Environment variables defined in the configuration are:
1. Set for the init process itself
2. Inherited by all child processes (services)
3. Can be overridden by per-service environment variables

Example usage:
```yaml
environment:
  TZ: UTC
  LANG: en_US.UTF-8
  APP_ENV: production
  LOG_LEVEL: info
```

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

# Or use initctl
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
    "API_TIMEOUT=30",
    "DEBUG=false"
]
```

### Command Line Parsing

The `ExecStart` command supports:
- Simple commands: `"/usr/bin/app"`
- Arguments: `"/usr/bin/app --config /etc/app.conf"`
- Quoted arguments: `"/usr/bin/app --name \"My App\""`
- Escaped characters: `"/usr/bin/app --path \\"/tmp/file\\\""`

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

#### Database with Dependencies

```toml
ExecStart = "/usr/bin/postgres -D /var/lib/postgresql/data"
Environment = [
    "POSTGRES_PASSWORD=secret",
    "POSTGRES_DB=myapp"
]
Restart = "always"
RestartSec = 10
WorkingDirectory = "/var/lib/postgresql"

# Other services depend on this
Before = ["webapp", "api"]
```

#### Application with Database Dependency

```toml
ExecStart = "/usr/bin/myapp"
Environment = [
    "DATABASE_URL=postgres://localhost/myapp"
]
Restart = "on-failure"
RestartSec = 15
WorkingDirectory = "/app"

# Wait for database to start
After = ["database"]
Requires = ["database"]
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

# Start after main app
After = ["webapp"]
```

#### Monitoring Agent

```toml
ExecStart = "/usr/bin/prometheus --config.file=/etc/prometheus/prometheus.yml"
Environment = [
    "GOMAXPROCS=2"
]
Restart = "always"
RestartSec = 5

# Start after all monitored services
After = ["webapp", "database", "cache"]
```

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

### Dependency Best Practices

#### 1. Use `After` for Soft Dependencies

```toml
# Good: Service can start even if logger fails
After = ["logger"]
```

#### 2. Use `Requires` for Critical Dependencies

```toml
# Good: Won't start without database
Requires = ["database"]
After = ["database"]  # Always combine with After
```

#### 3. Avoid Circular Dependencies

```toml
# Bad: Creates a circle
[a.service]
After = ["b"]

[b.service]
After = ["a"]
```

#### 4. Use `Before` for Reverse Ordering

```toml
# Infrastructure services specify what comes after
[database.service]
Before = ["webapp", "api", "worker"]
```

#### 5. Document Reverse Dependencies

```toml
# Helps understand service relationships
[database.service]
RequiredBy = ["webapp", "api"]
Before = ["webapp", "api"]
```

### Dependency Validation

At startup, the init system:

1. **Validates `Requires`**: Errors if required service doesn't exist
2. **Warns about `After`/`Before`**: Non-fatal warning if service doesn't exist
3. **Checks for cycles**: Errors if circular dependencies detected

```
[ERROR] Service 'webapp' requires 'database' which does not exist
[WARN] Service 'webapp' has After='cache' which does not exist (non-fatal)
[INFO] Computed startup order: [database, redis, webapp, monitor]
```

### Complex Dependency Example

```toml
# infrastructure.service - Base services
[infrastructure.service]
ExecStart = "/usr/bin/setup-infra"
Before = ["database", "cache", "queue"]

# database.service
[database.service]
ExecStart = "/usr/bin/postgres"
After = ["infrastructure"]
Before = ["api", "webapp", "worker"]
RequiredBy = ["api", "webapp"]

# cache.service
[cache.service]
ExecStart = "/usr/bin/redis-server"
After = ["infrastructure"]
Before = ["api", "webapp"]

# queue.service
[queue.service]
ExecStart = "/usr/bin/rabbitmq-server"
After = ["infrastructure"]
Before = ["worker"]
RequiredBy = ["worker"]

# api.service
[api.service]
ExecStart = "/usr/bin/api-server"
After = ["database", "cache"]
Requires = ["database"]
Before = ["webapp"]

# webapp.service
[webapp.service]
ExecStart = "/usr/bin/webapp"
After = ["api", "database", "cache"]
Requires = ["database", "api"]

# worker.service
[worker.service]
ExecStart = "/usr/bin/worker"
After = ["database", "queue"]
Requires = ["database", "queue"]

# monitor.service - Monitors everything
[monitor.service]
ExecStart = "/usr/bin/monitor"
After = ["database", "cache", "queue", "api", "webapp", "worker"]
```

**Startup order**:
1. infrastructure
2. database, cache, queue (parallel, all after infrastructure)
3. api (after database, cache)
4. webapp (after api, database, cache)
5. worker (after database, queue)
6. monitor (after everything)

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
- Service must be enabled
- Service must be in stopped state
- Error if service is already running
- Clears manual stop flag (allows automatic restarts)

**Error Cases:**
```
✗ Error: Service 'webapp' is disabled
✗ Error: Service 'webapp' is already running
✗ Error: Service 'webapp' not found
```

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

**Error Cases:**
```
✗ Error: Service 'webapp' is not running
✗ Error: Service 'webapp' not found
```

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
- Service must be enabled

**Error Cases:**
```
✗ Error: Service 'webapp' is disabled
✗ Error: Service 'webapp' not found
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
# Enable service (requires reload or restart to start)
initctl enable webapp

# Enable and start immediately
initctl enable --now webapp
```

**Output:**
```
✓ Service 'webapp' enabled
✓ Service 'webapp' started
```

**Behavior:**
- Renames `.service.disabled` to `.service`
- Triggers SIGHUP (reload) signal to init
- With `--now`: Also starts the service immediately

**Error Cases:**
```
✗ Error: Failed to enable service 'webapp': Service file not found
```

---

### `disable`

Disable a service.

**Syntax:**
```bash
initctl disable <SERVICE>
```

**Example:**
```bash
initctl disable webapp
```

**Output:**
```
✓ Service 'webapp' disabled
```

**Behavior:**
- Stops the service if running (sends SIGTERM)
- Renames `.service` to `.service.disabled`
- Triggers SIGHUP (reload) signal to init
- Service won't start on next boot

**Error Cases:**
```
✗ Error: Failed to disable service 'webapp': Service file not found
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
- Removes all rotated log files (`.log.1`, `.log.2`, etc.)
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
  Services: 12 total, 10 enabled, 9 active
  Service Directory: /service
  Log Directory: /log
```

**Information Displayed:**
- System uptime since init started
- Total number of services configured
- Number of enabled services
- Number of currently active (running) services
- Configured service directory
- Configured log directory

---

### `reload`

Reload service configurations without restarting the system.

**Syntax:**
```bash
initctl reload
```

**Output:**
```
✓ System reload initiated
```

**Behavior:**
1. Sends SIGHUP signal to init process
2. Init rescans service directory
3. Loads new service files
4. Stops removed/disabled services
5. Starts new enabled services
6. Respects dependency ordering

**Use Cases:**
- Added new service files
- Modified existing service files
- Enabled/disabled services manually
- Want to apply changes without full restart

**Note**: Running services are not restarted. To apply changes to a running service, use `initctl restart <service>`.

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
- Monitoring and alerting

---

## Usage Guide

### Basic Operations

#### Starting the Init System

The init system is designed to run as PID 1 and starts automatically when the enclave boots.

```bash
# Standard startup (uses default config)
exec /sbin/init

# With custom config path
exec /sbin/init --config /etc/my-init.yaml

# Using environment variable
export INIT_CONFIG=/etc/my-init.yaml
exec /sbin/init

# In kernel command line
init=/sbin/init
```

#### Checking Service Status

```bash
# List all services
initctl list

# Check specific service
initctl status myapp

# Check if init is responsive
initctl ping

# Show system status
initctl system-status
```

#### Managing Services

```bash
# Start a service
initctl start myapp

# Stop a service
initctl stop myapp

# Restart a service
initctl restart myapp

# Enable a service
initctl enable myapp

# Enable and start immediately
initctl enable --now myapp

# Disable a service (stops it first)
initctl disable myapp
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

# 2. Reload configurations
initctl reload

# 3. Restart the service to apply changes
initctl restart myapp
```

**Alternative: Restart only one service**
```bash
# Edit service file
vim /service/myapp.service

# Just restart the service
initctl restart myapp
```

**Note:** `reload` rescans all services. For single service changes, `restart` is faster.

#### Adding New Services at Runtime

```bash
# 1. Create new service file
cat > /service/newapp.service << 'EOF'
ExecStart = "/usr/bin/newapp"
Restart = "always"
RestartSec = 5
ServiceEnable = true
After = ["database"]
EOF

# 2. Reload service configurations
initctl reload

# 3. Verify service is loaded
initctl status newapp

# 4. Service should start automatically if enabled
# Or start manually
initctl start newapp
```

#### Removing Services at Runtime

```bash
# 1. Stop the service
initctl stop myapp

# 2. Remove or disable the service file
rm /service/myapp.service
# or
mv /service/myapp.service /service/myapp.service.disabled

# 3. Reload configurations
initctl reload
```

#### Managing Service Dependencies

When working with services that have dependencies:

```bash
# 1. Check service dependencies
initctl status webapp
# Look for: After, Before, Requires

# 2. Verify dependency startup order
# Check init logs for startup order
grep "Computed startup order" /var/log/init.log

# 3. Start services in correct order (automatic)
# Init handles ordering automatically

# 4. If dependency fails, check required services
initctl status database  # Required by webapp
```

#### Debugging Service Issues

```bash
# 1. Check service status
initctl status myapp

# 2. View logs
initctl logs myapp -n 100

# 3. Check exit code
# Look for "Last Exit Code" in status output

# 4. Check dependencies
# Look for "Requires", "After" in status

# 5. Try starting manually for debugging
# Stop the service first
initctl stop myapp

# Start in a shell for debugging
/usr/bin/myapp --verbose

# Once fixed, restart via init
initctl start myapp
```

#### Handling Circular Dependencies

If you encounter circular dependency errors:

```bash
# Error message
[ERROR] Failed to compute startup order: Circular dependency detected

# 1. Check service dependencies
initctl status service-a
initctl status service-b

# 2. Identify the cycle
# service-a After=[service-b]
# service-b After=[service-a]

# 3. Fix by removing one dependency
vim /service/service-a.service
# Remove or modify After directive

# 4. Reload
initctl reload
```

#### Log Rotation Management

Logs automatically rotate when they exceed `max_log_size`. To manually manage logs:

```bash
# Check log file sizes
du -h /log/*.log

# View rotated logs
ls -lh /log/myapp.log*
# myapp.log      - current
# myapp.log.1    - previous
# myapp.log.2    - older

# Clear logs if needed
initctl logs-clear myapp

# Or manually remove old rotations
rm /log/myapp.log.{3,4,5}

# Compress old logs
find /log -name "*.log.*" -exec gzip {} \;
```

#### Testing Service Dependencies

```bash
# 1. Create test services with dependencies
cat > /service/test-db.service << 'EOF'
ExecStart = "/bin/sleep infinity"
Before = ["test-app"]
EOF

cat > /service/test-app.service << 'EOF'
ExecStart = "/bin/sleep infinity"
After = ["test-db"]
Requires = ["test-db"]
EOF

# 2. Reload and check startup order
initctl reload

# 3. Verify order in logs
# test-db should start before test-app

# 4. Test failure handling
# Stop test-db and see if test-app complains
initctl stop test-db
```

#### Using Custom Configuration Paths

```bash
# Development environment
init --config /opt/dev-init.yaml

# Production environment
init --config /etc/prod-init.yaml

# Testing environment
INIT_CONFIG=/tmp/test-init.yaml init

# Multiple environments with initctl
export INIT_SOCKET=/run/init-dev.sock
initctl list

export INIT_SOCKET=/run/init-prod.sock
initctl list
```

### System Administration

#### Backup and Restore

**Backup service configurations:**
```bash
# Backup all service files
tar -czf services-backup-$(date +%Y%m%d).tar.gz /service/

# Backup init configuration
cp /etc/init.yaml /backup/init.yaml.$(date +%Y%m%d)

# Backup logs (optional)
tar -czf logs-backup-$(date +%Y%m%d).tar.gz /log/
```

**Restore service configurations:**
```bash
# Restore service files
tar -xzf services-backup-20240115.tar.gz -C /

# Restore init configuration
cp /backup/init.yaml.20240115 /etc/init.yaml

# Reload to apply restored services
initctl reload
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
# /opt/scripts/archive-logs.sh
DATE=$(date +%Y%m%d)
tar -czf /archive/logs-${DATE}.tar.gz /log/*.log.*
find /log -name "*.log.*" -delete
echo "Logs archived to /archive/logs-${DATE}.tar.gz"
```

3. **Schedule archival** (create a service for it):
```toml
# /service/log-archiver.service
ExecStart = "/bin/sh -c 'sleep 86400 && /opt/scripts/archive-logs.sh'"
Restart = "always"
```

4. **Monitor log directory size**:
```bash
# Add to monitoring service
du -sh /log
df -h /log
```

#### Graceful Shutdown

To safely shutdown the enclave:

```bash
# Option 1: Using initctl (recommended)
initctl shutdown

# Option 2: Send signal to init
kill -TERM 1

# Option 3: System command (if available)
shutdown -h now

# Option 4: Reboot instead
initctl reboot
```

All methods will:
1. Stop all services gracefully (SIGTERM)
2. Wait 5 seconds
3. Force kill remaining processes (SIGKILL)
4. Shutdown the system

#### Service Health Monitoring

Create a monitoring service:

```toml
# /service/health-monitor.service
ExecStart = "/opt/scripts/health-monitor.sh"
Restart = "always"
RestartSec = 60
After = ["webapp", "database", "cache"]
```

```bash
#!/bin/sh
# /opt/scripts/health-monitor.sh

while true; do
    # Check if services are running
    if ! initctl status webapp | grep -q "active"; then
        echo "WARNING: webapp is not active"
    fi

    if ! initctl status database | grep -q "active"; then
        echo "CRITICAL: database is not active"
    fi

    sleep 60
done
```

---

## Advanced Topics

### Signal Handling

The init system handles signals as follows:

| Signal | Behavior |
|--------|----------|
| `SIGCHLD` | Reap zombie processes, check for service exits, trigger restarts |
| `SIGTERM` | Initiate graceful shutdown |
| `SIGINT` | Initiate graceful shutdown (Ctrl+C) |
| `SIGHUP` | Reload service configurations |
| Others | Blocked in init process |

**Child Process Signals:**
- All signals are unblocked in child processes
- Services receive signals directly
- Services can handle signals for graceful shutdown

**Example: Service handling SIGTERM**
```bash
#!/bin/sh
# Service that handles SIGTERM gracefully

trap 'echo "Shutting down..."; cleanup; exit 0' TERM

cleanup() {
    # Save state
    echo "Saving state..."
    # Close connections
    echo "Closing connections..."
}

# Main loop
while true; do
    # Do work
    sleep 1
done
```

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
    │    │ Unblock │
    │    │ Signals │
    │    ├─────────┤
    │    │ Setsid  │
    │    ├─────────┤
    │    │ Setpgid │
    │    ├─────────┤
    │    │ Chdir   │
    │    ├─────────┤
    │    │ Set Env │
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
  ┌──────▼──────┐
  │  Enabled?   │
  └──────┬──────┘
         │
    Restart?
    Yes ├─┐
    No  │ │
        │ └──> [Wait RestartSec] ──> [Check Deps] ──> [Start Cmd]
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

**Disabling:**
```yaml
vsock:
  enabled: false
```

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

**Disabling:**
```yaml
pivot_root: false
```

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

**Request Types:**
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

### Dependency Resolution Algorithm

**Kahn's Algorithm for Topological Sort:**

```rust
fn compute_startup_order(services: &HashMap<String, ServiceDependencies>) -> Result<Vec<String>> {
    // 1. Initialize in-degree for each service
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();

    for service_name in services.keys() {
        in_degree.insert(service_name.clone(), 0);
        graph.insert(service_name.clone(), Vec::new());
    }

    // 2. Build dependency graph
    for (service_name, deps) in services {
        // After: service starts after these
        for after in &deps.after {
            if services.contains_key(after) {
                graph.get_mut(after).unwrap().push(service_name.clone());
                *in_degree.get_mut(service_name).unwrap() += 1;
            }
        }

        // Before: service starts before these
        for before in &deps.before {
            if services.contains_key(before) {
                graph.get_mut(service_name).unwrap().push(before.clone());
                *in_degree.get_mut(before).unwrap() += 1;
            }
        }

        // Requires: must start after required services
        for required in &deps.requires {
            if services.contains_key(required) {
                graph.get_mut(required).unwrap().push(service_name.clone());
                *in_degree.get_mut(service_name).unwrap() += 1;
            }
        }
    }

    // 3. Topological sort
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut result: Vec<String> = Vec::new();

    // Find nodes with no incoming edges
    for (service, &degree) in &in_degree {
        if degree == 0 {
            queue.push_back(service.clone());
        }
    }

    // Process nodes
    while let Some(service) = queue.pop_front() {
        result.push(service.clone());

        if let Some(neighbors) = graph.get(&service) {
            for neighbor in neighbors {
                if let Some(degree) = in_degree.get_mut(neighbor) {
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }
    }

    // 4. Check for cycles
    if result.len() != services.len() {
        return Err("Circular dependency detected".to_string());
    }

    Ok(result)
}
```

**Time Complexity**: O(V + E) where V is number of services and E is number of dependencies

**Space Complexity**: O(V + E)

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

4. Check configuration file:
   ```bash
   # Verify config exists
   ls -l /etc/init.yaml

   # Test with default config
   init  # Uses /etc/init.yaml

   # Test with custom config
   init --config /tmp/test.yaml
   ```

5. Check environment variable:
   ```bash
   echo $INIT_CONFIG
   # If set, init uses this path
   ```

#### Service Won't Start

**Symptom:** Service shows as inactive, won't start

**Debugging:**
```bash
# Check service status
initctl status myapp

# Check if service is enabled
initctl list | grep myapp

# View logs
initctl logs myapp

# Check service file syntax
cat /service/myapp.service

# Check for .disabled extension
ls -l /service/myapp.service*

# Try running command manually
/usr/bin/myapp
```

**Common Causes:**
- Service is disabled (`.service.disabled` or `ServiceEnable = false`)
- Incorrect `ExecStart` path
- Missing executable permissions
- Missing dependencies (`Requires` not satisfied)
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

# Check restart count
initctl status myapp | grep "Restart Count"
```

**Solutions:**

1. Fix application errors causing crashes

2. Adjust restart policy:
   ```toml
   Restart = "on-failure"
   RestartSec = 30  # Longer delay
   ```

3. Check for port conflicts:
   ```bash
   netstat -tulpn | grep :8080
   ```

4. Check resource issues:
   ```bash
   free -h  # Memory
   df -h    # Disk space
   ```

5. Temporarily disable restart to debug:
   ```toml
   Restart = "no"
   ```

#### Dependency Resolution Failures

**Symptom:** Services start in wrong order or not at all

**Debugging:**
```bash
# Check dependencies
initctl status myapp

# Look for errors in init logs
dmesg | grep -i "dependency\|circular"

# Check startup order
grep "Computed startup order" /var/log/messages
```

**Common Issues:**

1. **Circular dependencies:**
   ```toml
   # service-a.service
   After = ["service-b"]

   # service-b.service
   After = ["service-a"]
   ```
   **Fix:** Remove one of the dependencies

2. **Missing required service:**
   ```toml
   Requires = ["nonexistent-service"]
   ```
   **Fix:** Create the required service or remove dependency

3. **Typo in service name:**
   ```toml
   After = ["databse"]  # Should be "database"
   ```
   **Fix:** Correct the spelling

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

# Set environment variable
export INIT_SOCKET=/run/init.sock
initctl list

# Check socket permissions
ls -l /run/init.sock
# Should be: srwxr-xr-x
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

# Check disk space
df -h /log
```

**Solutions:**

1. Ensure log directory exists and is writable:
   ```bash
   mkdir -p /log
   chmod 755 /log
   ```

2. Check `log_dir` in `/etc/init.yaml`:
   ```yaml
   log_dir: /log
   ```

3. Verify service is actually writing to stdout/stderr

4. Check if service redirects output elsewhere

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
echo $?  # 0 if exists, 1 if not

# Force reload
initctl reload
```

**Solutions:**
- Wait a moment for status to update
- Service may have just crashed (check logs)
- Reload service configurations: `initctl reload`

#### Enable/Disable Not Working

**Symptom:** `enable` or `disable` commands fail

**Debugging:**
```bash
# Check service file exists
ls -l /service/myapp.service*

# Check file permissions
ls -l /service/

# Try manual rename
mv /service/myapp.service /service/myapp.service.disabled

# Check for errors
initctl enable myapp 2>&1
```

**Solutions:**

1. Ensure service directory is writable:
   ```bash
   chmod 755 /service
   ```

2. Check for file locks:
   ```bash
   lsof | grep myapp.service
   ```

3. Reload after manual changes:
   ```bash
   initctl reload
   ```

### Debug Mode

Enable debug logging in init system:

```yaml
# /etc/init.yaml
environment:
  RUST_LOG: debug
```

Check debug output:
```bash
# View init logs
journalctl -u init

# Or check console output
dmesg | grep init

# Or stderr if redirected
cat /var/log/init.log
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

# Check for infinite loops in services
strace -p <PID>
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

# Check init memory usage
ps aux | grep "^root.*init$"
```

#### Too Many Log Files

```bash
# Check log directory size
du -sh /log

# List largest log files
du -h /log/* | sort -rh | head

# Adjust rotation settings in /etc/init.yaml:
max_log_size: 5242880    # 5 MB (smaller files)
max_log_files: 3         # Fewer rotations

# Or clear old logs
initctl logs-clear myapp

# Or remove old rotations
find /log -name "*.log.*" -delete
```

#### Slow Service Startup

```bash
# Check dependency chain length
initctl status myapp | grep -E "After|Requires"

# Reduce dependencies if possible
# Or adjust startup delays

# Check for services waiting unnecessarily
# Remove unused After directives

# Parallel startup not supported
# Services start sequentially per dependency order
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

# Run specific tests
cargo test test_dependencies

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
│   └── dependencies.rs        # Dependency resolution
├── examples/
│   ├── init.yaml              # Example init configuration
│   └── services/              # Example service files
│       ├── webapp.service
│       ├── database.service
│       └── monitor.service
├── tests/
│   ├── integration/           # Integration tests
│   └── dependencies_test.rs   # Dependency resolution tests
├── docs/
│   └── README.md              # This file
└── README.md                  # Project overview
```

### Testing

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test '*'

# Test specific module
cargo test config
cargo test dependencies

# Test with output
cargo test -- --nocapture

# Test dependency resolution
cargo test test_simple_order
cargo test test_circular_dependency
cargo test test_requires
```

### Running Tests

**Dependency Resolution Tests:**
```bash
cd enclave-init
cargo test -p enclave-init --lib dependencies
```

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

A: No, it uses TOML format which is simpler to parse. However, basic directives (`ExecStart`, `Restart`, `After`, `Before`, `Requires`) are similar to systemd.

**Q: Can services run as different users?**

A: Not currently. All services run as the same user as init (typically root). This is by design for enclave environments.

**Q: How do I run multiple instances of the same service?**

A: Create separate service files:
```bash
/service/worker-1.service
/service/worker-2.service
/service/worker-3.service
```

**Q: Can I reload configuration without restarting?**

A: Yes! Use `initctl reload` to reload service configurations. Running services are not automatically restarted; use `initctl restart <service>` to apply changes.

**Q: How do I specify the config file location?**

A: Three ways:
```bash
# CLI argument (highest priority)
init --config /etc/my-config.yaml

# Environment variable
export INIT_CONFIG=/etc/my-config.yaml
init

# Default location
# init looks for /etc/init.yaml
```

### Service Management

**Q: How do I prevent a service from starting at boot?**

A: Two methods:

1. Disable via initctl:
   ```bash
   initctl disable myapp
   ```

2. Set in service file:
   ```toml
   ServiceEnable = false
   ```

**Q: Can services communicate with each other?**

A: Yes, through normal IPC mechanisms (sockets, pipes, shared memory, etc.). The init system doesn't impose restrictions.

**Q: How do I ensure one service starts before another?**

A: Use dependency directives:
```toml
# In service-a.service
After = ["service-b"]

# Or in service-b.service
Before = ["service-a"]
```

**Q: What happens if all services exit?**

A: Init continues running, waiting for commands via control socket. It will never exit voluntarily.

**Q: How do I handle optional dependencies?**

A: Use `After` without `Requires`:
```toml
# Start after logger if it exists, but don't fail if it doesn't
After = ["logger"]
```

**Q: Can I have multiple services depend on one service?**

A: Yes:
```toml
# In database.service
Before = ["webapp", "api", "worker"]

# Or in each dependent service
After = ["database"]
```

### Dependencies

**Q: What's the difference between `After` and `Requires`?**

A:
- `After`: Soft dependency, ordering only. Service starts even if dependency fails.
- `Requires`: Hard dependency. Service won't start if dependency is missing or fails.

**Q: Can I have circular dependencies?**

A: No, the init system detects and rejects circular dependencies:
```
[ERROR] Circular dependency detected in service definitions
```

**Q: What happens if a required service fails?**

A: The dependent service won't start. Check logs:
```bash
initctl status webapp
# Shows: Service 'webapp' requires 'database' which is not running
```

**Q: How do I debug dependency issues?**

A:
```bash
# Check service dependencies
initctl status myapp

# Check startup order
grep "Computed startup order" /var/log/messages

# Validate dependencies
initctl reload  # Will show errors
```

### Logging

**Q: Can I send logs to syslog?**

A: Services can log to syslog if configured. Init system logs to files only.

**Q: How long are logs kept?**

A: Based on rotation settings. With defaults (10MB × 5 files), up to 50MB per service.

**Q: Can I export logs to external storage?**

A: Yes, periodically copy `/log/` directory or setup a service that forwards logs.

**Q: Are logs persistent across reboots?**

A: Only if `/log` is on persistent storage. In enclaves, usually stored in memory and lost on reboot.

**Q: How do I view all rotated logs?**

A:
```bash
# View current log
cat /log/myapp.log

# View all logs (current + rotated)
cat /log/myapp.log*

# Or use tail
tail -f /log/myapp.log
```

### Performance

**Q: What's the overhead of the init system?**

A: Minimal. Init uses <10MB RAM typically and negligible CPU when idle.

**Q: How many services can it manage?**

A: Tested with 100+ services. No hard limit, but consider resource constraints.

**Q: Does it support cgroups resource limits?**

A: Init mounts cgroups but doesn't configure limits. You can configure limits manually or via service wrapper scripts.

**Q: Can services start in parallel?**

A: No, services start sequentially based on dependency order. This ensures correct ordering but may be slower than parallel startup.

**Q: How fast is dependency resolution?**

A: O(V + E) complexity using topological sort. Very fast even with complex dependency graphs.

### Security

**Q: Is the control socket secured?**

A: It uses Unix domain socket permissions. Only users with access to the socket can control services.

**Q: Can services escape the enclave?**

A: No, enclave isolation is enforced by the hypervisor, not init.

**Q: Does it validate service files?**

A: Basic validation only. Malformed files are logged and skipped.

**Q: Can I restrict which services can be controlled?**

A: Not currently. Any user with socket access can control all services.

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
  ENVIRONMENT: production

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
  ENVIRONMENT: development

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
ServiceEnable = true

After = ["database", "cache"]
Requires = ["database"]
Before = ["monitor"]
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
ServiceEnable = true

After = ["cache"]
Before = ["webapp"]
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
ServiceEnable = true

After = ["database"]
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
ServiceEnable = true

After = ["webapp", "database", "cache"]
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
ServiceEnable = true

After = ["database"]
Requires = ["database"]
```

#### Database Service

```toml
# /service/database.service
ExecStart = "/usr/bin/postgres -D /var/lib/postgresql/data"
Environment = [
    "POSTGRES_PASSWORD=secret",
    "POSTGRES_DB=myapp",
    "POSTGRES_USER=appuser"
]
Restart = "always"
RestartSec = 10
WorkingDirectory = "/var/lib/postgresql"
ServiceEnable = true

Before = ["webapp", "api", "worker"]
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
| SIGHUP | Reload | Reload service configurations |
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
│   └── init                          # Init binary (PID 1)
├── usr/
│   └── bin/
│       └── initctl                   # Control tool
├── etc/
│   ├── init.yaml                     # Init configuration
│   └── ...
├── service/                          # Service definitions
│   ├── webapp.service
│   ├── database.service
│   ├── worker.service
│   ├── monitor.service
│   └── backup.service.disabled       # Disabled service
├── log/                              # Service logs
│   ├── webapp.log
│   ├── webapp.log.1
│   ├── webapp.log.2
│   ├── database.log
│   └── worker.log
├── run/
│   └── init.sock                     # Control socket
├── proc/                             # Process information
├── sys/                              # Kernel objects
├── dev/                              # Device nodes
└── tmp/                              # Temporary files
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

#### Optimizing Dependency Chains

```toml
# Bad: Long linear chain
# a -> b -> c -> d -> e

# Better: Parallel where possible
# a, b, c, d all independent
# e depends on all of them
```

### Command Reference Summary

#### Init Commands
```bash
# Start init with default config
init

# Start init with custom config
init --config /path/to/config.yaml

# Use environment variable
INIT_CONFIG=/path/to/config.yaml init
```

#### Service Management
```bash
initctl list                    # List all services
initctl status <service>        # Show service status
initctl start <service>         # Start service
initctl stop <service>          # Stop service
initctl restart <service>       # Restart service
initctl enable <service>        # Enable service
initctl enable --now <service>  # Enable and start
initctl disable <service>       # Disable service
```

#### Logs
```bash
initctl logs <service>          # Show last 50 lines
initctl logs <service> -n 100   # Show last 100 lines
initctl logs-clear <service>    # Clear logs
```

#### System
```bash
initctl system-status           # Show system status
initctl reload                  # Reload configurations
initctl reboot                  # Reboot system
initctl shutdown                # Shutdown system
initctl ping                    # Test connectivity
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
- YAML configuration support
- VSOCK integration for Nitro Enclaves
- NSM driver loading
- Comprehensive documentation

---

