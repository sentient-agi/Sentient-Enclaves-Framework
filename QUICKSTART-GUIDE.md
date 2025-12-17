# Sentient Enclaves Framework – Quickstart Guide

This guide is for **application developers** who want to run their apps inside **AWS Nitro Enclaves** using the Secure Enclaves Framework and its **reproducible build system** (`rbuilds.sh`).

It focuses on:
- Why this framework is useful for you.
- The minimal steps to go from **Dockerfile → EIF → running enclave**.
- How to plug in your app without touching low-level kernel / Nitro plumbing.

---

## 1. Why Use This Framework?

### Advantages for App Developers

- **You stay at the Dockerfile level**
  Describe your app in a Dockerfile; the framework takes care of:
  - Building a Nitro-ready Linux kernel.
  - Packaging your app + dependencies into an EIF (enclave image file for AWS Nitro Enclaves run-time).
  - Bootstrapping VSock secure local channel, VSock networking and proxies, attestation, and FS monitoring.

- **Consistent, reproducible builds**
  `rbuilds.sh` pins toolchains and structure so the same inputs yield the same enclave image. This helps with:
  - Compliance and audits.
  - Debugging (no "works only on my machine" EIFs).
  - Attestation and long-term reproducibility.

- **Batteries included**:
  - Secure Local Channel (SLC) over VSock for commands execution, file and directory transfer, to/from enclave.
  - Forward / reverse / transparent proxies for enclave networking.
  - Attestation web server integrated into the enclave.
  - File system monitor (per-file attestation of runtime data).

- **Evolvable init system**:
  - Today: stable C/clang-based init.
  - In testing: Rust-based `enclave-init` with better supervision, logging, and safety.
  - You don’t need to implement PID 1; the framework does.

### When You Should Use It

Use this framework if you:

- Need to run sensitive workloads in AWS Nitro Enclaves.
- Want a **repeatable way** to build enclave images from your app's Dockerfile.
- Don't want to maintain custom kernels, initramfs, Nitro wiring, and VSock plumbing yourself.

---

## 2. Core Concepts (60‑second mental model)

- **Dockerfile** – describes your app environment (OS, deps, binaries). This is your main responsibility.
- **`rbuilds.sh`** – orchestrator that:
  - Builds a Nitro-ready kernel.
  - Builds the init system + enclave services (framework components).
  - Export Docker container image and convert it into `initramfs` (enclave kernel ramdisk) CPIO format.
  - Assembles everything into an **EIF**.
  - Provides commands to run and manage enclaves.
- **EIF (Enclave Image File)** – the final image Nitro Enclaves run.
- **Pipeline VSock Secure Local Channel** - how you interact with enclave for run commands and file/directory transfers (for UX similar to `docker exec` and `docker cp`).
- **VSock & Proxies** – how the enclave talks (indirectly) to the outside world.
- **Attestation & FS monitor** – how the enclave proves what it’s running and what data it touches.

---

## 3. Prerequisites

On an AWS EC2 instance that supports **Nitro Enclaves**:

- Nitro Enclaves enabled on the instance.
- A modern Linux (e.g., Amazon Linux 2023).
- `docker` (or compatible container runtime).
- `nitro-cli` installed and working.
- Basic shell tools: `bash`, `time`, `tee`.

Clone the repo and move into it:

```bash
git clone https://github.com/sentient-agi/Sentient-Enclaves-Framework.git
cd Sentient-Enclaves-Framework
```

---

## 4. TL;DR Flow

1. **Write a Dockerfile** for your app (e.g., `my-app-enclave.dockerfile`).
2. **Run `rbuilds.sh`** to build everything:
   - Kernel, init and system services, framework components, your apps and services, EIF, enclave.
3. **Run the enclave** (debug or normal mode).
4. **Attach to the enclave console**, test your app, iterate.

Everything else (VSock, SLC, proxies, attestation, FS monitor) is handled by the framework.

---

## 5. Step-by-Step: From Dockerfile to Running Enclave

### Step 1 – Create your app Dockerfile

Example skeleton (`my-app-enclave.dockerfile`):

```dockerfile
FROM amazonlinux:2023

# System deps
RUN yum update -y && \
    yum install -y python3 git && \
    yum clean all

# App code
WORKDIR /app
COPY . /app

# Install app dependencies (example)
RUN pip3 install -r requirements.txt

# Default command (can be overridden by framework env/cmd)
CMD ["python3", "main.py"]
```

Keep it minimal; the framework will add kernel, init, and infrastructure around it.

---

### Step 2 – Build all stages (kernel, init & services, framework components/apps, rootfs, your app, EIF, enclave)

From repo root:

```bash
mkdir -vp ./eif/; \
/usr/bin/time -v -o ./eif/make_build.log \
./rbuilds/rbuilds.sh \
  --tty \
  --debug \
  --dockerfile ./my-app-enclave.dockerfile \
  --network \
  --init-c \
  --cmd "make_all" \
  2>&1 3>&1 | tee ./eif/make_build.output
```

What this does for you:

- Compiles a Nitro-ready kernel.
- Builds the SLC, proxies, attestation server, FS monitor.
- Builds the init system.
- Packages them with your app (in `rootfs`) into an EIF.
- Prepares the enclave configuration.

You end up with:

- Logs under `./eif/`.
- One or more `*.eif` files you can run.

---

### Step 3 – Run the enclave

Debug mode (recommended for first run):

```bash
./rbuilds/rbuilds.sh --tty --debug --network --init-c \
  --cmd "run_eif_image_debugmode_cli" 2>&1 3>&1
```

Normal mode:

```bash
./rbuilds/rbuilds.sh --tty --debug --network --init-c \
  --cmd "run_eif_image" 2>&1 3>&1
```

Behind the scenes this:

- Creates an enclave from the EIF.
- Boots the custom kernel.
- Starts init, which brings up SLC, proxies, attestation server, FS monitor, and your app.

---

### Step 4 – Inspect and manage enclaves

Attach to a running enclave’s console:

```bash
./rbuilds/rbuilds.sh --tty --debug --network --init-c \
  --cmd "attach_console_to_enclave" 2>&1 3>&1
```

List enclaves:

```bash
./rbuilds/rbuilds.sh --tty --debug --network --init-c \
  --cmd "list_enclaves" 2>&1 3>&1
```

Drop one enclave:

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

## 6. How To Customize App Behavior

### Control the main command

Inside the enclave rootfs, the framework uses `rbuilds/rootfs_base/cmd` and environment variables (from `rbuilds/rootfs_base/env`) to define the **primary application command**.

Common patterns:

- Keep your Dockerfile’s default `CMD` simple (e.g., `main.py`).
- Override or extend behavior using the framework’s `cmd` and `env` files when you need more control:
  - Different app entrypoint in different environments.
  - Extra flags or configuration.
- Or add your app `CMD` (or `ENTRY`) and `ENV` from Dockerfile directives/commands into [`rbuilds/enclave.init/init.sh`](rbuilds/enclave.init/init.sh) init shell script using Bash equivalent commands (`env KEY=VALUE your_app_cmd --key value`).

**Note:**
See [`enclave-init/README.md`](enclave-init/README.md), which describes new way of launching apps in enclaves using new Enclave's Init system service files with syntax similar to `systemd` services.

### Using the Secure Local Channel (SLC)

Once the enclave is running:

- SLC lets you **execute commands inside the enclave** from the host (over VSock).
- You can also **upload/download files and directories** via SLC.

This is useful for:

- Pushing updated configs or models into the enclave.
- Pulling logs or outputs out without exposing direct network access.

(Refer to the SLC-specific docs for exact CLI usage in [pipeline/README.md](pipeline/README.md) and [pipeline/CLI-REFERENCE.md](pipeline/CLI-REFERENCE.md).)

---

## 7. Why Reproducible Builds Matter (and How You Benefit)

Even if you "just want to ship features", reproducible builds are a big deal in enclaves:

- **Verifiable Attestation**
  The attestation document includes hashes of your kernel, init, and rootfs. Reproducible builds mean a verifier can:
  - Rebuild from source.
  - Get the same measurement.
  - Confidently say "this enclave is running the code we reviewed".

- **Easy rollback / roll-forward**
  If a change breaks something, you can rebuild the previous version and get the **exact same EIF** as before.

- **Auditability & compliance**
  Regulated environments (finance, healthcare, etc.) care about **deterministic artifacts**. Having a reproducible pipeline is a huge plus.

- **Debugging without guesswork**
  "It works on my machine" doesn’t cut it for enclaves. Reproducibility lets you:
  - Reproduce bugs across environments.
  - Share exact build inputs with other teams.

And you get all of this **without having to maintain the build system yourself** – it’s baked into `rbuilds.sh`.

---

## 8. Next Steps & Deeper Dives

Once you have the basic flow working:

- Read the main `rbuilds.sh` README for:
  - Full stage breakdown (`make_kernel`, `make_apps`, `make_init`, `make_eif`, `make_enclave`).
  - Advanced automation shell usage.
- Explore the **Rust `enclave-init`** if you:
  - Want more structured service management.
  - Care about advanced logging and health checks.
- Integrate the **attestation web server** with your backend:
  - Verify enclave measurements.
  - Validate that specific models / data were loaded.

**Note:**
Every component of Enclaves Frmaework has its own `README` and documentation with reference guide, placed in the corresponding component directory, so please refer to it.

For most app developers, however, the core loop is:

1. Edit Dockerfile.
2. `rbuilds.sh --cmd make_all`.
3. `rbuilds.sh --cmd run_eif_image_debugmode_cli`.
4. Test, iterate, repeat.

That’s enough to start shipping enclave-based applications with strong security guarantees (via attestation, FS granular proofs and hashing) and a robust, reproducible build story.

