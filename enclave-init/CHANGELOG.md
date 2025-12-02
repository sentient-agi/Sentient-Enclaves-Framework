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

