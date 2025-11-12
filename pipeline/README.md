# Pipeline - Enclave's Secure Local Channel Protocol

## Overview

**Pipeline** is a secure local channel protocol implementation designed for AWS Nitro Enclaves communication. It provides a secure, encrypted communication channel between an enclave and its parent EC2 instance using VSOCK (Virtual Socket) protocol with cryptographic security.

## Architecture

Pipeline implements a client-server architecture for secure communication:

- **Server Mode (`listen`)**: Runs inside the enclave, listening for incoming connections
- **Client Mode (`run`)**: Executes commands remotely via the secure channel
- **File Transfer**: Secure file send/receive operations between host and enclave

### Key Components

1. **VSOCK Communication**: Uses AF_VSOCK sockets for enclave-to-host communication
2. **Cryptography**: Implements secure channel encryption (likely using the `cryptography` submodule)
3. **Configuration**: TOML-based configuration for flexible deployment
4. **CLI Interface**: Command-line interface for all operations

## Installation & Building

### Prerequisites

- Rust 1.91.0 or later
- AWS Nitro Enclaves SDK (for enclave deployment)
- Access to an EC2 instance with Nitro Enclave support

### Build from Source

```bash
# Clone the repository
cd pipeline

# Build the project
cargo build --release

# The binary will be available at:
# target/release/pipeline
```

## Configuration

Pipeline requires a configuration file in TOML format. By default, it looks for:
```
./.config/pipeline.config.toml
```

You can specify a custom config path using the `--config` flag.

### Configuration Structure

Create a configuration file at `.config/pipeline.config.toml`:

```toml
# Default VSOCK Context Identifier
# This is the CID of the enclave
cid = 3

# Default VSOCK port number
# Must match the port used by 'pipeline listen'
port = 5000

# The future configuration may include:
# - VSOCK connection parameters
# - Encryption settings
# - Timeout values
# - Buffer sizes
```

## Usage

### Basic Command Structure

```bash
pipeline [OPTIONS] <SUBCOMMAND>
```

### Global Options

- `--config <PATH>`: Specify custom configuration file path (default: `./.config/pipeline.config.toml`)

### Subcommands

#### 1. Listen Mode (Server)

Starts the Pipeline server, typically inside the enclave:

```bash
pipeline listen [OPTIONS]
```

This mode:
- Opens a VSOCK listener
- Accepts incoming connections from the host
- Processes commands and file transfers
- Maintains the secure channel

**Use case**: Run this inside your Nitro Enclave to accept connections from the parent EC2 instance.

#### 2. Run Mode (Execute Remote Command)

Executes a command on the remote Pipeline server:

```bash
pipeline run [OPTIONS] -- <COMMAND> [ARGS...]
```

This mode:
- Connects to the Pipeline server
- Sends the command for execution
- Returns the exit code of the remote command

**Example**:
```bash
# Execute a command inside the enclave (with output to local console)
pipeline run -- /usr/bin/my-secure-app --flag value

# Execute a command inside the enclave, without command output waiting (with output to enclave's debug console, when enclave is running in debug mode)
pipeline run --no-wait -- /usr/bin/my-secure-app --flag value

# The exit code will match the remote command's exit code
echo $?
```

#### 3. Send File

Securely sends a file to the remote endpoint:

```bash
pipeline send-file [OPTIONS] <SOURCE> <DESTINATION>
```

**Example**:
```bash
# Send a file to the enclave
pipeline send-file ./local-file.txt /enclave/path/file.txt
```

#### 4. Receive File

Securely receives a file from the remote endpoint:

```bash
pipeline recv-file [OPTIONS] <SOURCE> <DESTINATION>
```

**Example**:
```bash
# Receive a file from the enclave
pipeline recv-file /enclave/path/output.txt ./local-output.txt
```

## Typical Workflow

### Setup for Enclave Communication

1. **Inside the Enclave (Server)**:
```bash
# Start the Pipeline server
pipeline listen
```

2. **On the Host EC2 Instance (Client)**:
```bash
# Execute a command inside the enclave
pipeline run -- /app/process-data --input data.json

# Send a file into the enclave
pipeline send-file ./sensitive-data.bin /enclave/input/data.bin

# Receive processed results
pipeline recv-file /enclave/output/results.bin ./results.bin
```

## Security Features

- **Encrypted Channel**: All communications are encrypted using the cryptography module
- **Isolated Execution**: Runs within AWS Nitro Enclave's trusted execution environment
- **VSOCK Transport**: Uses VSOCK for secure, isolated network communication
- **Configuration Validation**: Validates configuration before establishing connections

## Project Structure

```
pipeline/
├── src/
│   ├── main.rs          # Entry point and CLI handler
│   ├── lib.rs           # Core library functions (listen, run, send_file, recv_file)
│   ├── cli.rs           # CLI app builder
│   ├── cli_parser.rs    # Argument parsing structures
│   ├── config.rs        # Configuration management
│   ├── vsock.rs         # VSOCK socket implementation
│   └── cats.rs          # ASCII art and easter eggs
├── cryptography/        # Cryptographic implementations
├── .config/             # Default configuration directory
└── Cargo.toml          # Project dependencies
```

## Dependencies

Key dependencies include:
- `clap` (4.5.45) - Command-line argument parsing
- `tokio` (1.47.1) - Async runtime
- `serde` (1.0.219) - Serialization/deserialization
- `toml` (0.8.23) - Configuration file parsing
- `nix` (0.26.4) - Unix system calls (for VSOCK)
- Various crypto libraries for secure communication

## Error Handling

- Configuration file errors: Ensure `.config/pipeline.config.toml` exists and is valid
- Connection errors: Verify VSOCK connectivity between host and enclave
- Permission errors: Ensure proper permissions for file operations

## Development

### Running Tests

```bash
cargo test
```

### Building for Enclave

When building for deployment inside a Nitro Enclave, ensure you're targeting the appropriate architecture and linking requirements.

## Easter Eggs 🐱

Pipeline includes some friendly ASCII art cats:
```bash
pipeline --"=(^\">,.•.,<\"^)="  # Meet George
pipeline --"=(^\",..,\"^)="      # Meet Pascal
```

## Troubleshooting

1. **"Missing configuration file" error**: Create `.config/pipeline.config.toml` or specify a valid config path
2. **Connection refused**: Ensure the Pipeline server is running in listen mode
3. **VSOCK errors**: Verify Nitro Enclave is properly configured and VSOCK support is enabled

## License

This project appears to be part of a larger Secure Enclaves Framework. Check the `LICENSE-APACHE` file in the repository root for licensing information.

## Related Projects

Pipeline is part of the Secure Enclaves Framework that includes:
- `pf-proxy` - Port forwarding proxy
- `ra-web-srv` - Remote attestation web service
- `fs-monitor` - Filesystem monitoring

---

## Quick Start Example

```bash
# 1. Create configuration
mkdir -vp .config
cat > .config/pipeline.config.toml << EOF
cid = 3
port = 5000
EOF

# 2. In your enclave, start the server
pipeline listen

# 3. From the host, interact with the enclave
pipeline run -- echo "Hello from enclave"
pipeline send-file data.txt /enclave/data.txt
pipeline recv-file /enclave/result.txt result.txt
```

---

# Pipeline - Complete CLI Reference

## Table of Contents
- [Overview](#overview)
- [Global Options](#global-options)
- [Subcommands](#subcommands)
  - [listen](#listen-server-mode)
  - [run](#run-execute-remote-command)
  - [send-file](#send-file-upload-to-enclave)
  - [recv-file](#recv-file-download-from-enclave)
- [Configuration File](#configuration-file)
- [VSOCK Parameters](#vsock-parameters)
- [Usage Examples](#usage-examples)
- [Easter Eggs](#easter-eggs)

---

## Overview

Pipeline is a **VSOCK-based secure communication protocol** for AWS Nitro Enclaves that enables:
- Remote command execution inside enclaves
- Bidirectional file transfer between host and enclave
- Encrypted channel communication
- Secure local channel protocol implementation

```
pipeline [GLOBAL_OPTIONS] <SUBCOMMAND> [SUBCOMMAND_OPTIONS]
```

---

## Global Options

### `--config`, `-c <PATH>`
**Type:** String (path)
**Required:** No
**Default:** `./.config/pipeline.config.toml`

Specifies the path to the configuration file containing default VSOCK parameters.

**Example:**
```bash
pipeline --config /etc/pipeline/custom.toml listen --port 5000
```

### `--help`, `-h`
Display help information for the command or subcommand.

```bash
pipeline --help
pipeline run --help
```

### `--version`, `-V`
Display the version information.

```bash
pipeline --version
```

---

## Subcommands

### `listen` (Server Mode)

**Purpose:** Start a Pipeline server that listens for incoming VSOCK connections. This mode is typically run **inside the enclave**.

**Syntax:**
```bash
pipeline listen --port <PORT>
```

#### Options

##### `--port <PORT>`
**Type:** u32
**Required:** Yes
**Description:** VSOCK port number to listen on

**Example:**
```bash
# Inside the enclave
pipeline listen --port 5000
```

**Behavior:**
- Opens a VSOCK listener on the specified port
- Accepts connections from the parent EC2 instance (host)
- Processes incoming commands and file transfer requests
- Runs indefinitely until terminated
- Handles multiple operations sequentially or concurrently (implementation dependent)

**Use Cases:**
- Running as a daemon inside Nitro Enclave
- Accepting remote commands from the host EC2 instance
- Serving as file transfer endpoint

---

### `run` (Execute Remote Command)

**Purpose:** Execute a command inside the enclave from the host EC2 instance.

**Syntax:**
```bash
pipeline run --cid <CID> --port <PORT> --command <COMMAND> [--no-wait]
```

#### Options

##### `--cid <CID>`
**Type:** u32
**Required:** Yes
**Description:** VSOCK Context Identifier (CID) of the target enclave

Common CID values:
- `3` - Typical CID for the first enclave on an EC2 instance
- `127` - Can be used in development/testing scenarios
- Custom CID assigned by AWS Nitro Hypervisor

##### `--port <PORT>`
**Type:** u32
**Required:** Yes
**Description:** VSOCK port number where the Pipeline server is listening

##### `--command <COMMAND>`
**Type:** String
**Required:** Yes
**Description:** Full command to execute inside the enclave (including arguments)

**Important:** Quote the entire command if it contains spaces or special characters.

##### `--no-wait`
**Type:** Boolean flag
**Required:** No
**Default:** false (wait for result)

When specified, Pipeline will:
- Send the command to the enclave
- Return immediately without waiting for execution completion
- The command output will appear in the enclave's debug console (if debug mode is enabled)
- The exit code will be 0 if the command was successfully dispatched

**Without `--no-wait` (default behavior):**
- Waits for command completion
- Returns stdout/stderr to the calling terminal
- Returns the actual exit code of the executed command

#### Examples

**Basic command execution:**
```bash
# Execute a simple command
pipeline run --cid 3 --port 5000 --command "echo 'Hello from enclave'"

# Run a script with arguments
pipeline run --cid 3 --port 5000 --command "/opt/app/process.sh --input data.json"

# Execute complex command with piping (quote properly)
pipeline run --cid 3 --port 5000 --command "cat /proc/cpuinfo | grep 'model name'"
```

**Fire-and-forget execution:**
```bash
# Start long-running process without waiting
pipeline run --cid 3 --port 5000 --command "/opt/app/long-job.sh" --no-wait

# Useful for background tasks
pipeline run --cid 3 --port 5000 --command "nohup /opt/app/daemon &" --no-wait
```

**Exit code handling:**
```bash
# Capture exit code
pipeline run --cid 3 --port 5000 --command "test -f /enclave/data/input.txt"
echo "Exit code: $?"

# Use in conditionals
if pipeline run --cid 3 --port 5000 --command "check-status.sh"; then
    echo "Command succeeded"
else
    echo "Command failed"
fi
```

---

### `send-file` (Upload to Enclave)

**Purpose:** Securely transfer a file from the host to the enclave.

**Syntax:**
```bash
pipeline send-file --cid <CID> --port <PORT> --localpath <LOCAL_PATH> --remotepath <REMOTE_PATH>
```

#### Options

##### `--cid <CID>`
**Type:** u32
**Required:** Yes
**Description:** VSOCK Context Identifier of the target enclave

##### `--port <PORT>`
**Type:** u32
**Required:** Yes
**Description:** VSOCK port number where Pipeline server is listening

##### `--localpath <LOCAL_PATH>`
**Type:** String (file path)
**Required:** Yes
**Description:** Path to the source file on the host filesystem

##### `--remotepath <REMOTE_PATH>`
**Type:** String (file path)
**Required:** Yes
**Description:** Destination path inside the enclave filesystem

#### Examples

```bash
# Send configuration file
pipeline send-file \
  --cid 3 \
  --port 5000 \
  --localpath ./config.json \
  --remotepath /app/config/settings.json

# Send binary data
pipeline send-file \
  --cid 3 \
  --port 5000 \
  --localpath ~/data/model.bin \
  --remotepath /enclave/models/ml-model.bin

# Send with absolute paths
pipeline send-file \
  --cid 3 \
  --port 5000 \
  --localpath /var/data/input.csv \
  --remotepath /tmp/processing/input.csv
```

**Behavior:**
- Reads file from local filesystem
- Transmits securely over VSOCK encrypted channel
- Writes to remote filesystem inside enclave
- Preserves file contents but NOT permissions/metadata
- Creates parent directories if they don't exist (implementation dependent)

**Error Conditions:**
- Local file doesn't exist or isn't readable
- Remote path isn't writable
- Insufficient disk space in enclave
- Connection errors to Pipeline server

---

### `recv-file` (Download from Enclave)

**Purpose:** Securely retrieve a file from the enclave to the host.

**Syntax:**
```bash
pipeline recv-file --cid <CID> --port <PORT> --remotepath <REMOTE_PATH> --localpath <LOCAL_PATH>
```

#### Options

##### `--cid <CID>`
**Type:** u32
**Required:** Yes
**Description:** VSOCK Context Identifier of the target enclave

##### `--port <PORT>`
**Type:** u32
**Required:** Yes
**Description:** VSOCK port number where Pipeline server is listening

##### `--remotepath <REMOTE_PATH>`
**Type:** String (file path)
**Required:** Yes
**Description:** Source path of the file inside the enclave

##### `--localpath <LOCAL_PATH>`
**Type:** String (file path)
**Required:** Yes
**Description:** Destination path on the host filesystem

#### Examples

```bash
# Retrieve computation results
pipeline recv-file \
  --cid 3 \
  --port 5000 \
  --remotepath /enclave/output/results.json \
  --localpath ./results/output.json

# Download logs
pipeline recv-file \
  --cid 3 \
  --port 5000 \
  --remotepath /var/log/app.log \
  --localpath ~/logs/enclave-app.log

# Fetch generated report
pipeline recv-file \
  --cid 3 \
  --port 5000 \
  --remotepath /tmp/report.pdf \
  --localpath /data/reports/$(date +%Y%m%d)-report.pdf
```

**Behavior:**
- Reads file from enclave filesystem
- Transmits securely over VSOCK encrypted channel
- Writes to local host filesystem
- Overwrites local file if it exists
- Preserves file contents

**Error Conditions:**
- Remote file doesn't exist or isn't readable
- Local path isn't writable
- Insufficient disk space on host
- Connection errors to Pipeline server

---

## Configuration File

**Location:** `./.config/pipeline.config.toml` (default)

The configuration file uses TOML format and defines default VSOCK parameters.

### Configuration Structure

```toml
# Default VSOCK Context Identifier
# This is the CID of the enclave
cid = 3

# Default VSOCK port number
# Must match the port used by 'pipeline listen'
port = 5000
```

### Configuration Fields

#### `cid`
**Type:** u32
**Description:** Default Context Identifier for VSOCK connections

Common values:
- `3` - First enclave CID (typical production value)
- `127` - Special CID for testing/development
- `4-16` - Additional enclaves on the same instance

#### `port`
**Type:** u32
**Description:** Default VSOCK port number

**Port Selection Guidelines:**
- Use ports above 1024 (non-privileged)
- Common range: 5000-65535
- Must match between client and server
- Avoid conflicts with other services

### Configuration Priority

Pipeline uses this priority order:
1. **Command-line arguments** (highest priority)
2. **Configuration file** (`--config` specified)
3. **Default configuration file** (`./.config/pipeline.config.toml`)
4. **Error** if no configuration found and required parameters missing

### Example Configurations

**Development Setup:**
```toml
# .config/pipeline.config.toml
cid = 127
port = 5555
```

**Production Setup:**
```toml
# /etc/pipeline/production.toml
cid = 3
port = 5000
```

**Multi-Enclave Setup:**
```toml
# .config/enclave-1.toml
cid = 3
port = 5000

# .config/enclave-2.toml
cid = 4
port = 5001
```

---

## VSOCK Parameters

### Understanding VSOCK

**VSOCK (Virtual Socket)** is a communication protocol for virtual machines and their host. In AWS Nitro Enclaves:

- **Host CID:** Always `3` (the parent EC2 instance)
- **Enclave CID:** Assigned by the hypervisor (typically starts at `3` for first enclave)
- **Port:** User-defined, similar to TCP ports

### Finding Your Enclave CID

```bash
# Inside the enclave, check CID
cat /proc/sys/kernel/hostname

# On the host, check running enclaves
nitro-cli describe-enclaves
```

### Port Selection Best Practices

1. **Consistency:** Use the same port across all components
2. **Documentation:** Document your port assignments
3. **Avoidance:** Don't use well-known ports (0-1024)
4. **Standardization:** Use environment-specific defaults

**Example Port Scheme:**
- Development: 5555
- Staging: 5001
- Production: 5000

---

## Usage Examples

### Complete Workflow Example

**Setup:**
```bash
# Create configuration
mkdir -p .config
cat > .config/pipeline.config.toml << EOF
cid = 3
port = 5000
EOF
```

**Inside Enclave (Server):**
```bash
# Start Pipeline server
pipeline listen --port 5000
```

**On Host (Client Operations):**

```bash
# 1. Check enclave connectivity
pipeline run --cid 3 --port 5000 --command "hostname"

# 2. Send input data
pipeline send-file \
  --cid 3 \
  --port 5000 \
  --localpath ./input.json \
  --remotepath /app/data/input.json

# 3. Process data
pipeline run \
  --cid 3 \
  --port 5000 \
  --command "/app/bin/processor --input /app/data/input.json --output /app/data/output.json"

# 4. Retrieve results
pipeline recv-file \
  --cid 3 \
  --port 5000 \
  --remotepath /app/data/output.json \
  --localpath ./output.json

# 5. Verify results
cat output.json
```

### Automation Script Example

```bash
#!/bin/bash
# deploy-and-run.sh

ENCLAVE_CID=3
ENCLAVE_PORT=5000

# Deploy application files
echo "Deploying application..."
pipeline send-file --cid $ENCLAVE_CID --port $ENCLAVE_PORT \
  --localpath ./app.bin --remotepath /app/app.bin

pipeline send-file --cid $ENCLAVE_CID --port $ENCLAVE_PORT \
  --localpath ./config.toml --remotepath /app/config.toml

# Run application
echo "Starting application..."
pipeline run --cid $ENCLAVE_CID --port $ENCLAVE_PORT \
  --command "chmod +x /app/app.bin && /app/app.bin --config /app/config.toml" \
  --no-wait

echo "Application started in background"
```

### Error Handling Example

```bash
#!/bin/bash

set -e  # Exit on error

# Function to check if enclave is reachable
check_enclave() {
    if ! pipeline run --cid 3 --port 5000 --command "echo 'ping'" > /dev/null 2>&1; then
        echo "ERROR: Cannot connect to enclave"
        exit 1
    fi
}

# Verify connectivity
check_enclave

# Send file with error handling
if pipeline send-file --cid 3 --port 5000 \
    --localpath ./data.bin --remotepath /enclave/data.bin; then
    echo "File transferred successfully"
else
    echo "ERROR: File transfer failed"
    exit 1
fi
```

### Multi-Enclave Management

```bash
#!/bin/bash
# manage-multiple-enclaves.sh

# Define enclaves
declare -A ENCLAVES=(
    ["enclave-1"]="3:5000"
    ["enclave-2"]="4:5001"
    ["enclave-3"]="5:5002"
)

# Execute command on all enclaves
for name in "${!ENCLAVES[@]}"; do
    IFS=':' read -r cid port <<< "${ENCLAVES[$name]}"

    echo "Executing on $name (CID: $cid, Port: $port)"
    pipeline run --cid "$cid" --port "$port" --command "$1"
done
```

---

## Easter Eggs

Pipeline includes friendly ASCII art cats as Easter eggs! 🐱

### George the BSH Cat

```bash
pipeline --george
# or
pipeline -g
```

Displays a beautiful ASCII art of George, a British Shorthair cat.

### Pascal the BSH Cat

```bash
pipeline --pascal
# or
pipeline -p
```

Displays a beautiful ASCII art of Pascal, another British Shorthair cat.

These commands output to stdout and exit with code 0, making them perfect for:
- Testing your Pipeline installation
- Brightening your terminal
- Showing some personality in automated scripts

**Fun fact:** The CLI identifiers are `=(^\">,.•.,<\"^)=` and `=(^\",..,\"^)=` which look like cat faces!

---

## Common Use Cases

### 1. Secure Computation
```bash
# Send sensitive data
pipeline send-file --cid 3 --port 5000 \
  --localpath ./sensitive.dat --remotepath /secure/input.dat

# Process in isolation
pipeline run --cid 3 --port 5000 \
  --command "/secure/compute --input /secure/input.dat --output /secure/result.dat"

# Retrieve results only
pipeline recv-file --cid 3 --port 5000 \
  --remotepath /secure/result.dat --localpath ./result.dat
```

### 2. Log Collection
```bash
# Fetch logs periodically
while true; do
    pipeline recv-file --cid 3 --port 5000 \
      --remotepath /var/log/app.log \
      --localpath "/logs/app-$(date +%s).log"
    sleep 300  # Every 5 minutes
done
```

### 3. Health Monitoring
```bash
# Check enclave health
pipeline run --cid 3 --port 5000 --command "systemctl status my-app"
echo "Health check exit code: $?"
```

### 4. Configuration Updates
```bash
# Update configuration without restart
pipeline send-file --cid 3 --port 5000 \
  --localpath ./new-config.toml --remotepath /app/config.toml

pipeline run --cid 3 --port 5000 \
  --command "killall -HUP app"  # Reload config
```

---

## Troubleshooting

### Connection Issues

**Problem:** `Connection refused` or timeout errors

**Solutions:**
1. Verify Pipeline server is running: `pipeline listen --port 5000`
2. Check CID is correct: `nitro-cli describe-enclaves`
3. Verify port matches between client and server
4. Check VSOCK is enabled in enclave configuration

### Configuration Errors

**Problem:** `Missing configuration file` error

**Solution:**
```bash
# Create default configuration
mkdir -p .config
cat > .config/pipeline.config.toml << EOF
cid = 3
port = 5000
EOF
```

### Permission Errors

**Problem:** Cannot read/write files

**Solutions:**
1. Check file permissions in enclave
2. Verify paths are absolute or relative correctly
3. Ensure parent directories exist

### Command Execution Failures

**Problem:** Commands fail with non-zero exit codes

**Debug approach:**
```bash
# Run with explicit paths
pipeline run --cid 3 --port 5000 --command "which my-command"

# Check environment
pipeline run --cid 3 --port 5000 --command "env"

# Test with simple command
pipeline run --cid 3 --port 5000 --command "echo 'test'"
```

---

## Advanced Topics

### Using with Custom Config

```bash
# Different configs for different environments
pipeline --config prod.toml run --command "app --mode production"
pipeline --config dev.toml run --command "app --mode development"
```

### Integration with CI/CD

```yaml
# .gitlab-ci.yml example
deploy-to-enclave:
  script:
    - pipeline send-file --cid 3 --port 5000 --localpath ./app --remotepath /app/app
    - pipeline run --cid 3 --port 5000 --command "systemctl restart app"
```

### Security Considerations

1. **Authentication:** Pipeline uses VSOCK isolation (physical security)
2. **Encryption:** Communication is encrypted via the cryptography module
3. **Access Control:** Only the host can communicate with its enclaves
4. **Audit Logging:** Implement logging on both sides for audit trails

---

## Summary

Pipeline provides a **secure, efficient, and easy-to-use** interface for enclave communication:

- **Server Mode (`listen`):** Run inside enclave to accept connections
- **Command Execution (`run`):** Execute commands remotely with or without waiting
- **File Upload (`send-file`):** Securely transfer files to enclave
- **File Download (`recv-file`):** Retrieve files from enclave
- **Configuration:** Flexible TOML-based configuration
- **VSOCK Protocol:** Native enclave communication support

---
