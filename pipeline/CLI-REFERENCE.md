# Pipeline - Complete CLI Reference

## Table of Contents
- [Overview](#overview)
- [Global Options](#global-options)
- [Subcommands](#subcommands)
  - [listen](#listen-server-mode)
  - [run](#run-execute-remote-command)
  - [send-file](#send-file-upload-to-enclave)
  - [recv-file](#recv-file-download-from-enclave)
  - [send-dir](#send-dir-upload-directory-to-enclave)
  - [recv-dir](#recv-dir-download-directory-from-enclave)
- [Configuration File](#configuration-file)
- [VSOCK Parameters](#vsock-parameters)
- [Usage Examples](#usage-examples)
- [Easter Eggs](#easter-eggs)

---

## Overview

Pipeline is a **VSOCK-based secure communication protocol** for AWS Nitro Enclaves that enables:
- Remote command execution inside enclaves
- Bidirectional file transfer between host and enclave
- **Recursive directory transfer** between host and enclave (NEW)
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
- Processes incoming commands, file transfer requests, and directory transfer requests
- Runs indefinitely until terminated
- Handles multiple operations sequentially or concurrently (implementation dependent)

**Use Cases:**
- Running as a daemon inside Nitro Enclave
- Accepting remote commands from the host EC2 instance
- Serving as file and directory transfer endpoint

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
- Creates parent directories if they don't exist

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
- Creates parent directories if they don't exist
- Preserves file contents

**Error Conditions:**
- Remote file doesn't exist or isn't readable
- Local path isn't writable
- Insufficient disk space on host
- Connection errors to Pipeline server

---

### `send-dir` (Upload Directory to Enclave)

**Purpose:** Recursively transfer an entire directory from the host to the enclave, preserving the directory structure.

**Syntax:**
```bash
pipeline send-dir --cid <CID> --port <PORT> --localdir <LOCAL_DIR> --remotedir <REMOTE_DIR>
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

##### `--localdir <LOCAL_DIR>`
**Type:** String (directory path)
**Required:** Yes
**Description:** Path to the source directory on the host filesystem

##### `--remotedir <REMOTE_DIR>`
**Type:** String (directory path)
**Required:** Yes
**Description:** Destination directory path inside the enclave filesystem

#### Examples

```bash
# Send application bundle
pipeline send-dir \
  --cid 3 \
  --port 5000 \
  --localdir ./my-application \
  --remotedir /enclave/app

# Send ML model directory with all weights and configs
pipeline send-dir \
  --cid 3 \
  --port 5000 \
  --localdir ~/models/llm-v2 \
  --remotedir /enclave/models/llm-v2

# Send configuration directory
pipeline send-dir \
  --cid 3 \
  --port 5000 \
  --localdir /etc/myapp \
  --remotedir /enclave/config
```

**Behavior:**
- Recursively traverses the local directory
- Collects all files with their relative paths
- Creates the destination directory in the enclave
- Creates all necessary subdirectories automatically
- Transfers each file while preserving relative path structure
- Shows progress for each file transfer
- Reports total number of files transferred

**Directory Structure Preservation:**
```
Local: ./my-app/                    Remote: /enclave/app/
├── bin/                     ->     ├── bin/
│   └── app                         │   └── app
├── config/                  ->     ├── config/
│   ├── settings.toml               │   ├── settings.toml
│   └── secrets.json                │   └── secrets.json
└── data/                    ->     └── data/
    ├── input/                          ├── input/
    │   └── data.csv                    │   └── data.csv
    └── models/                         └── models/
        └── weights.bin                     └── weights.bin
```

**Error Conditions:**
- Local directory doesn't exist
- Local path is not a directory
- Insufficient disk space in enclave
- Permission errors on remote filesystem
- Connection errors to Pipeline server

---

### `recv-dir` (Download Directory from Enclave)

**Purpose:** Recursively retrieve an entire directory from the enclave to the host, preserving the directory structure.

**Syntax:**
```bash
pipeline recv-dir --cid <CID> --port <PORT> --localdir <LOCAL_DIR> --remotedir <REMOTE_DIR>
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

##### `--localdir <LOCAL_DIR>`
**Type:** String (directory path)
**Required:** Yes
**Description:** Destination directory path on the host filesystem

##### `--remotedir <REMOTE_DIR>`
**Type:** String (directory path)
**Required:** Yes
**Description:** Path to the source directory inside the enclave filesystem

#### Examples

```bash
# Retrieve computation results
pipeline recv-dir \
  --cid 3 \
  --port 5000 \
  --localdir ./results \
  --remotedir /enclave/output

# Download all logs
pipeline recv-dir \
  --cid 3 \
  --port 5000 \
  --localdir ./collected-logs \
  --remotedir /var/log/myapp

# Retrieve processed dataset
pipeline recv-dir \
  --cid 3 \
  --port 5000 \
  --localdir ~/downloads/processed-data \
  --remotedir /enclave/data/processed
```

**Behavior:**
- Requests directory listing from enclave
- Creates the local destination directory
- Creates all necessary subdirectories automatically
- Transfers each file while preserving relative path structure
- Shows progress for each file transfer
- Reports total number of files transferred

**Directory Structure Preservation:**
```
Remote: /enclave/output/            Local: ./results/
├── reports/                 ->     ├── reports/
│   ├── summary.pdf                 │   ├── summary.pdf
│   └── detailed.json               │   └── detailed.json
├── logs/                    ->     ├── logs/
│   ├── app.log                     │   ├── app.log
│   └── errors.log                  │   └── errors.log
└── data/                    ->     └── data/
    └── output.csv                      └── output.csv
```

**Error Conditions:**
- Remote directory doesn't exist
- Remote directory is empty
- Insufficient disk space on host
- Permission errors on local filesystem
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

# 2. Send input data (single file)
pipeline send-file \
  --cid 3 \
  --port 5000 \
  --localpath ./input.json \
  --remotepath /app/data/input.json

# 3. Send entire application directory (NEW!)
pipeline send-dir \
  --cid 3 \
  --port 5000 \
  --localdir ./my-app \
  --remotedir /app

# 4. Process data
pipeline run \
  --cid 3 \
  --port 5000 \
  --command "/app/bin/processor --input /app/data/input.json --output /app/output/"

# 5. Retrieve single result file
pipeline recv-file \
  --cid 3 \
  --port 5000 \
  --remotepath /app/output/result.json \
  --localpath ./result.json

# 6. Retrieve entire output directory (NEW!)
pipeline recv-dir \
  --cid 3 \
  --port 5000 \
  --localdir ./output \
  --remotedir /app/output

# 7. Verify results
ls -la ./output/
```

### Directory Transfer Examples

#### Deploy Full Application
```bash
#!/bin/bash
# deploy-app.sh - Deploy complete application to enclave

ENCLAVE_CID=3
ENCLAVE_PORT=5000

echo "Deploying application to enclave..."

# Send entire application directory
pipeline send-dir \
  --cid $ENCLAVE_CID \
  --port $ENCLAVE_PORT \
  --localdir ./dist/my-application \
  --remotedir /app

echo "Starting application..."
pipeline run \
  --cid $ENCLAVE_CID \
  --port $ENCLAVE_PORT \
  --command "chmod +x /app/bin/* && /app/bin/start.sh" \
  --no-wait

echo "Application deployed successfully!"
```

#### Collect All Logs
```bash
#!/bin/bash
# collect-logs.sh - Collect all logs from enclave

ENCLAVE_CID=3
ENCLAVE_PORT=5000
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_DIR="./logs_$TIMESTAMP"

echo "Collecting logs from enclave..."

pipeline recv-dir \
  --cid $ENCLAVE_CID \
  --port $ENCLAVE_PORT \
  --localdir "$LOG_DIR" \
  --remotedir /var/log/app

echo "Logs saved to $LOG_DIR"
ls -la "$LOG_DIR"
```

#### ML Model Deployment
```bash
#!/bin/bash
# deploy-model.sh - Deploy ML model with all artifacts

ENCLAVE_CID=3
ENCLAVE_PORT=5000
MODEL_NAME=$1

if [ -z "$MODEL_NAME" ]; then
  echo "Usage: deploy-model.sh <model-name>"
  exit 1
fi

echo "Deploying model: $MODEL_NAME"

# Send model directory with weights, config, tokenizer, etc.
pipeline send-dir \
  --cid $ENCLAVE_CID \
  --port $ENCLAVE_PORT \
  --localdir "./models/$MODEL_NAME" \
  --remotedir "/enclave/models/$MODEL_NAME"

# Verify deployment
pipeline run \
  --cid $ENCLAVE_CID \
  --port $ENCLAVE_PORT \
  --command "ls -la /enclave/models/$MODEL_NAME"

echo "Model $MODEL_NAME deployed successfully!"
```

### Automation Script Example

```bash
#!/bin/bash
# deploy-and-run.sh

ENCLAVE_CID=3
ENCLAVE_PORT=5000

# Deploy application files (directory transfer)
echo "Deploying application..."
pipeline send-dir --cid $ENCLAVE_CID --port $ENCLAVE_PORT \
  --localdir ./app --remotedir /app

# Deploy configuration files
echo "Deploying configuration..."
pipeline send-dir --cid $ENCLAVE_CID --port $ENCLAVE_PORT \
  --localdir ./config --remotedir /app/config

# Run application
echo "Starting application..."
pipeline run --cid $ENCLAVE_CID --port $ENCLAVE_PORT \
  --command "chmod +x /app/bin/* && /app/bin/app --config /app/config/settings.toml" \
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

# Send directory with error handling
if pipeline send-dir --cid 3 --port 5000 \
    --localdir ./data --remotedir /enclave/data; then
    echo "Directory transferred successfully"
else
    echo "ERROR: Directory transfer failed"
    exit 1
fi

# Receive directory with error handling
if pipeline recv-dir --cid 3 --port 5000 \
    --localdir ./results --remotedir /enclave/output; then
    echo "Results retrieved successfully"
else
    echo "ERROR: Failed to retrieve results"
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

# Deploy to all enclaves
for name in "${!ENCLAVES[@]}"; do
    IFS=':' read -r cid port <<< "${ENCLAVES[$name]}"

    echo "Deploying to $name (CID: $cid, Port: $port)"

    # Deploy application
    pipeline send-dir --cid "$cid" --port "$port" \
        --localdir ./app --remotedir /app

    # Start application
    pipeline run --cid "$cid" --port "$port" \
        --command "/app/bin/start.sh" --no-wait
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

### 1. Secure Computation with Directories
```bash
# Send entire input dataset directory
pipeline send-dir --cid 3 --port 5000 \
  --localdir ./datasets/experiment-001 --remotedir /secure/input

# Process in isolation
pipeline run --cid 3 --port 5000 \
  --command "/secure/compute --input-dir /secure/input --output-dir /secure/output"

# Retrieve all results
pipeline recv-dir --cid 3 --port 5000 \
  --localdir ./results/experiment-001 --remotedir /secure/output
```

### 2. Application Deployment
```bash
# Deploy entire application with all dependencies
pipeline send-dir --cid 3 --port 5000 \
  --localdir ./dist/my-secure-app --remotedir /app

# Set permissions and start
pipeline run --cid 3 --port 5000 \
  --command "chmod -R +x /app/bin && /app/bin/start.sh"
```

### 3. Log Collection
```bash
# Fetch all logs periodically
while true; do
    TIMESTAMP=$(date +%Y%m%d_%H%M%S)
    pipeline recv-dir --cid 3 --port 5000 \
      --localdir "/logs/snapshot_$TIMESTAMP" \
      --remotedir /var/log/app
    sleep 3600  # Every hour
done
```

### 4. Configuration Updates
```bash
# Update entire configuration directory
pipeline send-dir --cid 3 --port 5000 \
  --localdir ./new-config --remotedir /app/config

# Reload application
pipeline run --cid 3 --port 5000 \
  --command "killall -HUP app"  # Reload config
```

### 5. Backup and Restore
```bash
# Backup entire data directory from enclave
pipeline recv-dir --cid 3 --port 5000 \
  --localdir "./backups/$(date +%Y%m%d)" \
  --remotedir /enclave/data

# Restore from backup
pipeline send-dir --cid 3 --port 5000 \
  --localdir ./backups/20240115 \
  --remotedir /enclave/data
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

**Problem:** Cannot read/write files or directories

**Solutions:**
1. Check file/directory permissions in enclave
2. Verify paths are absolute or relative correctly
3. Ensure parent directories exist (they are created automatically for files)

### Directory Transfer Issues

**Problem:** `Local directory does not exist` error

**Solutions:**
1. Verify the local directory path is correct
2. Use absolute paths if relative paths are ambiguous
3. Check that the path points to a directory, not a file

**Problem:** `Remote directory is empty or does not exist` error

**Solutions:**
1. Verify the remote directory exists in the enclave
2. Check if the directory contains any files
3. Ensure proper permissions on the remote directory

**Problem:** Directory transfer seems slow

**Solutions:**
1. Large directories with many small files may take longer due to per-file overhead
2. Consider archiving large directories before transfer if speed is critical
3. Check network/VSOCK throughput

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
    # Deploy entire application directory
    - pipeline send-dir --cid 3 --port 5000 --localdir ./dist --remotedir /app
    # Restart application
    - pipeline run --cid 3 --port 5000 --command "systemctl restart app"
    # Collect deployment logs
    - pipeline recv-dir --cid 3 --port 5000 --localdir ./deploy-logs --remotedir /var/log/deploy
```

### Security Considerations

1. **Authentication:** Pipeline uses VSOCK isolation (physical security)
2. **Encryption:** Communication is encrypted via the cryptography module
3. **Access Control:** Only the host can communicate with its enclaves
4. **Audit Logging:** Implement logging on both sides for audit trails
5. **Directory Integrity:** All files in directory transfers maintain their integrity

---

## Summary

Pipeline provides a **secure, efficient, and easy-to-use** interface for enclave communication:

- **Server Mode (`listen`):** Run inside enclave to accept connections
- **Command Execution (`run`):** Execute commands remotely with or without waiting
- **File Upload (`send-file`):** Securely transfer files to enclave
- **File Download (`recv-file`):** Retrieve files from enclave
- **Directory Upload (`send-dir`):** Recursively transfer directories to enclave (NEW)
- **Directory Download (`recv-dir`):** Recursively retrieve directories from enclave (NEW)
- **Configuration:** Flexible TOML-based configuration
- **VSOCK Protocol:** Native enclave communication support

---
