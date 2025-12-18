# `rbuilds.sh` – Reproducible Builds System for AWS Nitro Enclaves

`rbuilds.sh` is the reproducible build orchestrator for the **Sentient Enclaves Framework**.
It turns an application Dockerfile into a fully configured **AWS Nitro Enclaves EIF** image with:

- A custom **Linux kernel** tuned for Nitro Enclaves (vsock, Netfilter, NAT).
- A custom **init system** (C/clang-based init and a brand new `enclave-init` written in Rust).
- **Secure Local Channel (SLC)** for VSock-based binary protocol for executinon commands in enclave from host, file and directory transfer.
- **Forward / reverse / transparent proxies** for enclave networking.
- **Remote attestation web server**.
- **File system monitor** for:
  - handling any FS changing events via `inotify`
  - make per-file attestation starting with files hashing scheme (SHA3-512)
  - and pairing with remote attestation web server through NATS KV storage (enclave service bus) to producing VRF proofs for hashes and generate attestation documents, which are includes hash and proof for files.

This README documents `rbuilds.sh` end-to-end: quick start, build pipeline stages, CLI reference, and how the init systems and runtime components fit together.

---

## Table of Contents

1. [Conceptual Overview](#conceptual-overview)
2. [Features](#features)
3. [Prerequisites & Environment](#prerequisites--environment)
4. [Repository Layout](#repository-layout)
5. [Quick Start](#quick-start)
6. [Build Pipeline & Stages](#build-pipeline--stages)
7. [CLI Usage & Global Options](#cli-usage--global-options)
8. [Command Reference (`--cmd`)](#command-reference---cmd)
9. [Interactive & Automation Shell Mode](#interactive--automation-shell-mode)
10. [What Gets Packaged Into the EIF](#what-gets-packaged-into-the-eif)
11. [Init Systems](#init-systems)
    - [Current C/clang-based init](#current-cclang-based-init)
    - [New Rust `enclave-init`](#new-rust-enclave-init)
12. [CI / Automation Patterns](#ci--automation-patterns)
13. [Troubleshooting & Tips](#troubleshooting--tips)

---

## Conceptual Overview

At a high level, `rbuilds.sh`:

1. Builds a **custom Linux kernel** that supports Nitro Enclaves, vsock, and the necessary Netfilter/NAT stack.
2. Builds the **init system** that runs as PID 1 inside the enclave and orchestrates all services.
3. Builds the enclave’s **userland components**:
   - Secure Local Channel (SLC) tools.
   - Forward / reverse / transparent VSock-based proxies.
   - Remote attestation web server.
   - File system monitor for per-file attestation.
4. Assembles everything into an **EIF image** using standard Linux/BSD CLI tools (`bsd-tar`, etc.) and `nitro-cli`.
5. Provides commands for **creating, listing, attaching to, and destroying** enclaves, and for running EIF images in both debug and normal modes.

`rbuilds.sh` is:

- **Reproducible** – deterministic build pipeline for kernel, init, and userland.
- **Composable** – split into clearly defined stages (kernel, init, framework components/apps/services, user apps/services, rootfs, EIF, enclave).
- **Operator-friendly** – offers a CLI (`--cmd`) and an interactive+automation shell mode suitable for local development and CI.

---

## Features

- **Reproducible builds** of enclave images with pinned toolchains and configurations.
- **End-to-end pipeline** – from a Dockerfile to a running Nitro Enclave.
- **Custom kernel** tuned for:
  - vsock.
  - Netfilter and NAT for transparent proxies.
- **Secure Local Channel (SLC)**:
  - Execute commands inside the enclave via VSock.
  - Transfer files and directories via VSock.
- **Networking proxies**:
  - Forward proxies.
  - Reverse proxies.
  - Transparent proxies (with iptables/Netfilter integration).
- **Remote attestation web server**:
  - Attestation document handling.
  - Integration with KMS and external verifiers (by design).
- **File system monitor**:
  - `inotify`-based monitoring of virtual enclave file systems (CoW FS hashing layer for enclave ramdisk/`initramfs`).
  - Per-file attestation of runtime data.
- **Multiple init implementations**:
  - Current C/clang-based init.
  - New Rust-based `enclave-init` (designed to replace the former, build integration is in progress).
- **Interactive command shell and automation mode**:
  - For orchestrating builds and enclave lifecycle via stdin (useful for CI).

---

## Prerequisites & Environment

### Platform

- An **AWS EC2 instance** that supports **AWS Nitro Enclaves**:
  - Nitro-based instance family.
  - Nitro Enclaves feature enabled in the EC2 instance configuration.

### Host OS & Tools

On the host EC2 instance you should have:

- A modern Linux distribution (e.g., Amazon Linux 2023 or equivalent).
- **Docker** or a compatible container runtime.
- **Nitro CLI**: `nitro-cli` installed and configured.
- Standard Unix tools:
  - `bash`
  - `time` (`/usr/bin/time`)
  - `tee`
  - `mkdir` and other `coreutils`.

### Important Note

`rbuilds.sh` assumes it is run from the **project root** of the `sentient-enclaves-framework` repository:

```bash
git clone https://github.com/sentient-agi/Sentient-Enclaves-Framework.git
cd Sentient-Enclaves-Framework
./rbuilds/rbuilds.sh [...]
```

---

## Repository Layout

Only the relevant pieces for `rbuilds.sh` are listed here:

- `rbuilds/rbuilds.sh`
  Main build orchestrator shell script.

- `rbuilds/enclave.init/`
  Current (legacy) C/clang-based init system scripts, e.g. (for instance):
  - `init_revp+tpp.sh` – orchestrates startup of proxies (forward/reverse/transparent), SLC, attestation server, FS monitor, and the main applications.

- `rbuilds/rootfs_base/cmd`
  Default **command** to run (by init system) inside the enclave (primary entrypoint for init scripts that running framework components and userland apps).

- `rbuilds/rootfs_base/env`
  Default **environment variables** for the enclave init (VSock ports/CIDs, proxy modes, etc.).

- `enclave-init/`
  Rust-based enclave's init system, see:
  - [`enclave-init/README.md`](enclave-init/README.md)
  - [`enclave-init/src/main.rs`](enclave-init/src/main.rs)

- Build and cache directories (created/populated by `rbuilds.sh`):
  - `.docker/` – Docker-related artifacts, Dockerfiles.
  - `.linux/` – kernel build outputs, kernel `sysctl` and `systemd` init configurations for enclave host system.
  - `.bin/` – auxiliary binaries / toolchain artifacts.
  - `eif/` – EIF images and related logs.

- Userland components included in the EIF (paths may vary in repo):
  - Pipeline SLC (secure VSock channel).
  - Proxies (forward, reverse, transparent).
  - RA web server.
  - File system monitor.

---

## Quick Start

This section walks through a minimal build → run → debug flow.

### 1. Prepare an Application Dockerfile

You need a Dockerfile that defines the application environment to be placed inside the enclave. For example:

```text
./pipeline-slc-network-al2023.dockerfile
```

Typical steps in that Dockerfile:

- Start with Amazon Linux 2023 (or another supported base image).
- Install your application and all runtime dependencies.
- Optionally configure user, working directory, app entrypoint and app environment.

### 2. Build Everything (Kernel, Init, Apps, EIF, Enclave)

From the project root:

```bash
mkdir -vp ./eif/; \
/usr/bin/time -v -o ./eif/make_build.log \
./rbuilds/rbuilds.sh \
  --tty \
  --debug \
  --dockerfile ./pipeline-slc-network-al2023.dockerfile \
  --network \
  --init-c \
  --cmd "make_all" \
  2>&1 3>&1 | tee ./eif/make_build.output
```

What this does:

- Creates `./eif/` as the output directory for EIFs and logs.
- Runs the **full pipeline** (`make_nitro`, `make_kernel`, `make_apps`, `make_init`, `make_eif`, `make_enclave`).
- Records detailed resource usage in `./eif/make_build.log`.
- Streams build output to both the terminal and `./eif/make_build.output`.

**About `3>&1`:**

- The `--tty` flag uses file descriptor `3` as the pseudo-TTY.
- `3>&1` merges that TTY output back into your shell so you see everything in real time.

### 3. Run the Enclave

Once an EIF is built and registered, you can run it via:

```bash
# Debug mode with CLI-style console
./rbuilds/rbuilds.sh --tty --debug --network --init-c \
  --cmd "run_eif_image_debugmode_cli" 2>&1 3>&1

# Normal mode
./rbuilds/rbuilds.sh --tty --debug --network --init-c \
  --cmd "run_eif_image" 2>&1 3>&1
```

### 4. Attach, Inspect, and Tear Down

Attach to a running enclave console:

```bash
./rbuilds/rbuilds.sh --tty --debug --network --init-c \
  --cmd "attach_console_to_enclave" 2>&1 3>&1
```

List enclaves:

```bash
./rbuilds/rbuilds.sh --tty --debug --network --init-c \
  --cmd "list_enclaves" 2>&1 3>&1
```

Drop (terminate) a single enclave (recently created):

```bash
./rbuilds/rbuilds.sh --tty --debug --network --init-c \
  --cmd "drop_enclave" 2>&1 3>&1
```

Drop all enclaves:

```bash
./rbuilds/rbuilds.sh --tty --debug --network --init-c \
  --cmd "drop_enclaves_all" 2>&1 3>&1
```

---

## Build Pipeline & Stages

`rbuilds.sh` exposes build "stages" via the `--cmd` option. You can run them individually or via the `make_all` meta-command.

Common pattern:

```bash
./rbuilds/rbuilds.sh \
  --tty \
  --debug \
  --dockerfile ./pipeline-slc-network-al2023.dockerfile \
  --network \
  --init-c \
  --cmd "make_<stage>" \
  2>&1 3>&1
```

### Stage Overview

- `make_nitro`
  Initialize Nitro-related build environment, setup Nitro run-time on EC2 instance and base artifacts.

- `make_clear`
  Clean build artifacts and reset state.

- `make_kernel`
  Build the custom Linux kernel with Nitro + vsock + Netfilter.

- `make_apps`
  Build userland components:
  - Pipeline SLC.
  - Proxies (forward, reverse, transparent).
  - RA web server.
  - FS monitor.

- `make_init`
  Build and stage the init system (C-based today, Rust-based will be included later).

- `make_eif`
  Package kernel, init, and rootfs into an EIF image.

- `make_enclave`
  Create and configure an enclave from the generated EIF.

- `make_all`
  Run all of the above in the correct order.

---

## CLI Usage & Global Options

You can use `rbuilds.sh` in:

1. **CLI mode** (with `--cmd`), or
2. **Interactive/automation shell mode** (with commands via stdin).

### Basic CLI Pattern

```bash
./rbuilds/rbuilds.sh [GLOBAL OPTIONS] --cmd "<command>" [redirects]
```

### Global Options

#### `--tty`

Enable a TTY for Docker / internal processes.
Use with `3>&1` so TTY output is visible:

```bash
./rbuilds/rbuilds.sh --tty --debug --cmd "make_nitro" 2>&1 3>&1
```

#### `--debug`

Enable verbose logging / tracing for:

- `rbuilds.sh` itself.
- Nested build steps (kernel, Docker builds, etc.).

Recommended for all development and CI runs.

#### `--dockerfile <path>`

Specify the application Dockerfile used to construct the enclave root filesystem:

```bash
--dockerfile ./pipeline-slc-network-al2023.dockerfile
```

Required for any stage that needs the app rootfs (`make_all`, `make_apps`, `make_eif`, etc.).

#### `--network`

Enable network access in the enclave. Activate both, forward and reverse port forwarding proxies. (`--reverse-network` + `--forward-network`)

- Allows apps in enclave to pull packages/images/model files.
- Allows apps and other tools to reach AWS endpoints, like KMS, if required.
- Expose API endpoints of apps to host. (For exposing these endpoints further from host - routing and DNS forwarding setup required.)

#### `--init-c`

Select the C/clang-based init system as the init implementation to bundle into the EIF.

> When the Rust `enclave-init` will be fully tested and integrated, expect an alternative flag (e.g., `--init-rust`) to select it. For now, `--init-c` is the stable, supported path.

#### `--cmd "<command>"`

Specify the operation to perform.
Typical values include:

- Build-related:
  - `make_nitro`
  - `make_clear`
  - `make_kernel`
  - `make_apps`
  - `make_init`
  - `make_eif`
  - `make_enclave`
  - `make_all`
- Enclave lifecycle:
  - `run_eif_image_debugmode_cli`
  - `run_eif_image`
  - `attach_console_to_enclave`
  - `list_enclaves`
  - `drop_enclave`
  - `drop_enclaves_all`

#### `--memory`, `--cpus` and `--cid`

Resources allocation for Nitro run-time setup (for Nitro allocator service configuration) and CID set for enclave.

#### `--man` and `--info`, `-h|-hh|-hhh` and `-?|-??|-???` - help and manual output options

Built-in help:

```bash
./rbuilds/rbuilds.sh --man
./rbuilds/rbuilds.sh --info
```

- `--man` – Print extended help & man strings, detailed manual (options, commands, notes).
- `--info` – Print exhaustive documentation.

- `-?|-h` - Print CLI keys/args/options/parameters help
- `-??|-hh` - Print extended help
- `-???|-hhh` - Print extended help with man messages/strings

---

## Command Reference – `--cmd`

Below is a summary of the main commands.
Use `./rbuilds/rbuilds.sh --man` for the authoritative, version-specific reference.

### Build / Maintenance Commands

#### `make_nitro`

Initialize Nitro-related build environment:

```bash
./rbuilds/rbuilds.sh --tty --debug --cmd "make_nitro" 2>&1 3>&1
```

Typical responsibilities:

- Check / prepare `nitro-cli` and Nitro allocator service.
- Resources allocation for Nitro run-time setup (for Nitro allocator service configuration) and CID set for enclave.
- Prepare and setup packages, base images or helper containers into system.
- Initialize internal directories.

#### `make_clear`

Clean build artifacts:

```bash
./rbuilds/rbuilds.sh --tty --debug --cmd "make_clear" 2>&1 3>&1
```

Removes intermediate build containers, outputs, allowing for a fully fresh build.

#### `make_all`

Run the full build pipeline:

```bash
./rbuilds/rbuilds.sh --tty --debug \
  --dockerfile ./pipeline-slc-network-al2023.dockerfile \
  --network --init-c \
  --cmd "make_all" 2>&1 3>&1
```

#### `make_kernel`

Build the custom Linux kernel:

```bash
./rbuilds/rbuilds.sh --tty --debug \
  --dockerfile ./pipeline-slc-network-al2023.dockerfile \
  --network --init-c \
  --cmd "make_kernel" 2>&1 3>&1
```

Outputs kernel artifact(s) into `./kernel_blobs/` directory.

#### `make_apps`

Build userland binaries:

```bash
./rbuilds/rbuilds.sh --tty --debug \
  --dockerfile ./pipeline-slc-network-al2023.dockerfile \
  --network --init-c \
  --cmd "make_apps" 2>&1 3>&1
```

Includes:

- Pipeline SLC tool.
- Forward/reverse/transparent proxies.
- Attestation web server.
- File system monitor.

#### `make_init`

Build and stage the init system:

```bash
./rbuilds/rbuilds.sh --tty --debug \
  --dockerfile ./pipeline-slc-network-al2023.dockerfile \
  --network --init-c \
  --cmd "make_init" 2>&1 3>&1
```

Uses:

- `rbuilds/enclave.init/` (current C/clang based init scripts path).
- `rbuilds/rootfs_base/env` and `rbuilds/rootfs_base/cmd`.

#### `make_eif`

Package everything into an EIF:

```bash
./rbuilds/rbuilds.sh --tty --debug \
  --dockerfile ./pipeline-slc-network-al2023.dockerfile \
  --network --init-c \
  --cmd "make_eif" 2>&1 3>&1
```

Generates `.eif` files under `./eif/`.

#### `make_enclave`

Create/configure an enclave from the EIF:

```bash
./rbuilds/rbuilds.sh --tty --debug \
  --dockerfile ./pipeline-slc-network-al2023.dockerfile \
  --network --init-c \
  --cmd "make_enclave" 2>&1 3>&1
```

---

### Enclave Lifecycle Commands

Assume you already have an EIF image built and available.

#### `run_eif_image_debugmode_cli`

Run the enclave in debug mode with console:

```bash
./rbuilds/rbuilds.sh --tty --debug --network --init-c \
  --cmd "run_eif_image_debugmode_cli" 2>&1 3>&1
```

Good for:

- Live debugging.
- Verifying that init and services start correctly.

#### `run_eif_image`

Run the enclave normally:

```bash
./rbuilds/rbuilds.sh --tty --debug --network --init-c \
  --cmd "run_eif_image" 2>&1 3>&1
```

Use this for non-interactive test runs or as a basis for production deployment.

#### `attach_console_to_enclave`

Attach a console to an already-running enclave (that running in `debugmode_cli`):

```bash
./rbuilds/rbuilds.sh --tty --debug --network --init-c \
  --cmd "attach_console_to_enclave" 2>&1 3>&1
```

#### `list_enclaves`

List active enclaves on the host:

```bash
./rbuilds/rbuilds.sh --tty --debug --network --init-c \
  --cmd "list_enclaves" 2>&1 3>&1
```

#### `drop_enclave`

Drop/terminate currently built and running enclave by its name (according to the name of the `dockerfile`):

```bash
./rbuilds/rbuilds.sh --tty --debug --network --init-c \
  --cmd "drop_enclave" 2>&1 3>&1
```

#### `drop_recent_enclave`

Drop/terminate currently built and running enclave by its enclave ID in Nitro Enclaves runtime:

```bash
./rbuilds/rbuilds.sh --tty --debug --network --init-c \
  --cmd "drop_recent_enclave" 2>&1 3>&1
```

#### `drop_enclaves_all`

Terminate all enclaves:

```bash
./rbuilds/rbuilds.sh --tty --debug --network --init-c \
  --cmd "drop_enclaves_all" 2>&1 3>&1
```

---

## Interactive & Automation Shell Mode

Besides `--cmd`, `rbuilds.sh` can act as a small shell, a REPL, that reads commands either from own shell prompt (interactive shell mode) or from stdin (automation shell mode).
This is useful for:
- Interactive exploration.
- CI or other orchestration tools that want to script multiple commands.

### Interactive Shell

Start:

```bash
mkdir -vp ./eif/; \
/usr/bin/time -v -o ./eif/make_build.log \
./rbuilds/rbuilds.sh 2>&1 | tee ./eif/make_build.output
```

Then, at the prompt, type commands such as:

```text
make all
make eif
attach_console_to_enclave
list_enclaves
drop_enclave
drop_enclaves_all
```

### Automation via stdin

You can pipe commands directly:

```bash
{ echo "attach_console_to_enclave"; } | ./rbuilds/rbuilds.sh 2>&1
{ echo "list_enclaves"; }            | ./rbuilds/rbuilds.sh 2>&1
{ echo "drop_enclave"; }             | ./rbuilds/rbuilds.sh 2>&1
{ echo "drop_enclaves_all"; }        | ./rbuilds/rbuilds.sh 2>&1
```

With timing + logs:

```bash
{ echo "attach_console_to_enclave"; } \
  | /usr/bin/time -v -o ./eif/make_build.log \
    ./rbuilds/rbuilds.sh 2>&1 | tee ./eif/make_build.output
```

For build commands that ask interactive questions, you can pre-feed answers:

```bash
{ echo "make eif";
  echo "y";
  echo "y";
  echo "y";
  echo "y";
  echo "y";
  echo "y";
  echo "y"; } \
  | /usr/bin/time -v -o ./make_build.log \
    ./rbuilds/rbuilds.sh 2>&1 | tee ./make_build.output
```

Or a full build:

```bash
{ echo "make all"; } \
  | /usr/bin/time -v -o ./make_build.log \
    ./rbuilds/rbuilds.sh 2>&1 | tee ./make_build.output
```

### Local Shell Escape (`lsh`)

There is a special command `lsh` that lets you run host commands from internal interactive shell prompt (REPL) and thorugh automation/stdin shell interface:

```bash
{ echo "lsh"; echo "ls -lah"; whoami; uname -a; date; pwd; } \
  | /usr/bin/time -v -o ./make_build.log \
    ./rbuilds/rbuilds.sh 2>&1 | tee ./make_build.output
```

Use this with care, as it bridges the interactive/automation shell and your host environment (through script eval loop, REPL).

---

## What Gets Packaged Into the EIF

The EIF images produced by `rbuilds.sh` contain:

### 1. Custom Linux Kernel

- Built specifically for Nitro Enclaves.
- Includes:
  - vsock support.
  - Netfilter/NAT support required by proxies.
  - Other options suitable for enclave workloads.

### 2. Init System

- Runs as **PID 1**.
- Responsibility:
  - Mount core filesystems (`/proc`, `/sys`, `/dev`, etc.).
  - Read configuration (`env`, `cmd`) and environment variables.
  - Start and supervise enclave services.
  - Coordinate graceful shutdown.

### 3. Secure Local Channel (SLC)

- VSock-based channel for:
  - Executing commands inside the enclave from the host.
  - Transferring files and directories between host and enclave.
- Designed to be the main control/data plane between host and enclave, beyond attestation.

### 4. Proxies (Forward / Reverse / Transparent)

- Implemented as VSock-aware services with iptables/Netfilter integration.
- Typical roles:
  - Forward proxy for outbound connections (enclave → host → internet).
  - Reverse proxy for inbound requests into enclave services.
  - Transparent proxy using NAT rules so applications do not need proxy awareness.

### 5. Remote Attestation Web Server

- Enclave-resident HTTP service for attestation flows.
- Responsibilities (conceptual):
  - Handling Nitro attestation documents.
  - Interacting with KMS or other key management systems.
  - Providing a REST-like interface for verifiers / clients.
  - Attesting enclave base running image (EIF) in Nitro Enclaves runtime, enclave state and referenced artifacts (e.g., binaries, models).

### 6. File System Monitor

- `inotify`-based monitor inside the enclave.
- Watches file system changes (e.g., new data, updated models).
- Triggers **per-file attestation** so each specific runtime artifact can be attested, not just the initial EIF.

---

## Init Systems

### Current C/clang-based init

**Location:**

- Scripts and helpers in: `rbuilds/enclave.init/`
- Boot scripts such as: `rbuilds/enclave.init/init_revp+tpp.sh`
- Configuration:
  - `rbuilds/rootfs_base/env` – default environment.
  - `rbuilds/rootfs_base/cmd` – default command.

**High-level behavior:**

1. **Boot entrypoint**
   Bootloading:
   - Kernel bootload NSM device driver. Init send magic number as liveness check of enclave successful boot to hypervisor.
   - Kernel boots and runs the initramfs that includes a small C-based init, which then executes `init_revp+tpp.sh` from `cmd` and set environment from `env` and init shell scripts.

2. **Environment setup**
   Init:
   - Loads environment variables from `env`.
   - Reads the primary app command from `cmd`.
   - Configures VSock ports, proxy modes, attestation options, etc.

3. **Service startup**
   The init script orchestrates:
   - Starting Pipeline SLC (VSock control protocol channel).
   - Starting forward/reverse/transparent proxies and configuring netfilter/iptables.
   - Starting NATS bus.
   - Starting the RA web server.
   - Starting the file system monitor.
   - Finally launching the main application (`cmd`) in the configured environment.

4. **Lifecycle management**
   - Propagates signals to managed services.
   - Ensures clean shutdown when the enclave is terminated.

This C/clang-based init is the **current default** selected via `--init-c`.

---

### New Rust `enclave-init`

**Location:**

- [`enclave-init/README.md`](enclave-init/README.md)
- [`enclave-init/src/main.rs`](enclave-init/src/main.rs)

The Rust `enclave-init` is the new init system designed to eventually replace the C/clang-based init. It is written in Rust for:

- Better safety guarantees.
- Clearer error handling and logging.
- More robust service supervision.

**Intended responsibilities:**

- Run as PID 1 inside the enclave.
- Mount required file systems.
- Parse service files with configuration (similar to `env` and `cmd`) and environment variables.
- Manage services and processes in an enclave system.
- Handling of kernel signals for the runing processes.
- Start and supervise:
  - Pipeline SLC.
  - Proxies (forward, reverse, transparent).
  - Attestation web server.
  - File system monitor.
  - The main application process.
- Enforce restart policies / failure handling as needed.
- Provide structured logs for debugging and observability.

**Integration with `rbuilds.sh`:**

- The `make_init` stage will build the Rust binary and package it as `/init` in the rootfs.
- A new CLI flag (e.g., `--init-rust`) can be used to select this implementation once integration is complete.
- Until then, `--init-c` remains the stable path for production builds.

---

## CI / Automation Patterns

Below are examples of how to integrate `rbuilds.sh` into CI pipelines or higher-level orchestration.

### Pattern 1: Simple “build all” job

```bash
mkdir -vp ./eif/
/usr/bin/time -v -o ./eif/make_build.log \
./rbuilds/rbuilds.sh \
  --tty \
  --debug \
  --dockerfile ./pipeline-slc-network-al2023.dockerfile \
  --network \
  --init-c \
  --cmd "make_all" \
  2>&1 3>&1 | tee ./eif/make_build.output
```

Artifacts to keep:

- `./eif/make_build.log`
- `./eif/make_build.output`
- `./eif/*.eif` (EIF images)

### Pattern 2: Stage-by-stage builds

Useful when you want to cache intermediate outputs or debug a specific stage.

```bash
# Kernel only
./rbuilds/rbuilds.sh --tty --debug \
  --dockerfile ./pipeline-slc-network-al2023.dockerfile \
  --network --init-c \
  --cmd "make_kernel" 2>&1 3>&1

# Apps only
./rbuilds/rbuilds.sh --tty --debug \
  --dockerfile ./pipeline-slc-network-al2023.dockerfile \
  --network --init-c \
  --cmd "make_apps" 2>&1 3>&1

# Init only
./rbuilds/rbuilds.sh --tty --debug \
  --dockerfile ./pipeline-slc-network-al2023.dockerfile \
  --network --init-c \
  --cmd "make_init" 2>&1 3>&1

# EIF only (after previous stages are cached)
./rbuilds/rbuilds.sh --tty --debug \
  --dockerfile ./pipeline-slc-network-al2023.dockerfile \
  --network --init-c \
  --cmd "make_eif" 2>&1 3>&1
```

### Pattern 3: Complex flows via automation shell

For multi-step flows where you want a single entrypoint:

```bash
{
  echo "make all";          # full build
  echo "list_enclaves";     # sanity check
  echo "run_eif_image";     # run the enclave
} | /usr/bin/time -v -o ./eif/make_build.log \
    ./rbuilds/rbuilds.sh 2>&1 | tee ./eif/make_build.output
```

---

## Troubleshooting & Tips

### 1. Build fails with Docker/network errors

- Ensure Docker is running and your user has permission to use it.
- If your Dockerfile needs network access (package managers, etc.), ensure host network working properly.
- Check proxy/firewall rules if your build cannot reach external registries.

### 2. Nitro-related failures (cannot run EIF)

- Confirm that:
  - The EC2 instance supports Nitro Enclaves.
  - Nitro Enclaves are enabled in the instance configuration.
  - `nitro-cli` is installed and working (`nitro-cli --help`).
- Check that `/var/run/nitro_enclaves/*` or related resources exist and are accessible.
- Check for sufficient memory and CPUs allocation (by Nitro allocator service) and there are enough hardware resources available on the host.
- Check for enabling Huge Pages (1GiB pages allocation by kernel).
- Check if NUMA nodes are enabled on host. If NUMA involves memory/CPU allocation issues (for Nitro allocator service) and led to resources shortage on running NUMA node - disable NUMA in GRUB bootloader kernel boot parameters for testing and debugging purposes if needed. This will enable unified contiguous resources pool.

### 3. Enclave runs but network doesn’t work

- Remember: enclaves have **no direct network** – everything is via:
  - VSock to the host.
  - Proxies in host and enclave.
- Check:
  - That `make_apps` completed successfully and proxies are present and running.
  - VSock CID and port values (`env` configuration) are correct.
  - iptables rules on host and enclave enabled properly by init scripts.

### 4. Attestation or FS monitor issues

- Ensure those binaries were built (`make_apps`).
- Check that required configuration (keys, KMS integration, endpoints) is in place.
- Look at enclave logs via `run_eif_image_debugmode_cli` and `attach_console_to_enclave`.

### 5. Getting more help

Use the built-in help:

```bash
./rbuilds/rbuilds.sh --info
./rbuilds/rbuilds.sh --man
```

These commands show the exact options and commands supported by your version of `rbuilds.sh`.

---

## License

This project is licensed under the **Apache 2.0 License**. See the [`LICENSE-APACHE`](LICENSE-APACHE) file for the details.

---

