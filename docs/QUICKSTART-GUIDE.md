# Sentient Enclaves Framework - Quick Start Guide

## 🚀 From Zero to Enclave in Minutes

This guide will help you get started with the Sentient Enclaves Framework, whether you're new to secure enclaves or an experienced developer looking to build confidential applications.

---

## What is the Sentient Enclaves Framework?

The Sentient Enclaves Framework is a comprehensive toolkit for building, deploying, and running secure applications inside AWS Nitro Enclaves. Think of it as a complete development platform that handles all the complexity of:

- **Building reproducible enclave images** - Same inputs always produce the same output
- **Secure communication** - Encrypted channels between your application and the outside world
- **Remote attestation** - Cryptographic proof that your code is running unmodified
- **Service management** - A lightweight init system designed for enclaves

---

## Why Use This Framework?

### For Beginners

| Challenge | Our Solution |
|-----------|--------------|
| "Enclaves are complex" | One-command build system handles everything |
| "I don't know where to start" | Guided setup with sensible defaults |
| "Documentation is scattered" | Everything in one place with examples |

### For Advanced Users

| Feature | Benefit |
|---------|---------|
| **Reproducible builds** | Deterministic PCR values for audit and verification |
| **Modular architecture** | Swap components, customize kernel, extend functionality |
| **Production-ready tools** | Battle-tested remote attestation, file monitoring, networking |
| **Automation-friendly** | CI/CD integration via command-line interface |

---

## Prerequisites

### Hardware Requirements

- **EC2 Instance**: Nitro Enclave-enabled instance type
  - Recommended: `c5.xlarge`, `c5.2xlarge`, `m5.xlarge`, `m5.2xlarge` or larger
  - Minimum: 4 vCPUs, 8GB RAM (for basic builds)
  - Production: 16+ vCPUs, 32GB+ RAM (for faster builds)

### Software Requirements

- **Operating System**: Amazon Linux 2023 (recommended) or Amazon Linux 2
- **Docker**: For isolated build environments
- **Git**: For cloning the repository

---

## Part 1: Quick Start (For Beginners)

### Step 1: Launch an EC2 Instance

1. Log into AWS Console
2. Launch an EC2 instance with:
   - AMI: Amazon Linux 2023
   - Instance type: `c5.2xlarge` (or any Nitro Enclave-enabled type)
   - Enable "Nitro Enclaves" in advanced settings

### Step 2: Clone and Setup

```bash
# Connect to your instance
ssh -i your-key.pem ec2-user@your-instance-ip

# Clone the repository
git clone https://github.com/sentient-agi/sentient-enclaves-framework.git
cd sentient-enclaves-framework/rbuilds

# Install Nitro Enclaves (automatic setup)
./rbuilds.sh --cmd "make_nitro"

# Reboot when prompted (required for Nitro allocator)
```

### Step 3: Build Everything (One Command)

After rebooting, reconnect and run:

```bash
cd sentient-enclaves-framework/rbuilds

# Create output directory and build everything
mkdir -vp ./eif/
./rbuilds.sh --tty --network --init-c --cmd "make_all" 2>&1 3>&1
```

☕ **This takes 20-30 minutes.** The build system will:
1. Build a custom Linux kernel with NSM support
2. Compile all Rust applications
3. Build the init system
4. Create the enclave image (EIF)
5. Launch the enclave with debug console

### Step 4: Verify It's Working

When the enclave boots, you'll see debug output. From another terminal:

```bash
# List running enclaves
./rbuilds.sh --cmd "list_enclaves"

# You should see your enclave listed with status "RUNNING"
```

### Step 5: Interact with Your Enclave

```bash
# From the host, run a command inside the enclave
./sentient-enclaves-framework/pipeline run -- ls -la /apps/

# Send a file to the enclave
./sentient-enclaves-framework/pipeline send-file ./test.txt /apps/test.txt

# Retrieve a file from the enclave
./sentient-enclaves-framework/pipeline recv-file /apps/test.txt ./retrieved.txt
```

🎉 **Congratulations!** You have a working enclave!

---

## Part 2: Understanding What You Built

### What's Inside Your Enclave?

```
/apps/
├── pipeline          # Secure communication server (VSOCK)
├── ra-web-srv        # Remote attestation HTTPS API
├── fs-monitor        # File system integrity monitor
├── nats-server       # Message bus for internal services
├── .config/          # Configuration files
├── certs/            # TLS certificates
└── .logs/            # Application logs
```

### Key Services

| Service | Purpose | Port |
|---------|---------|------|
| `pipeline` | Host ↔ Enclave communication | VSOCK 53000 |
| `ra-web-srv` | Remote attestation API | HTTPS 8443 |
| `nats-server` | Internal message bus | 4222 |
| `fs-monitor` | File integrity monitoring | - |

---

## Part 3: Developer Quick Reference

### Build Commands

```bash
# Full build (everything)
./rbuilds.sh --tty --network --init-c --cmd "make_all" 2>&1 3>&1

# Individual stages
./rbuilds.sh --cmd "make_kernel"    # Build custom kernel
./rbuilds.sh --cmd "make_apps"      # Build Rust applications
./rbuilds.sh --cmd "make_init"      # Build init system
./rbuilds.sh --cmd "make_eif"       # Build enclave image

# With networking support
./rbuilds.sh --network --cmd "make_all" 2>&1 3>&1

# Without networking (smaller, more secure)
./rbuilds.sh --init-c --cmd "make_all" 2>&1 3>&1
```

### Enclave Management

```bash
# Run enclave (debug mode with console)
./rbuilds.sh --network --cmd "run_eif_image_debugmode_cli" 2>&1 3>&1

# Run enclave (production mode)
./rbuilds.sh --network --cmd "run_eif_image" 2>&1 3>&1

# Attach to running enclave's console
./rbuilds.sh --cmd "attach_console_to_enclave"

# List all running enclaves
./rbuilds.sh --cmd "list_enclaves"

# Stop specific enclave
./rbuilds.sh --cmd "drop_enclave"

# Stop all enclaves
./rbuilds.sh --cmd "drop_enclaves_all"

# Cleanup build artifacts
./rbuilds.sh --cmd "make_clear"
```

### Pipeline Commands (Host ↔ Enclave)

```bash
# Execute command inside enclave
./pipeline run -- /path/to/command --args

# Execute without waiting for output
./pipeline run --no-wait -- /path/to/command

# Send file to enclave
./pipeline send-file local_file.txt /enclave/path/file.txt

# Receive file from enclave
./pipeline recv-file /enclave/path/file.txt local_file.txt
```

### Remote Attestation API

```bash
# Generate attestation for files
curl -k -X POST https://127.0.0.1:8443/generate \
  -H "Content-Type: application/json" \
  -d '{"path": "/apps/"}'

# Get file hash
curl -k "https://127.0.0.1:8443/hash/?path=/apps/pipeline"

# Get attestation document
curl -k "https://127.0.0.1:8443/doc/?path=/apps/pipeline&view=json_hex"

# Get PCR values
curl -k "https://127.0.0.1:8443/pcrs/"

# Verify hash
curl -k -X POST https://127.0.0.1:8443/verify_hash/ \
  -H "Content-Type: application/json" \
  -d '{"file_path": "/apps/pipeline", "sha3_hash": "..."}'
```

---

## Part 4: Advanced Configuration

### CLI Options Reference

```bash
./rbuilds.sh [OPTIONS] --cmd "COMMAND"

# Display Options
--tty                    # Allocate TTY for interactive output
--debug                  # Enable verbose logging
-q, --question           # Ask before each step

# Build Options
--network                # Enable networking (forward + reverse proxy)
--rev-net                # Enable reverse proxy only
--fw-net                 # Enable forward proxy only
--init-c                 # Use C init system (default)
--init-go                # Use Go init system
--init-rs                # Use Rust init system
--dockerfile FILE        # Custom dockerfile for rootfs

# Kernel Options
--kernel VERSION         # Kernel version (default: 6.14.5)
--user NAME              # Build user (default: sentient_build)
--host NAME              # Build host (default: sentient_builder)

# Enclave Options
--memory SIZE            # Memory in MiB (default: 262144)
--cpus COUNT             # CPU count (default: 16)
--cid VALUE              # VSOCK CID (default: 127)
```

### Custom Dockerfile

Create your own enclave environment:

```dockerfile
# my-app.dockerfile
FROM amazonlinux:2023

# Install your dependencies
RUN dnf install -y python3 nodejs

# Copy your application
COPY ./my-app /apps/my-app

# Set permissions
RUN chmod +x /apps/my-app/*
```

Build with your dockerfile:

```bash
./rbuilds.sh --tty --network --init-c \
  --dockerfile ./my-app.dockerfile \
  --cmd "make_all" 2>&1 3>&1
```

### Configuration Files

#### Pipeline Configuration
**File**: `.config/pipeline.config.toml`
```toml
cid = 127
port = 53000
```

#### Remote Attestation Configuration
**File**: `.config/ra_web_srv.config.toml`
```toml
[ports]
http = 8080
https = 8443

[keys]
sk4proofs = ""  # Auto-generated if empty
sk4docs = ""
vrf_cipher_suite = "SECP256R1_SHA256_TAI"

[nats]
nats_persistency_enabled = 1
nats_url = "nats://127.0.0.1:4222"
hash_bucket_name = "fs_hashes"
att_docs_bucket_name = "fs_att_docs"
```

---

## Part 5: Understanding Reproducible Builds

### Why Reproducibility Matters

Reproducible builds ensure that:
- **Same source = Same binary** - Every build produces identical output
- **Verifiable PCRs** - The enclave's identity can be verified
- **Audit trail** - Anyone can reproduce and verify builds
- **Trust** - Users can verify they're running the expected code

### PCR (Platform Configuration Registers)

| PCR | Description | Use Case |
|-----|-------------|----------|
| PCR0 | Enclave image hash | Verify correct image |
| PCR1 | Kernel + bootstrap hash | Verify kernel integrity |
| PCR2 | Application hash | Verify app code |
| PCR8 | Signing certificate | Verify signer identity |

### Verifying Build Reproducibility

```bash
# Build twice and compare PCRs
./rbuilds.sh --cmd "make_all" 2>&1 3>&1

# PCRs are saved in:
cat ./eif/init_c_eif/app-builder-secure-enclaves-framework.eif.pcr

# Compare with expected values
curl -k "https://127.0.0.1:8443/pcrs/"
```

---

## Part 6: Troubleshooting

### Common Issues

#### "Nitro Enclaves not enabled"
```bash
# Check if running on supported instance type
aws ec2 describe-instance-types --instance-types $(curl -s http://169.254.169.254/latest/meta-data/instance-type) \
  --query 'InstanceTypes[].NitroEnclavesSupport.Enabled'

# Should return: "supported"
```

#### "Not enough memory"
```bash
# Check allocator configuration
cat /etc/nitro_enclaves/allocator.yaml

# Modify memory allocation
sudo vi /etc/nitro_enclaves/allocator.yaml
# Set: memory_mib: 4096 (or more)

# Restart allocator
sudo systemctl restart nitro-enclaves-allocator
```

#### "Connection refused" to enclave
```bash
# Verify enclave is running
nitro-cli describe-enclaves

# Check Pipeline server is listening (in debug console)
# Look for: "Pipeline listening on port 53000"

# Verify CID matches configuration
cat .config/pipeline.config.toml
```

#### Build fails with "out of disk space"
```bash
# Clean Docker resources
docker system prune -a

# Clean build artifacts
./rbuilds.sh --cmd "make_clear"

# Check disk space
df -h
```

### Debug Mode

```bash
# Run with verbose output
./rbuilds.sh --tty --debug --cmd "make_all" 2>&1 3>&1

# View build logs
cat ./eif/make_build.log
cat ./eif/run-enclave.log

# View enclave debug console
./rbuilds.sh --cmd "attach_console_to_enclave"

# View application logs (inside enclave via Pipeline)
./pipeline run -- cat /apps/.logs/ra-web-srv.log
```

---

## Part 7: Next Steps

### Learn More

1. **Explore the Components**
   - Read `pipeline/README.md` and `pipeline/CLI-REFERENCE.md` for VSock secure local channel communication details
   - Read `pf-proxy/README.md` and `pf-proxy/CLI-REFERENCE.md` for VSock proxy enclave's networking
   - Read `rbuilds/README.md` for enclave's reproducible image build system reference
   - Read `ra-web-srv/README.md` for attestation API reference
   - Read `fs-monitor/README.md` for file integrity monitoring usage and CoW FS layer for enclave's ramdisk
   - Read `enclave-init/README.md` for init system configuration and usage
   - Read `enclave-engine/README.md` for enclave's provisioning system and usage of CVM launcher for KVM/QEMU

2. **Build Your Own Application**
   - Create a custom Dockerfile
   - Add your application to `/apps/`
   - Configure services in init system

3. **Production Deployment**
   - Remove debug mode
   - Configure proper TLS certificates
   - Set up IAM roles for KMS integration
   - Configure networking as needed

### Resources

- [AWS Nitro Enclaves Documentation](https://docs.aws.amazon.com/enclaves/)
- [VSOCK Protocol Reference](https://man7.org/linux/man-pages/man7/vsock.7.html)
- Project documentation in `docs/` directory

---

## Quick Command Cheat Sheet

```bash
# === SETUP ===
./rbuilds.sh --cmd "make_nitro"           # Install Nitro Enclaves

# === BUILD ===
./rbuilds.sh --network --cmd "make_all"   # Build everything
./rbuilds.sh --cmd "make_kernel"          # Just kernel
./rbuilds.sh --cmd "make_apps"            # Just applications
./rbuilds.sh --cmd "make_eif"             # Just EIF image

# === RUN ===
./rbuilds.sh --network --cmd "run_eif_image_debugmode_cli"  # Debug mode
./rbuilds.sh --network --cmd "run_eif_image"                # Production

# === MANAGE ===
./rbuilds.sh --cmd "list_enclaves"        # List running
./rbuilds.sh --cmd "attach_console_to_enclave"  # View output
./rbuilds.sh --cmd "drop_enclave"         # Stop enclave
./rbuilds.sh --cmd "drop_enclaves_all"    # Stop all

# === INTERACT ===
./pipeline run -- <command>               # Run command in enclave
./pipeline send-file <src> <dst>          # Send file to enclave
./pipeline recv-file <src> <dst>          # Get file from enclave

# === CLEANUP ===
./rbuilds.sh --cmd "make_clear"           # Remove build containers
```

---

**Happy Building!** 🔐
