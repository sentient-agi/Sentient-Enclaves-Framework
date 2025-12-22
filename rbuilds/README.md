# Reproducible Builds System (`rbuilds.sh`)

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Prerequisites](#prerequisites)
- [Quick Start Guide](#quick-start-guide)
- [Installation](#installation)
- [Architecture](#architecture)
- [CLI Reference](#cli-reference)
- [Build Stages](#build-stages)
- [Configuration Options](#configuration-options)
- [Networking Support](#networking-support)
- [Init Systems](#init-systems)
- [Usage Guide](#usage-guide)
- [Advanced Usage](#advanced-usage)
- [Components Reference](#components-reference)
- [Dockerfile Reference](#dockerfile-reference)
- [Troubleshooting](#troubleshooting)
- [Examples](#examples)

---

## Overview

`rbuilds.sh` is a comprehensive shell script for building reproducible AWS Nitro Enclave images (EIF). It provides an end-to-end solution for creating custom enclave environments with:

- **Custom Linux kernel builds** with networking stack support
- **Init system compilation** (C-based and Go-based implementations)
- **Rust application builds** for the Sentient Secure Enclaves (SSE) Framework
- **Transparent VSock proxies** for enclave networking
- **Remote attestation** components
- **File system monitoring** for per-file attestation

The script ensures byte-level reproducibility of enclave images, enabling verifiable builds for confidential computing applications.

### Key Capabilities

| Capability | Description |
|------------|-------------|
| **Reproducible Builds** | Deterministic EIF image generation with consistent PCR hashes |
| **Custom Kernel** | Linux kernel with networking modules for enclave connectivity |
| **Multiple Init Systems** | Support for C, Go, and Rust-based init systems |
| **Networking Stack** | Forward and reverse proxy support for TCP/UDP over VSock |
| **Remote Attestation** | Built-in RA web server with VRF proofs and PCR verification |
| **File System Monitoring** | Real-time inotify-based file hash tracking |
| **Pipeline SLC** | Secure local channel for host-enclave communication |

---

## Features

### Core Features

- ✅ **Automated Build Pipeline** - Single command to build entire enclave stack
- ✅ **Interactive Shell Mode** - Step-by-step guided builds
- ✅ **CLI Automation** - Scriptable command interface
- ✅ **Docker-based Isolation** - All builds in isolated containers
- ✅ **Modular Architecture** - Build stages can run independently
- ✅ **Configurable Networking** - Forward, reverse, or bidirectional proxies
- ✅ **Multiple Init Systems** - Choose between C, Go, or Rust init
- ✅ **Comprehensive Logging** - Build timing and output capture

### Framework Components Built

| Component | Description |
|-----------|-------------|
| `pipeline` | Secure local channel protocol for host-enclave communication |
| `ra-web-srv` | Remote attestation HTTPS web server |
| `fs-monitor` | File system change tracking and hashing |
| `pf-proxy` | VSock port forwarding proxies (forward/reverse/transparent) |
| `nats-server` | Embedded message queue for enclave service bus |
| `eif_build` | EIF image assembly tool |
| `eif_extract` | EIF image extraction/inspection tool |

---

## Prerequisites

### System Requirements

- **Platform**: AWS EC2 instance with Nitro Enclaves support
- **OS**: Amazon Linux 2023 (recommended) or Amazon Linux 2
- **Architecture**: x86_64
- **vCPUs**: Minimum 4 (excluding certain instance types)
- **Memory**: Sufficient for enclave allocation (default: 838656 MiB configurable)

### Instance Requirements

Nitro-based Intel or AMD instances with at least 4 vCPUs. **Excluded instance types**:
- c7i.24xlarge, c7i.48xlarge
- G4ad
- m7i.24xlarge, m7i.48xlarge, M7i-Flex
- r7i.24xlarge, r7i.48xlarge, R7iz
- T3, T3a
- Trn1, Trn1n
- U-*, VT1

### Software Dependencies

The script automatically installs required dependencies:
- `docker` - Container runtime
- `aws-nitro-enclaves-cli` - Nitro Enclaves management
- `sed`, `grep`, `pcregrep` - Text processing utilities

---

## Quick Start Guide

### 1. Enable Nitro Enclaves

```bash
cd rbuilds/
sudo ./rbuilds.sh --cmd "make_nitro"
# System will prompt for reboot
```

### 2. Build Everything (Automated)

```bash
# Full automated build with networking support
mkdir -vp ./eif/
/usr/bin/time -v -o ./eif/make_build.log \
  ./rbuilds.sh --tty --debug \
    --dockerfile ./pipeline-slc-network-al2023.dockerfile \
    --network --init-c \
    --cmd "make_all" 2>&1 3>&1 | tee ./eif/make_build.output
```

### 3. Step-by-Step Build

```bash
# Build kernel
sudo ./rbuilds.sh --tty --debug \
  --dockerfile ./pipeline-slc-network-al2023.dockerfile \
  --network --init-c \
  --cmd "make_kernel" 2>&1 3>&1

# Build system components
sudo ./rbuilds.sh --tty --debug \
  --dockerfile ./pipeline-slc-network-al2023.dockerfile \
  --network --init-c \
  --cmd "make_apps" 2>&1 3>&1

# Build init system
sudo ./rbuilds.sh --tty --debug \
  --dockerfile ./pipeline-slc-network-al2023.dockerfile \
  --network --init-c \
  --cmd "make_init" 2>&1 3>&1

# Build EIF image
sudo ./rbuilds.sh --tty --debug \
  --dockerfile ./pipeline-slc-network-al2023.dockerfile \
  --network --init-c \
  --cmd "make_eif" 2>&1 3>&1
```

### 4. Run Enclave

```bash
# Run in debug mode with console output
sudo ./rbuilds.sh --tty --debug --network --init-c \
  --cmd "run_eif_image_debugmode_cli" 2>&1 3>&1
```

---

## Installation

### Initial Setup

1. **Clone the Repository**
```bash
git clone https://github.com/sentient-agi/sentient-enclaves-framework.git
cd sentient-enclaves-framework/rbuilds/
```

2. **Configure Nitro Enclaves**
```bash
sudo ./rbuilds.sh --cmd "make_nitro"
```

This command:
- Installs Docker and Nitro Enclaves CLI
- Adds current user to `docker` and `ne` groups
- Configures enclave resource allocation in `/etc/nitro_enclaves/allocator.yaml`
- Enables required system services
- Prompts for system reboot

### Resource Configuration

Default enclave allocation (configurable via CLI):

| Resource | Default Value | CLI Flag |
|----------|---------------|----------|
| Memory | 262144 MiB | `--memory <MiB>` |
| CPUs | 16 | `--cpus <count>` |
| VSock CID | 127 | `--cid <cid>` |

---

## Architecture

### Build Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                     rbuilds.sh Build Pipeline                   │
└─────────────────────────────┬───────────────────────────────────┘
                              │
         ┌────────────────────┼────────────────────┐
         ▼                    ▼                    ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│  make_kernel    │  │   make_apps     │  │   make_init     │
│                 │  │                 │  │                 │
│ Custom Linux    │  │ SSE Framework   │  │ Init System     │
│ Kernel + NSM    │  │ Components      │  │ (C/Go/Rust)     │
└────────┬────────┘  └────────┬────────┘  └────────┬────────┘
         │                    │                    │
         └────────────────────┼────────────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │    make_eif     │
                    │                 │
                    │ Assemble EIF    │
                    │ Image           │
                    └────────┬────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │  make_enclave   │
                    │                 │
                    │ Run & Manage    │
                    │ Enclave         │
                    └─────────────────┘
```

### Directory Structure

```
rbuilds/
├── rbuilds.sh                              # Main build script
├── patterns                                # CPIO archive exclusion patterns
├── kernel_config/                          # Kernel configurations
│   ├── artifacts_kmods/.config             # With kernel modules
│   ├── artifacts_static/.config            # Static modules
│   └── kernel_wo_net/.config               # Without networking
├── init_apps/                              # Init system sources
│   ├── init/                               # C-based init
│   └── init_go/                            # Go-based init
├── enclave.init/                           # Enclave runtime scripts
│   ├── init.sh                             # Main init script
│   ├── init_revp.sh                        # Reverse proxy init
│   ├── init_tpp.sh                         # Transparent proxy init
│   ├── init_revp+tpp.sh                    # Combined proxy init
│   ├── init_wo_net.sh                      # No networking init
│   ├── pf-*.sh                             # Proxy startup scripts
│   └── .config/                            # Configuration files
├── rootfs_base/                            # Base filesystem structure
│   ├── cmd                                 # Default command
│   └── env                                 # Environment variables
└── *.dockerfile                            # Build dockerfiles
```

### Component Flow

```
┌───────────────────────────────────────────────────────────────────┐
│                        Host EC2 Instance                          │
│                                                                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐             │
│  │ pf-rev-host  │  │ pf-tp-host   │  │   pf-host    │             │
│  │ (Reverse     │  │ (Transparent │  │   (DNS/UDP   │             │
│  │  Proxy)      │  │  Proxy)      │  │    Forward)  │             │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘             │
│         │                 │                 │                     │
│         └─────────────────┼─────────────────┘                     │
│                           │ VSock                                 │
└───────────────────────────┼───────────────────────────────────────┘
                            │
┌───────────────────────────┼───────────────────────────────────────┐
│                           ▼                                       │
│                     AWS Nitro Enclave                             │
│                                                                   │
│  ┌────────────────────────────────────────────────────────┐       │
│  │                     init (PID 1)                       │       │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐       │       │
│  │  │pipeline │ │ra-web-  │ │fs-      │ │nats-    │       │       │
│  │  │(SLC)    │ │srv      │ │monitor  │ │server   │       │       │
│  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘       │       │
│  │                                                        │       │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐                   │       │
│  │  │vs2ip    │ │ip2vs-tp │ │socat    │   (Proxy Layer)   │       │
│  │  │(Rev Prx)│ │(Fwd Prx)│ │(DNS)    │                   │       │
│  │  └─────────┘ └─────────┘ └─────────┘                   │       │
│  └────────────────────────────────────────────────────────┘       │
│                                                                   │
└───────────────────────────────────────────────────────────────────┘
```

---

## CLI Reference

### Global Options

| Option | Short | Environment | Default | Description |
|--------|-------|-------------|---------|-------------|
| `--help` | `-?`, `-h` | - | - | Print CLI help |
| `--help-ext` | `-??`, `-hh` | - | - | Print extended help |
| `--man` | - | - | - | Print man pages |
| `--info` | - | - | - | Print exhaustive documentation |
| `--debug` | `-v`, `--verbose` | - | Off | Enable verbose output |
| `--question` | `-q` | - | Off | Prompt before each command |
| `--local-shell` | `--lsh`, `-lsh` | - | Off | Enable local shell command execution |
| `--tty` | `--terminal` | - | Off | Allocate TTY for build output |

### Build Configuration

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--dockerfile <path>` | `-d` | `pipeline-al2023.dockerfile` | Dockerfile for rootfs image |
| `--kernel <version>` | `-k` | `6.14.5` | Linux kernel version |
| `--user <name>` | `-u` | `sentient_build` | Kernel build username |
| `--host <name>` | `-h` | `sentient_builder` | Kernel build hostname |

### Enclave Runtime

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--memory <MiB>` | `-m` | `262144` | Enclave memory allocation |
| `--cpus <count>` | - | `16` | Enclave CPU allocation |
| `--cid <cid>` | - | `127` | VSock CID for enclave |

### Networking Flags

| Option | Short | Description |
|--------|-------|-------------|
| `--network` | `-n` | Enable both forward and reverse proxies |
| `--forward-network` | `-fn` | Enable forward proxy only |
| `--reverse-network` | `-rn` | Enable reverse proxy only |

### Init System Selection

| Option | Description |
|--------|-------------|
| `--init-c`, `--clang` | Use C-based init system (default) |
| `--init-go`, `--golang` | Use Go-based init system |
| `--init-rs`, `--rust` | Use Rust-based init system |

### Command Execution

| Option | Short | Description |
|--------|-------|-------------|
| `--cmd <command>` | `-c` | Execute specified command (repeatable) |

---

## Build Stages

### Stage 1: Kernel Build (`make_kernel`)

Builds a custom Linux kernel with NSM driver support.

**Components:**
- Linux kernel (configurable version)
- NSM (Nitro Security Module) driver
- Network stack modules (when networking enabled)

**Commands:**
```bash
docker_kcontainer_clear    # Clean previous container
docker_kimage_clear        # Clean previous image
docker_kimage_build        # Build kernel build environment
docker_prepare_kbuildenv   # Setup build environment
docker_kernel_build        # Compile kernel
```

**Output:**
- `kernel_blobs/bzImage` - Compressed kernel image
- `kernel_blobs/.config` - Kernel configuration
- `kernel_blobs/nsm.ko` - NSM kernel module
- `kernel_blobs/kernel_modules/` - Kernel modules

### Stage 2: Applications Build (`make_apps`)

Builds SSE Framework Rust applications.

**Components:**
- `pipeline` - Secure local channel
- `ra-web-srv` - Remote attestation web server
- `fs-monitor` - File system monitor
- `ip-to-vsock`, `vsock-to-ip` - Standard proxies
- `ip-to-vsock-transparent`, `vsock-to-ip-transparent` - Transparent proxies
- `transparent-port-to-vsock` - Port-based transparent proxy
- `eif_build`, `eif_extract` - EIF image tools

**Commands:**
```bash
docker_apps_rs_container_clear    # Clean previous container
docker_apps_rs_image_clear        # Clean previous image
docker_apps_rs_image_build        # Build Rust build environment
docker_prepare_apps_rs_buildenv   # Clone repositories
docker_apps_rs_build              # Compile applications
```

### Stage 3: Init System Build (`make_init`)

Builds init system for enclave boot.

**Options:**
- **C-based init** (`--init-c`): Minimal C init binary
- **Go-based init** (`--init-go`): Go init with enhanced features
- **Rust-based init** (`--init-rs`): New Rust init system (in beta testing stage)

**Commands:**
```bash
docker_init_apps_container_clear    # Clean previous container
docker_init_apps_image_clear        # Clean previous image
docker_init_apps_image_build        # Build init build environment
docker_prepare_init_apps_buildenv   # Setup sources
docker_init_apps_build              # Compile init + NATS server
```

**Additional build for NATS server:**
- Compiles `nats-server` for enclave service bus

### Stage 4: EIF Image Build (`make_eif`)

Assembles final Enclave Image Format (EIF) file.

**Process:**
1. Create init CPIO archive
2. Create rootfs base CPIO archive
3. Export Docker container as rootfs CPIO
4. Include kernel modules CPIO
5. Assemble ramdisk from all CPIOs
6. Build EIF with kernel + ramdisk

**Commands:**
```bash
docker_eif_build_container_clear     # Clean app container
docker_eif_build_image_clear         # Clean images
docker_container_apps_image_build    # Build app image
init_and_rootfs_base_images_build    # Create init/base CPIOs
docker_to_rootfs_fs_image_build      # Export rootfs CPIO
ramdisk_image_build                  # Combine all CPIOs
eif_build_with_initc                 # Build EIF (C init)
eif_build_with_initgo                # Build EIF (Go init)
```

**Output:**
- `eif/init_c_eif/app-builder-secure-enclaves-framework.eif`
- `eif/init_go_eif/app-builder-secure-enclaves-framework.eif`
- PCR hashes for attestation verification

---

## Configuration Options

### Kernel Configuration

Three kernel configurations available in `kernel_config/`:

| Config | Path | Description |
|--------|------|-------------|
| With Modules | `artifacts_kmods/.config` | Network modules as loadable |
| Static | `artifacts_static/.config` | Network compiled statically |
| No Network | `kernel_wo_net/.config` | Minimal, no networking |

Selection is automatic based on `--network` flag.

### Environment Variables

Default enclave environment (`rootfs_base/env`):
```
SHELL='/usr/bin/env bash'
HOME=/apps
PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
RUST_LOG=debug
RUST_BACKTRACE=full
CERT_DIR=/apps/certs/
```

### Init Configuration

Init startup command (`rootfs_base/cmd`):
```
/bin/bash
-c
--
echo -e "init started"; echo -e "executing init.sh"; cd /apps/; ./init.sh 2>&1 & disown; echo -e "init.sh executed"; echo -e "init launched"; tail -f /dev/null
```

---

## Networking Support

### Network Modes

| Mode | Flag | Init Script | Description |
|------|------|-------------|-------------|
| None | (default) | `init_wo_net.sh` | No network access |
| Forward | `--forward-network` | `init_tpp.sh` | Outbound connections |
| Reverse | `--reverse-network` | `init_revp.sh` | Inbound connections |
| Both | `--network` | `init_revp+tpp.sh` | Full bidirectional |

### Forward Proxy Flow

```
Enclave App → iptables DNAT → ip2vs-tp → VSock → vs2ip-tp → Internet
```

**Guest side (`pf-tp-guest.sh`):**
- Configures localhost loopback
- Sets iptables NAT rules
- Starts `ip2vs-tp` transparent proxy

**Host side (`pf-tp-host.sh`):**
- Starts `vs2ip-tp` transparent proxy
- Routes VSock traffic to destination

### Reverse Proxy Flow

```
Internet → Host Port → ip2vs → VSock → vs2ip → Enclave Service
```

**Guest side (`pf-rev-guest.sh`):**
- Configures localhost
- Starts `vs2ip` proxies on multiple VSock ports
- Maps VSock ports to local service ports

**Host side (`pf-rev-host.sh`):**
- Sets iptables rules for port redirection
- Starts `ip2vs` and `tpp2vs` proxies
- Routes external traffic to enclave

### DNS Forwarding

DNS queries from enclave forwarded via UDP over VSock:

**Guest (`pf-guest.sh`):**
```bash
socat UDP-LISTEN:53,reuseaddr,fork VSOCK-CONNECT:5:8053
```

**Host (`pf-host.sh`):**
```bash
socat VSOCK-LISTEN:8053,reuseaddr,fork UDP:$(nameserver):53
```

### Port Mappings

| External Port | VSock Port | Service |
|---------------|------------|---------|
| 80 | 8080 | HTTP |
| 443 | 8443 | HTTPS |
| 9000-10000 | 9000-10000 | Custom range |
| 10000-11000 | 11001 | Transparent (port preserved) |

---

## Init Systems

### C-based Init (`init.c`)

Minimal init system in C. Features:
- NSM driver loading
- Console setup
- Basic process spawning
- Filesystem mounting

**Build:**
```bash
gcc -Wall -Wextra -Werror -O3 -flto -static -static-libgcc -o init init.c
```

### Go-based Init (`init.go`)

Enhanced init system in Go. Features:
- All C init features
- Environment variable handling
- Enhanced logging
- Signal handling

**Build:**
```bash
CGO_ENABLED=0 go build -a -trimpath -ldflags "-s -w -extldflags=-static" -o init init.go
```

### Rust-based Init (New)

Advanced init system with service management. Features:
- Service supervision with restart policies
- Dependency ordering (Before/After/Requires)
- Process management (list, start, stop, signal)
- Dual control protocol (Unix socket + VSock)
- Persistent logging with rotation
- System status monitoring

**Configuration:** `/etc/init.yaml`
**Control CLI:** `initctl`

---

## Usage Guide

### Interactive Mode

Start interactive shell:
```bash
./rbuilds.sh 2>&1
```

Commands available in interactive mode:
```bash
help              # Print help
help_ext          # Extended help
make kernel       # Build kernel
make apps         # Build applications
make init         # Build init system
make eif          # Build EIF image
make all          # Full build
make enclave      # Enclave management
make clear        # Clean all Docker artifacts
q                 # Toggle question prompts
lsh               # Toggle local shell access
network           # Toggle full networking
forward_network   # Toggle forward proxy
reverse_network   # Toggle reverse proxy
exit              # Exit shell
```

### Automation Mode

Pipe commands for automation:
```bash
# Single command
{ echo "make_kernel"; } | ./rbuilds.sh 2>&1

# Multiple commands
{
  echo "make kernel"
  echo "make apps"
  echo "make init"
  echo "make eif"
} | ./rbuilds.sh 2>&1

# With timing and logging
{ echo "make all"; } | /usr/bin/time -v -o ./eif/make_build.log \
  ./rbuilds.sh 2>&1 | tee ./eif/make_build.output
```

### Enclave Management

```bash
# Run with debug console attached
./rbuilds.sh --cmd "run_eif_image_debugmode_cli"

# Run without console (background)
./rbuilds.sh --cmd "run_eif_image_debugmode"

# Run in production mode
./rbuilds.sh --cmd "run_eif_image"

# Attach console to running enclave
./rbuilds.sh --cmd "attach_console_to_enclave"

# List running enclaves
./rbuilds.sh --cmd "list_enclaves"

# Terminate enclave
./rbuilds.sh --cmd "drop_enclave"

# Terminate all enclaves
./rbuilds.sh --cmd "drop_enclaves_all"
```

---

## Advanced Usage

### Custom Dockerfile

Use custom application dockerfile:
```bash
./rbuilds.sh --tty --debug \
  --dockerfile ./my-app.dockerfile \
  --network --init-c \
  --cmd "make_eif" 2>&1 3>&1
```

### Kernel Version Override

Build with specific kernel:
```bash
./rbuilds.sh --kernel 6.12.0 --cmd "make_kernel"
```

### Multiple Commands

Execute multiple build stages:
```bash
./rbuilds.sh \
  --cmd "make_kernel" \
  --cmd "make_apps" \
  --cmd "make_init" \
  --cmd "make_eif"
```

### Resource Configuration

Custom enclave resources:
```bash
./rbuilds.sh \
  --memory 16384 \
  --cpus 8 \
  --cid 16 \
  --cmd "run_eif_image_debugmode_cli"
```

### Build Without Networking

Minimal build without network stack:
```bash
./rbuilds.sh --tty --debug \
  --dockerfile ./pipeline-al2023.dockerfile \
  --init-c \
  --cmd "make_all" 2>&1 3>&1
```

### Local Shell Access

Enable local shell commands:
```bash
./rbuilds.sh --lsh
# Then in interactive mode:
lsh  # Enable local shell
ls -lah  # Execute local command
```

---

## Components Reference

### Pipeline (SLC)

Secure local channel for host-enclave communication.

**Enclave (listen mode):**
```bash
./pipeline listen --port 53000
```

**Host commands:**
```bash
# Execute command in enclave
./pipeline run --cid 127 --port 53000 --command "hostname"

# Send file to enclave
./pipeline send-file --cid 127 --port 53000 \
  --localpath ./data.txt --remotepath /apps/data.txt

# Receive file from enclave
./pipeline recv-file --cid 127 --port 53000 \
  --remotepath /apps/output.txt --localpath ./output.txt
```

### Remote Attestation Web Server

HTTPS server for enclave attestation.

**Endpoints:**
| Endpoint | Method | Description |
|----------|--------|-------------|
| `/generate` | POST | Generate attestation for path |
| `/ready/` | GET | Check processing status |
| `/hash/` | GET | Get file hash |
| `/proof/` | GET | Get VRF proof |
| `/doc/` | GET | Get attestation document |
| `/pcrs/` | GET | Get PCR registers |
| `/pubkeys/` | GET | Get public keys |
| `/verify_hash/` | POST | Verify file hash |
| `/verify_proof/` | POST | Verify VRF proof |
| `/verify_doc/` | POST | Verify document signature |
| `/verify_cert_bundle/` | POST | Verify certificate chain |
| `/verify_pcrs/` | POST | Compare PCR values |

### File System Monitor

Real-time file change tracking.

**Configuration:**
- Watch directory: `/apps/`
- Ignore file: `.fsignore`
- NATS integration for hash storage

**Operation:**
- Monitors inotify events
- Computes SHA3-512 hashes
- Stores hashes in NATS KV bucket
- Triggers attestation document generation

### NATS Server

Embedded message queue for service bus.

**Configuration:** `.config/nats.config`
**Ports:**
- 4222: NATS protocol
- 4242: HTTP monitoring

**KV Buckets:**
- `fs_hashes`: File hash storage
- `fs_att_docs`: Attestation documents

---

## Dockerfile Reference

### Available Dockerfiles

| File | Description |
|------|-------------|
| `rust-build-toolkit-al2023.dockerfile` | Rust build environment |
| `golang-clang-build-toolkit.dockerfile` | Go/C build environment |
| `eif-builder-al2023.dockerfile` | EIF assembly environment |
| `pipeline-al2023.dockerfile` | Basic pipeline image |
| `pipeline-slc-network-al2023.dockerfile` | Network-enabled image |

### Creating Custom Dockerfile

```dockerfile
FROM public.ecr.aws/amazonlinux/amazonlinux:2023 as enclave_app

ENV SHELL="/usr/bin/env bash"
ENV RUST_LOG="debug"
ENV RUST_BACKTRACE="full"

WORKDIR /apps

# Install dependencies
RUN dnf upgrade -y
RUN dnf install -y your-dependencies

# Copy your application
COPY --link your-app /apps/

# Default command (not used, init.sh takes over)
CMD tail -f /dev/null
```

---

## Troubleshooting

### Common Issues

**Docker permission denied:**
```bash
sudo usermod -aG docker $USER
# Re-login or: newgrp docker
```

**Enclave fails to start:**
```bash
# Check allocator configuration
cat /etc/nitro_enclaves/allocator.yaml

# Verify service status
sudo systemctl status nitro-enclaves-allocator.service

# Check available resources
nitro-cli describe-enclaves --metadata
```

**Build fails with missing tools:**
```bash
# Install required packages
sudo dnf install -y sed grep pcre-tools
```

**Network connectivity issues in enclave:**
```bash
# Verify proxies are running on host
pidof vs2ip-tp
pidof ip2vs-tp

# Check iptables rules
sudo iptables-save | grep -i nat
```

**TTY output issues:**
```bash
# Use 3>&1 for TTY mode
./rbuilds.sh --tty --cmd "make_all" 2>&1 3>&1
```

### Debug Mode

Enable verbose logging:
```bash
./rbuilds.sh --debug --tty --cmd "make_kernel" 2>&1 3>&1
```

### Clean Build

Remove all build artifacts:
```bash
./rbuilds.sh --cmd "make_clear"
# Or clean specific stage:
./rbuilds.sh --cmd "docker_kcontainer_clear"
```

---

## Examples

### Example 1: Complete Build for AI Inference Server

```bash
#!/bin/bash
# build-inference-server.sh

cd rbuilds/

# Build with networking for model download capability
./rbuilds.sh --tty --debug \
  --dockerfile ../reference_apps/inference_server/inference_server.dockerfile \
  --network --init-c \
  --memory 65536 \
  --cpus 16 \
  --cmd "make_all" 2>&1 3>&1

# Deploy enclave
./rbuilds.sh --network --init-c \
  --memory 65536 --cpus 16 --cid 16 \
  --cmd "run_eif_image_debugmode_cli" 2>&1 3>&1
```

### Example 2: Minimal Secure Computation

```bash
#!/bin/bash
# build-minimal.sh

cd rbuilds/

# Build without networking for isolated computation
./rbuilds.sh --tty --debug \
  --dockerfile ./pipeline-al2023.dockerfile \
  --init-c \
  --cmd "make_all" 2>&1 3>&1

# Run in production mode
./rbuilds.sh --init-c --cmd "run_eif_image" 2>&1 3>&1
```

### Example 3: CI/CD Pipeline Integration

```yaml
# .github/workflows/build-enclave.yml
name: Build Enclave Image

on:
  push:
    branches: [main]

jobs:
  build:
    runs-on: [self-hosted, nitro-enabled]
    steps:
      - uses: actions/checkout@v4

      - name: Build EIF
        run: |
          cd rbuilds/
          ./rbuilds.sh --tty \
            --dockerfile ./pipeline-slc-network-al2023.dockerfile \
            --network --init-c \
            --cmd "make_all" 2>&1 3>&1

      - name: Upload Artifacts
        uses: actions/upload-artifact@v4
        with:
          name: enclave-eif
          path: rbuilds/eif/
```

### Example 4: Attestation Verification

```bash
#!/bin/bash
# verify-enclave.sh

# Get PCRs from built EIF
EIF_PCRS=$(nitro-cli describe-eif \
  --eif-path ./eif/init_c_eif/app-builder-secure-enclaves-framework.eif \
  | jq -r '.Measurements')

# Get runtime PCRs via attestation
RUNTIME_PCRS=$(curl -k https://127.0.0.1:8443/pcrs/)

# Compare (simplified)
echo "EIF PCRs: $EIF_PCRS"
echo "Runtime PCRs: $RUNTIME_PCRS"

# Verify attestation document
curl -k -X POST https://127.0.0.1:8443/verify_cert_bundle/ \
  -H "Content-Type: application/json" \
  -d '{"cose_doc_bytes": "'$(curl -k -s https://127.0.0.1:8443/doc/?path=/apps/&view=hex)'"}'
```

---

## Version Information

**Framework Version:** `0.9.0`
**Default Kernel Version:** `6.14.5`
**Supported Init Systems:** C, Go, Rust (New)

---

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE-APACHE](LICENSE-APACHE) file for details.

---

## References

- [AWS Nitro Enclaves Documentation](https://docs.aws.amazon.com/enclaves/latest/user/)
- [Sentient Enclaves Framework](https://github.com/sentient-agi/sentient-enclaves-framework)
- [Reproducible Builds System](../rbuilds/README.md) (THIS README)
- [Pipeline SLC Documentation](../pipeline/README.md)
- [PF-Proxy Documentation](../pf-proxy/README.md)
- [RA Web Server Documentation](../ra-web-srv/README.md)
- [FS Monitor Documentation](../fs-monitor/README.md)
- [Enclave Init System](../enclave-init/README.md)
- [Enclave Engine](../enclave-engine/README.md)

---

## Support

For issues, questions, or contributions:
- **GitHub Issues**: [Project Issues](https://github.com/sentient-agi/sentient-enclaves-framework/issues)
- **Documentation**: [Project Docs](../docs/)

---
