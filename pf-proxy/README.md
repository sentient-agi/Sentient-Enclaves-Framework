# PF-Proxy: Enclave Port Forwarding Proxy

PF-Proxy is a high-performance, asynchronous port forwarding proxy suite designed for AWS Nitro Enclaves and other vsock-based environments. It provides bidirectional communication between vsock (Virtual Socket) and IP networks with support for transparent proxying.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Binaries](#binaries)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [CLI Reference](#cli-reference)
- [Use Cases](#use-cases)
- [Technical Details](#technical-details)
- [Troubleshooting](#troubleshooting)
- [License](#license)

## Overview

PF-Proxy enables secure communication between isolated enclaves and the outside world by providing proxy binaries that:

- Forward traffic between vsock and TCP/IP networks
- Support transparent proxying with original destination preservation
- Handle concurrent connections asynchronously using Tokio
- Provide bidirectional data flow with automatic cleanup

### Key Features

- **Five specialized proxy modes** for different networking scenarios
- **Transparent proxy support** for seamless network interception
- **Asynchronous I/O** using Tokio for high performance
- **Automatic connection management** with proper cleanup
- **Original destination preservation** in transparent modes
- **IPv4 and IPv6 support** in transparent proxies
- **Linux-optimized** with SO_ORIGINAL_DST support

## Architecture

The crate consists of:

1. **Core Library** (`lib.rs`):
   - Vsock address parsing utilities
   - Original destination retrieval (Linux-specific)
   - Shared error handling

2. **Five Proxy Binaries**:
   - `vsock-to-ip`: Forward vsock connections to TCP/IP
   - `ip-to-vsock`: Forward TCP/IP connections to vsock
   - `vsock-to-ip-transparent`: Transparent proxy from vsock to IP
   - `ip-to-vsock-transparent`: Transparent proxy from IP to vsock
   - `transparent-port-to-vsock`: Port-based transparent proxy to vsock

### Data Flow

```
┌─────────────────────────────────────────────────────────────┐
│                      vsock-to-ip                            │
│  Enclave (vsock:4000) ──→ Proxy ──→ TCP (127.0.0.1:8080)    │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                      ip-to-vsock                            │
│  TCP (0.0.0.0:3000) ──→ Proxy ──→ Enclave (vsock:88:5000)   │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                vsock-to-ip-transparent                      │
│  Enclave (vsock:3:1200) ──→ Proxy ──→ Original TCP Dest     │
│  (Original dest transmitted via vsock stream)               │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│              ip-to-vsock-transparent                        │
│  TCP (127.0.0.1:1200) ──→ Proxy ──→ Enclave (vsock:3:port)  │
│  (Original dest read from SO_ORIGINAL_DST)                  │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│           transparent-port-to-vsock                         │
│  TCP (127.0.0.1:1200) ──→ Proxy ──→ Enclave (vsock:88:port) │
│  (Port extracted from SO_ORIGINAL_DST)                      │
└─────────────────────────────────────────────────────────────┘
```

## Binaries

### 1. `vsock-to-ip`
Forwards connections from a vsock listener to a TCP/IP endpoint.

**Use Case:** Enclave applications need to connect to host or external IP services.

### 2. `ip-to-vsock`
Forwards connections from a TCP/IP listener to a vsock endpoint.

**Use Case:** Host applications need to connect to enclave services.

### 3. `vsock-to-ip-transparent`
Transparent proxy that reads the original destination from the vsock stream and connects to it.

**Use Case:** Enclave transparently accesses any IP address, with the original destination determined by the enclave-side proxy.

### 4. `ip-to-vsock-transparent`
Transparent proxy that retrieves the original destination using `SO_ORIGINAL_DST` and forwards it to vsock.

**Use Case:** Intercept TCP traffic on the host and forward to enclave while preserving original destination information.

### 5. `transparent-port-to-vsock`
Transparent proxy that extracts the destination port from `SO_ORIGINAL_DST` and connects to a vsock endpoint with a specified CID.

**Use Case:** Port-based routing to enclave services while maintaining original port numbers.

## Installation

### Build from Source

```bash
# Clone the repository
cd pf-proxy

# Build all binaries
cargo build --release

# Binaries will be available in:
# target/release/vsock-to-ip
# target/release/ip-to-vsock
# target/release/vsock-to-ip-transparent
# target/release/ip-to-vsock-transparent
# target/release/transparent-port-to-vsock
```

### Build Individual Binary

```bash
cargo build --release --bin vsock-to-ip
```

## Quick Start

### Example 1: Simple vsock to IP forwarding

Forward enclave connections on vsock `88:4000` to a host service at `127.0.0.1:8080`:

```bash
./vsock-to-ip --vsock-addr 88:4000 --ip-addr 127.0.0.1:8080
```

### Example 2: IP to vsock forwarding

Listen on host `0.0.0.0:3000` and forward to enclave vsock `88:5000`:

```bash
./ip-to-vsock --ip-addr 0.0.0.0:3000 --vsock-addr 88:5000
```

### Example 3: Transparent proxy from enclave

Listen on vsock `3:1200` and transparently connect to any IP (destination sent via vsock):

```bash
./vsock-to-ip-transparent --vsock-addr 3:1200
```

### Example 4: Transparent proxy to enclave

Use with iptables to intercept traffic and forward to enclave:

```bash
# Setup iptables rule
sudo iptables -t nat -A OUTPUT -p tcp -j REDIRECT --to-ports 1200

# Run proxy
./ip-to-vsock-transparent --ip-addr 127.0.0.1:1200 --vsock-addr 3:1200
```

### Example 5: Port-based transparent proxy

Forward intercepted traffic to enclave CID 88, preserving original ports:

```bash
# Setup iptables rule
sudo iptables -t nat -A OUTPUT -p tcp -j REDIRECT --to-ports 1200

# Run proxy
./transparent-port-to-vsock --ip-addr 127.0.0.1:1200 --vsock 88
```

## CLI Reference

### Common Concepts

**Vsock Address Format:** `CID:PORT`
- `CID` (Context ID): Unique identifier for vsock endpoint (e.g., 88, 3)
- `PORT`: Port number (e.g., 4000, 1200)
- Example: `88:4000`, `3:1200`

**IP Address Format:** `IP:PORT`
- Standard TCP/IP address notation
- Examples: `127.0.0.1:8080`, `0.0.0.0:3000`, `192.168.1.10:9000`

---

### 1. vsock-to-ip

**Description:** Forwards connections from vsock to TCP/IP endpoint.

**Usage:**
```bash
vsock-to-ip [OPTIONS]
```

**Options:**

| Flag | Long Form | Type | Required | Description |
|------|-----------|------|----------|-------------|
| `-v` | `--vsock-addr` | String | Yes | Vsock address of the listener (format: `CID:PORT`)<br>Example: `88:4000` |
| `-i` | `--ip-addr` | String | Yes | IP address of the upstream target (format: `IP:PORT`)<br>Example: `127.0.0.1:8080` |
| `-h` | `--help` | - | No | Print help information |
| `-V` | `--version` | - | No | Print version information |

**Examples:**

```bash
# Listen on vsock 88:4000, forward to localhost:8080
vsock-to-ip -v 88:4000 -i 127.0.0.1:8080

# Forward enclave service to external server
vsock-to-ip --vsock-addr 3:5000 --ip-addr 192.168.1.100:9000

# Forward to IPv6 address (if supported)
vsock-to-ip -v 88:3000 -i [::1]:8080
```

**Typical Use Cases:**
- Enclave needs to access host HTTP service
- Enclave database client connecting to host database
- Enclave logging to host log collector

**Output:**
```
Listening on: VsockAddr { cid: 88, port: 4000 }
Proxying to: "127.0.0.1:8080"
Proxying to: "127.0.0.1:8080"
vsock to ip IO copy done, from "88:4000" to "127.0.0.1:8080"
ip to vsock IO copy done, from "127.0.0.1:8080" to "88:4000"
```

---

### 2. ip-to-vsock

**Description:** Forwards connections from TCP/IP listener to vsock endpoint.

**Usage:**
```bash
ip-to-vsock [OPTIONS]
```

**Options:**

| Flag | Long Form | Type | Required | Description |
|------|-----------|------|----------|-------------|
| `-i` | `--ip-addr` | String | Yes | IP address of the listener (format: `IP:PORT`)<br>Example: `0.0.0.0:3000` |
| `-v` | `--vsock-addr` | String | Yes | Vsock address of the upstream target (format: `CID:PORT`)<br>Example: `88:5000` |
| `-h` | `--help` | - | No | Print help information |
| `-V` | `--version` | - | No | Print version information |

**Examples:**

```bash
# Listen on all interfaces port 3000, forward to enclave vsock 88:5000
ip-to-vsock -i 0.0.0.0:3000 -v 88:5000

# Listen on localhost only, forward to enclave
ip-to-vsock --ip-addr 127.0.0.1:8080 --vsock-addr 3:4000

# Listen on specific interface
ip-to-vsock -i 192.168.1.10:9000 -v 88:9000
```

**Typical Use Cases:**
- Host web clients accessing enclave web service
- External services connecting to enclave API
- Load balancer forwarding to enclave backend

**Output:**
```
Listening on: "0.0.0.0:3000"
Proxying to: VsockAddr { cid: 88, port: 5000 }
Proxying to: VsockAddr { cid: 88, port: 5000 }
ip to vsock IO copy done, from "192.168.1.5:52314" to VsockAddr { cid: 88, port: 5000 }
vsock to ip IO copy done, from VsockAddr { cid: 88, port: 5000 } to "192.168.1.5:52314"
```

---

### 3. vsock-to-ip-transparent

**Description:** Transparent proxy from vsock to IP. Reads original destination from vsock stream.

**Usage:**
```bash
vsock-to-ip-transparent [OPTIONS]
```

**Options:**

| Flag | Long Form | Type | Required | Description |
|------|-----------|------|----------|-------------|
| `-v` | `--vsock-addr` | String | Yes | Vsock address of the listener, open to the transparent proxy sender (format: `CID:PORT`)<br>Example: `3:1200` |
| `-h` | `--help` | - | No | Print help information |
| `-V` | `--version` | - | No | Print version information |

**Protocol Details:**

The proxy expects the original destination to be sent through the vsock stream in the following format:

**For IPv4:**
```
Byte 0:     4 (u8)              - IP version indicator
Bytes 1-4:  IP address (u32_le) - IPv4 address in little-endian
Bytes 5-6:  Port (u16_le)       - Port number in little-endian
```

**For IPv6:**
```
Byte 0:      6 (u8)               - IP version indicator
Bytes 1-16:  IP address (u128_le) - IPv6 address in little-endian
Bytes 17-18: Port (u16_le)        - Port number in little-endian
```

**Examples:**

```bash
# Listen on vsock 3:1200 for transparent proxy connections
vsock-to-ip-transparent -v 3:1200

# Use with enclave-side transparent proxy
vsock-to-ip-transparent --vsock-addr 88:8080
```

**Typical Use Cases:**
- Enclave with iptables redirect sending traffic to host
- Dynamic destination routing based on enclave logic
- Multi-destination enclave applications

**Pairing:** Must be paired with `ip-to-vsock-transparent` running in the enclave.

**Output:**
```
Listening on: VsockAddr { cid: 3, port: 1200 }
Proxying to: 93.184.216.34:80
vsock to ip IO copy done, from "3:1200" to 93.184.216.34:80
ip to vsock IO copy done, from 93.184.216.34:80 to "3:1200"
```

---

### 4. ip-to-vsock-transparent

**Description:** Transparent proxy from IP to vsock. Retrieves original destination using `SO_ORIGINAL_DST` (Linux only).

**Usage:**
```bash
ip-to-vsock-transparent [OPTIONS]
```

**Options:**

| Flag | Long Form | Type | Required | Description |
|------|-----------|------|----------|-------------|
| `-i` | `--ip-addr` | String | Yes | IP address of the listener (format: `IP:PORT`)<br>Example: `127.0.0.1:1200` |
| `-v` | `--vsock-addr` | String | Yes | Vsock address of the upstream transparent proxy receiver (format: `CID:PORT`)<br>Example: `3:1200` |
| `-h` | `--help` | - | No | Print help information |
| `-V` | `--version` | - | No | Print version information |

**Requirements:**
- **Linux only** (uses `SO_ORIGINAL_DST`)
- Must be used with iptables REDIRECT or TPROXY rules
- Traffic must be intercepted before reaching this proxy

**Protocol Details:**

The proxy sends the original destination through the vsock stream:

**For IPv4:**
```
Byte 0:     4 (u8)              - IP version indicator
Bytes 1-4:  IP address (u32_le) - IPv4 address in little-endian
Bytes 5-6:  Port (u16_le)       - Port number in little-endian
```

**For IPv6:**
```
Byte 0:      6 (u8)               - IP version indicator
Bytes 1-16:  IP address (u128_le) - IPv6 address in little-endian
Bytes 17-18: Port (u16_le)        - Port number in little-endian
```

**Examples:**

```bash
# Basic transparent proxy setup
sudo iptables -t nat -A OUTPUT -p tcp --dport 80 -j REDIRECT --to-ports 1200
ip-to-vsock-transparent -i 127.0.0.1:1200 -v 3:1200

# Intercept HTTPS traffic
sudo iptables -t nat -A OUTPUT -p tcp --dport 443 -j REDIRECT --to-ports 1200
ip-to-vsock-transparent --ip-addr 127.0.0.1:1200 --vsock-addr 3:1200

# Intercept traffic from specific source
sudo iptables -t nat -A OUTPUT -p tcp -s 192.168.1.0/24 -j REDIRECT --to-ports 1200
ip-to-vsock-transparent -i 192.168.1.1:1200 -v 88:1200
```

**Typical Use Cases:**
- Intercept enclave outbound HTTP/HTTPS traffic
- Policy enforcement at network layer
- Traffic monitoring and logging
- Content filtering for enclave applications

**Pairing:** Must be paired with `vsock-to-ip-transparent` running on the host.

**Output:**
```
Listening on: "127.0.0.1:1200"
Proxying to: VsockAddr { cid: 3, port: 1200 }
Original destination: 93.184.216.34:80
Proxying to: VsockAddr { cid: 3, port: 1200 }
ip to vsock IO copy done, from "127.0.0.1:45678" to VsockAddr { cid: 3, port: 1200 }, with original_dst=93.184.216.34:80 from inbound TCP stream
vsock to ip IO copy done, from VsockAddr { cid: 3, port: 1200 } to "127.0.0.1:45678", with original_dst=93.184.216.34:80 from inbound TCP stream
```

**iptables Setup:**

```bash
# Redirect all outbound HTTP traffic
sudo iptables -t nat -A OUTPUT -p tcp --dport 80 -j REDIRECT --to-ports 1200

# Redirect specific subnet
sudo iptables -t nat -A OUTPUT -p tcp -s 10.0.0.0/8 -j REDIRECT --to-ports 1200

# Redirect multiple ports
sudo iptables -t nat -A OUTPUT -p tcp -m multiport --dports 80,443 -j REDIRECT --to-ports 1200

# Remove rules
sudo iptables -t nat -D OUTPUT -p tcp --dport 80 -j REDIRECT --to-ports 1200
```

---

### 5. transparent-port-to-vsock

**Description:** Transparent proxy that forwards to vsock using original destination port with a specified CID.

**Usage:**
```bash
transparent-port-to-vsock [OPTIONS]
```

**Options:**

| Flag | Long Form | Type | Required | Description |
|------|-----------|------|----------|-------------|
| `-i` | `--ip-addr` | String | Yes | IP address of the listener (format: `IP:PORT`)<br>Example: `127.0.0.1:1200` |
| `-v` | `--vsock` | u32 | Yes | CID from vsock address of the upstream side<br>Example: `88` (from `88:PORT` specification) |
| `-h` | `--help` | - | No | Print help information |
| `-V` | `--version` | - | No | Print version information |

**Requirements:**
- **Linux only** (uses `SO_ORIGINAL_DST`)
- Must be used with iptables REDIRECT rules
- Traffic must be intercepted before reaching this proxy

**How It Works:**
1. Listens on specified IP address
2. Retrieves original destination using `SO_ORIGINAL_DST`
3. Extracts the destination port
4. Connects to vsock at `CID:original_port`
5. Forwards bidirectional traffic

**Examples:**

```bash
# Forward intercepted traffic to enclave CID 88, preserving ports
sudo iptables -t nat -A OUTPUT -p tcp -j REDIRECT --to-ports 1200
transparent-port-to-vsock -i 127.0.0.1:1200 -v 88

# Port-based routing to specific enclave
sudo iptables -t nat -A OUTPUT -p tcp --dport 8080 -j REDIRECT --to-ports 1200
transparent-port-to-vsock --ip-addr 127.0.0.1:1200 --vsock 3

# Multiple service ports to same enclave
sudo iptables -t nat -A OUTPUT -p tcp -m multiport --dports 80,443,8080 -j REDIRECT --to-ports 1200
transparent-port-to-vsock -i 0.0.0.0:1200 -v 88
```

**Typical Use Cases:**
- Port-based service routing to enclave
- Microservices architecture with port preservation
- Multi-service enclave applications
- Service discovery using port numbers

**Example Scenario:**

```bash
# Enclave runs multiple services:
# - HTTP on vsock 88:80
# - HTTPS on vsock 88:443
# - API on vsock 88:8080

# Setup iptables to redirect all traffic to proxy
sudo iptables -t nat -A OUTPUT -p tcp -m multiport --dports 80,443,8080 -j REDIRECT --to-ports 1200

# Run proxy - original port determines enclave service
transparent-port-to-vsock -i 127.0.0.1:1200 -v 88

# Now:
# curl http://example.com:80   -> routes to vsock 88:80
# curl https://example.com:443 -> routes to vsock 88:443
# curl http://api.example.com:8080 -> routes to vsock 88:8080
```

**Output:**
```
Listening on: "127.0.0.1:1200"
Proxying to: 88
Original destination: 93.184.216.34:80
Proxying to: VsockAddr { cid: 88, port: 80 }
port to vsock IO copy done, from "127.0.0.1:45678" to VsockAddr { cid: 88, port: 80 }, with original_dst=93.184.216.34:80, ip=93.184.216.34, port=80, from inbound TCP stream
vsock to port IO copy done, from VsockAddr { cid: 88, port: 80 } to "127.0.0.1:45678", with original_dst=93.184.216.34:80, ip=93.184.216.34, port=80, from inbound TCP stream
```

**iptables Setup:**

```bash
# Redirect all TCP traffic
sudo iptables -t nat -A OUTPUT -p tcp -j REDIRECT --to-ports 1200

# Redirect specific ports
sudo iptables -t nat -A OUTPUT -p tcp -m multiport --dports 80,443,8080 -j REDIRECT --to-ports 1200

# Redirect from PREROUTING for incoming traffic
sudo iptables -t nat -A PREROUTING -p tcp --dport 80 -j REDIRECT --to-ports 1200

# Remove rules
sudo iptables -t nat -D OUTPUT -p tcp -j REDIRECT --to-ports 1200
```

---

## Use Cases

### Scenario 1: Enclave Web Service

**Requirement:** Host needs to access web service running in enclave.

**Solution:**
```bash
# In enclave, service runs on vsock 88:8080
# On host:
ip-to-vsock -i 0.0.0.0:8080 -v 88:8080
# Access via: curl http://localhost:8080
```

### Scenario 2: Enclave Database Client

**Requirement:** Enclave application needs to connect to host PostgreSQL.

**Solution:**
```bash
# PostgreSQL runs on host at 127.0.0.1:5432
# In enclave:
vsock-to-ip -v 88:5432 -i 127.0.0.1:5432
# Enclave connects to vsock 88:5432
```

### Scenario 3: Transparent Internet Access

**Requirement:** Enclave needs unrestricted internet access with destination determined at runtime.

**Solution:**
```bash
# In enclave:
sudo iptables -t nat -A OUTPUT -p tcp -j REDIRECT --to-ports 1200
ip-to-vsock-transparent -i 127.0.0.1:1200 -v 3:1200

# On host:
vsock-to-ip-transparent -v 3:1200
```

### Scenario 4: Multi-Service Enclave

**Requirement:** Enclave runs multiple services on different ports (HTTP:80, HTTPS:443, API:8080).

**Solution:**
```bash
# On host with iptables redirection:
sudo iptables -t nat -A OUTPUT -p tcp -m multiport --dports 80,443,8080 -j REDIRECT --to-ports 1200
transparent-port-to-vsock -i 127.0.0.1:1200 -v 88

# Enclave services listen on:
# - vsock 88:80 (HTTP)
# - vsock 88:443 (HTTPS)
# - vsock 88:8080 (API)
```

### Scenario 5: Secure Logging

**Requirement:** Enclave logs need to be sent to external log aggregator.

**Solution:**
```bash
# Log aggregator at logs.company.com:514
# In enclave:
vsock-to-ip -v 88:514 -i logs.company.com:514
# Enclave logs to vsock 88:514
```

## Technical Details

### Vsock (Virtual Socket)

Vsock is a socket address family designed for communication between virtual machines and their host. Key concepts:

- **CID (Context ID):** Unique identifier for each vsock endpoint
  - Host typically uses CID 2
  - Enclaves receive dynamic CIDs (e.g., 3, 88, 16)
  - Special CID values:
    - `VMADDR_CID_ANY` (-1U): Bind to any CID
    - `VMADDR_CID_HYPERVISOR` (0): Reserved for hypervisor
    - `VMADDR_CID_HOST` (2): Host system

- **Port:** Similar to TCP/IP ports (0-65535)

### SO_ORIGINAL_DST

`SO_ORIGINAL_DST` is a Linux socket option that retrieves the original destination of a redirected TCP connection. Used with:

- **iptables REDIRECT:** Changes destination address/port
- **iptables TPROXY:** Transparent proxy support

**Requirements:**
- Linux kernel with Netfilter support
- iptables rules to redirect traffic
- Socket must be obtained via `accept()` after iptables redirection

### Async I/O Architecture

All proxies use Tokio async runtime:

```rust
// Concurrent connection handling
tokio::spawn(async move {
    // Each connection handled in separate task
    let (mut read_inbound, mut write_inbound) = inbound.split();
    let (mut read_outbound, mut write_outbound) = outbound.split();

    tokio::try_join!(
        io::copy(&mut read_inbound, &mut write_outbound),
        io::copy(&mut read_outbound, &mut write_inbound)
    )
});
```

**Benefits:**
- Concurrent connection handling
- Efficient resource utilization
- Automatic task cleanup
- Bidirectional data flow

### Error Handling

All proxies use `anyhow::Result` for comprehensive error propagation:

```text
- Connection failures: Logged and task terminated
- Bind failures: Immediate program exit with error
- I/O errors: Connection-specific, doesn't affect other connections
- Parse errors: Validation at startup
```

### Connection Lifecycle

1. **Accept:** Proxy accepts inbound connection
2. **Connect:** Establishes outbound connection
3. **Split:** Splits streams into read/write halves
4. **Copy:** Bidirectional async copy operations
5. **Shutdown:** Graceful shutdown on completion
6. **Cleanup:** Automatic task cleanup via Tokio

### Performance Considerations

- **Async I/O:** Non-blocking operations via Tokio
- **Zero-copy:** Efficient buffer management
- **Concurrent:** Multiple connections without thread overhead
- **Resource limits:** Governed by OS file descriptor limits

### Platform Support

| Feature | Linux | Other OS |
|---------|-------|----------|
| Basic proxies (vsock-to-ip, ip-to-vsock) | ✅ | ✅ (if vsock available) |
| Transparent proxies (SO_ORIGINAL_DST) | ✅ | ❌ |
| IPv4 support | ✅ | ✅ |
| IPv6 support (transparent) | ✅ | ❌ |

## Troubleshooting

### Common Issues

#### 1. "Failed to bind listener to vsock: incorrect CID:port"

**Cause:** Invalid vsock address or permissions.

**Solutions:**
```bash
# Verify vsock format: CID:PORT
# Example: 88:4000, not 88-4000 or 88/4000

# Check if vsock is available
ls -l /dev/vsock

# Verify CID
cat /sys/module/vsock/parameters/cid
```

#### 2. "Failed to bind listener: malformed listening address:port"

**Cause:** Invalid IP address format.

**Solutions:**
```bash
# Use correct format: IP:PORT
# Valid: 0.0.0.0:8080, 127.0.0.1:3000
# Invalid: 0.0.0.0, 8080, localhost:8080
```

#### 3. "Failed to retrieve original destination from TCP stream"

**Cause:** Missing iptables rules or not running on Linux.

**Solutions:**
```bash
# Verify iptables rule exists
sudo iptables -t nat -L OUTPUT -n -v

# Add redirect rule
sudo iptables -t nat -A OUTPUT -p tcp --dport 80 -j REDIRECT --to-ports 1200

# Verify traffic is being redirected
sudo iptables -t nat -L OUTPUT -n -v | grep 1200
```

#### 4. Connection failures

**Cause:** Target service not running or firewall blocking.

**Solutions:**
```bash
# Verify target service is running
# For IP target:
netstat -tuln | grep 8080

# For vsock target:
# Check enclave service logs

# Check firewall
sudo iptables -L -n -v
```

#### 5. "Non Linux system, no support for SO_ORIGINAL_DST"

**Cause:** Running transparent proxy on non-Linux OS.

**Solution:** Transparent proxies require Linux. Use non-transparent proxies on other platforms.

### Debugging

#### Enable verbose logging

Add logging to see all connections:

```rust
// Already included in code
println!("Proxying to: {:?}", proxy_addr);
println!("Original destination: {:?}", orig_dst);
```

#### Test connectivity

```bash
# Test vsock connectivity
# From host to enclave:
nc -U /dev/vsock 88 4000

# Test IP connectivity
nc -zv 127.0.0.1 8080
```

#### Monitor connections

```bash
# Watch active connections
watch -n1 'sudo netstat -tnp | grep vsock'
watch -n1 'sudo ss -tn | grep 1200'

# Monitor iptables packet counts
watch -n1 'sudo iptables -t nat -L OUTPUT -n -v'
```

#### Check vsock devices

```bash
# List vsock devices
ls -l /dev/vsock

# Check vsock module
lsmod | grep vsock
modinfo vsock
```

### Performance Tuning

```bash
# Increase file descriptor limit
ulimit -n 65536

# Tune TCP parameters
sudo sysctl -w net.ipv4.tcp_max_syn_backlog=4096
sudo sysctl -w net.core.somaxconn=1024
sudo sysctl -w net.ipv4.tcp_fin_timeout=15

# Monitor performance
# CPU usage
top -p $(pgrep vsock-to-ip)

# Network stats
sar -n DEV 1
```

## Examples

### Complete Setup Examples

#### Example 1: Bidirectional Enclave Communication

```bash
# Topology:
# Host (CID 2) <-> Enclave (CID 88)
# Host service: 127.0.0.1:8080 (HTTP server)
# Enclave service: vsock 88:9000 (Application)

# On host - forward host traffic to enclave service:
ip-to-vsock -i 0.0.0.0:9000 -v 88:9000

# In enclave - forward enclave traffic to host service:
vsock-to-ip -v 88:8080 -i 127.0.0.1:8080

# Test:
# From host: curl http://localhost:9000 (reaches enclave)
# From enclave: curl http://vsock:88:8080 (reaches host)
```

#### Example 2: Transparent Proxy Chain

```bash
# In enclave - intercept all HTTP traffic:
sudo iptables -t nat -A OUTPUT -p tcp --dport 80 -j REDIRECT --to-ports 1200
ip-to-vsock-transparent -i 127.0.0.1:1200 -v 3:1200

# On host - receive and forward to internet:
vsock-to-ip-transparent -v 3:1200

# Now enclave can make HTTP requests to any destination:
# curl http://example.com (transparently proxied)
```

#### Example 3: Multi-Port Service Mesh

```bash
# Enclave runs:
# - Frontend: vsock 88:80
# - Backend API: vsock 88:8080
# - Admin: vsock 88:9000

# On host:
ip-to-vsock -i 0.0.0.0:80 -v 88:80       # Frontend
ip-to-vsock -i 0.0.0.0:8080 -v 88:8080   # API
ip-to-vsock -i 127.0.0.1:9000 -v 88:9000 # Admin (localhost only)

# Access:
curl http://server-ip:80      # Frontend
curl http://server-ip:8080    # API
curl http://localhost:9000    # Admin
```

## Building and Development

### Prerequisites

- Rust 1.91.0 or later
- Linux (for transparent proxy features)
- vsock support (kernel module or device)

### Build Commands

```bash
# Build all binaries
cargo build --release

# Build specific binary
cargo build --release --bin vsock-to-ip

# Run tests
cargo test

# Check without building
cargo check

# Format code
cargo fmt

# Run clippy lints
cargo clippy
```

### Project Structure

```
pf-proxy/
├── Cargo.toml                          # Package manifest
├── LICENSE-MIT                         # MIT License
├── LICENSE-APACHE                      # Apache License
├── README.md                           # This file
└── src/
    ├── lib.rs                          # Core library (utils, addr_info)
    ├── addr_info.rs                    # SO_ORIGINAL_DST support
    ├── vsock_to_ip.rs                  # vsock-to-ip binary
    ├── ip_to_vsock.rs                  # ip-to-vsock binary
    ├── vsock_to_ip_transparent.rs      # vsock-to-ip-transparent binary
    ├── ip_to_vsock_transparent.rs      # ip-to-vsock-transparent binary
    └── transparent_port_to_vsock.rs    # transparent-port-to-vsock binary
```

### Dependencies

```toml
[dependencies]
anyhow = "1.0.80"           # Error handling
clap = "4.5.1"              # CLI argument parsing
futures = "0.3"             # Async utilities
thiserror = "1.0.57"        # Custom error types
tokio = "1.44"              # Async runtime
tokio-vsock = "0.5.0"       # Vsock support
libc = "0.2"                # Linux system calls (Linux only)
```

## Security Considerations

### Enclave Isolation

- Proxies run **outside** the enclave boundary
- Data passes through vsock in plaintext
- Consider TLS for sensitive data

### Access Control

- Use firewall rules to restrict proxy access
- Bind to localhost (`127.0.0.1`) when possible
- Use iptables rules for fine-grained control

### Transparent Proxy Risks

- Can intercept all traffic if misconfigured
- Ensure iptables rules are specific
- Monitor proxy logs for suspicious activity

### Best Practices

1. **Least Privilege:** Only open necessary ports
2. **Localhost Binding:** Bind to `127.0.0.1` when external access not needed
3. **Monitoring:** Log and monitor all connections
4. **Encryption:** Use TLS over proxied connections for sensitive data
5. **Firewall Rules:** Restrict source/destination IPs in iptables
6. **Regular Updates:** Keep proxies and dependencies updated

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Contributing

Contributions are welcome! Please ensure:

- Code follows Rust style guidelines (rustfmt)
- All tests pass
- New features include documentation
- Security implications are considered

## References

- [Tokio Async Runtime](https://tokio.rs/)
- [tokio-vsock](https://github.com/rust-vsock/tokio-vsock)
- [AWS Nitro Enclaves](https://aws.amazon.com/ec2/nitro/nitro-enclaves/)
- [Linux vsock](https://man7.org/linux/man-pages/man7/vsock.7.html)
- [iptables NAT](https://netfilter.org/documentation/)

## Support

For issues, questions, or contributions, please refer to the project repository.

---

**Version:** 0.8.2

**Last Updated:** 2025-11-07
