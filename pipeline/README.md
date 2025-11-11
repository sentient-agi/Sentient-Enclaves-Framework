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
