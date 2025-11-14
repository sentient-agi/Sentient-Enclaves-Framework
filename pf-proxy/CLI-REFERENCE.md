# PF-Proxy CLI Reference Guide

Complete command-line interface reference for all five PF-Proxy binaries.

## Table of Contents

- [Common Concepts](#common-concepts)
- [vsock-to-ip](#vsock-to-ip)
- [ip-to-vsock](#ip-to-vsock)
- [vsock-to-ip-transparent](#vsock-to-ip-transparent)
- [ip-to-vsock-transparent](#ip-to-vsock-transparent)
- [transparent-port-to-vsock](#transparent-port-to-vsock)
- [Address Formats](#address-formats)
- [Error Codes](#error-codes)
- [Environment Variables](#environment-variables)
- [Output Format](#output-format)

## Common Concepts

### Address Notation

**Vsock Address Format:** `CID:PORT`
- **CID (Context ID):** Unsigned 32-bit integer identifying vsock endpoint
- **PORT:** Unsigned 32-bit integer (typically 0-65535)
- **Examples:** `88:4000`, `3:1200`, `16:8080`

**IP Address Format:** `IP:PORT`
- **IP:** IPv4 or IPv6 address
- **PORT:** TCP port number (0-65535)
- **Examples:** `127.0.0.1:8080`, `0.0.0.0:3000`, `[::1]:8080`

### Common Options

All binaries support:
- `-h, --help`: Display help information
- `-V, --version`: Display version information

### Return Codes

- `0`: Success
- `1`: Error (with error message to stderr)

---

## vsock-to-ip

### Synopsis

```bash
vsock-to-ip [OPTIONS]
```

### Description

Forwards connections from a vsock listener to a TCP/IP endpoint. Listens on a vsock address and proxies all accepted connections to a specified IP address.

### Options

```
-v, --vsock-addr <VSOCK_ADDR>
    Vsock address of the listener side

    Format: CID:PORT

    Where:
        CID  = Context ID (u32)
        PORT = Port number (u32)

    Examples:
        88:4000     Listen on vsock CID 88, port 4000
        3:1200      Listen on vsock CID 3, port 1200
        16:8080     Listen on vsock CID 16, port 8080

    Notes:
        - CID must match the enclave's assigned CID
        - Port must not be in use by another service
        - Special CID values:
            2 = Host
            VMADDR_CID_ANY (-1U) = Any available CID

-i, --ip-addr <IP_ADDR>
    IP address of the upstream side

    Format: IP:PORT

    Where:
        IP   = IPv4 or IPv6 address
        PORT = TCP port number (0-65535)

    Examples:
        127.0.0.1:8080      Localhost port 8080
        192.168.1.10:9000   Specific host and port
        0.0.0.0:3000        All interfaces port 3000
        [::1]:8080          IPv6 localhost
        [2001:db8::1]:443   IPv6 address

    Notes:
        - Must be reachable from proxy host
        - Target service must be listening
        - DNS names not supported (use IP addresses)

-h, --help
    Print help information and exit

-V, --version
    Print version information and exit
```

### Examples

#### Basic Usage

```bash
# Listen on vsock 88:4000, forward to localhost:8080
vsock-to-ip --vsock-addr 88:4000 --ip-addr 127.0.0.1:8080
```

#### Short Form

```bash
vsock-to-ip -v 88:4000 -i 127.0.0.1:8080
```

#### Forward to External Server

```bash
vsock-to-ip -v 3:5000 -i 192.168.1.100:9000
```

#### IPv6 Target

```bash
vsock-to-ip -v 88:3000 -i [::1]:8080
```

#### Multiple Instances

```bash
# Run multiple proxies for different services
vsock-to-ip -v 88:80 -i 127.0.0.1:8080 &
vsock-to-ip -v 88:443 -i 127.0.0.1:8443 &
vsock-to-ip -v 88:3000 -i 192.168.1.50:3000 &
```

### Use Cases

1. **Database Connection:** Enclave database client connecting to host database
   ```bash
   vsock-to-ip -v 88:5432 -i 127.0.0.1:5432  # PostgreSQL
   ```

2. **HTTP Service:** Enclave accessing host web service
   ```bash
   vsock-to-ip -v 88:8080 -i 127.0.0.1:8080
   ```

3. **External API:** Enclave calling external API
   ```bash
   vsock-to-ip -v 88:443 -i api.example.com:443
   ```

4. **Logging:** Enclave sending logs to aggregator
   ```bash
   vsock-to-ip -v 88:514 -i logs.company.com:514
   ```

### Error Messages

```
Error: Failed to bind listener to vsock: incorrect CID:port
  → Invalid vsock address or CID:PORT format
  → Check vsock address syntax

Error: failed to connect to TCP endpoint
  → Target IP service not available
  → Check IP address and port
  → Verify target service is running

Error: invalid vsock address, should contain one colon [:] sign
  → Vsock address format incorrect
  → Must be CID:PORT format

Error: failed to parse cid as a u32
  → CID not a valid number
  → Must be unsigned 32-bit integer

Error: failed to parse port as a u32
  → Port not a valid number
  → Must be unsigned 32-bit integer
```

### Output

```
Listening on: VsockAddr { cid: 88, port: 4000 }
Proxying to: "127.0.0.1:8080"
Proxying to: "127.0.0.1:8080"
vsock to ip IO copy done, from "88:4000" to "127.0.0.1:8080"
ip to vsock IO copy done, from "127.0.0.1:8080" to "88:4000"
Failed to transfer data: error=...
```

---

## ip-to-vsock

### Synopsis

```bash
ip-to-vsock [OPTIONS]
```

### Description

Forwards connections from a TCP/IP listener to a vsock endpoint. Listens on an IP address and proxies all accepted connections to a specified vsock address.

### Options

```
-i, --ip-addr <IP_ADDR>
    IP address of the listener side

    Format: IP:PORT

    Where:
        IP   = IPv4 or IPv6 address to bind
        PORT = TCP port number to listen on

    Examples:
        0.0.0.0:3000        All interfaces, port 3000
        127.0.0.1:8080      Localhost only, port 8080
        192.168.1.10:9000   Specific interface, port 9000
        [::]:8080           All IPv6 interfaces
        [::1]:3000          IPv6 localhost

    Notes:
        - Use 0.0.0.0 to accept from all IPv4 interfaces
        - Use 127.0.0.1 for localhost only (more secure)
        - Use specific IP for single interface
        - Port must not be in use

-v, --vsock-addr <VSOCK_ADDR>
    Vsock address of the upstream side

    Format: CID:PORT

    Where:
        CID  = Target enclave's Context ID
        PORT = Target vsock port number

    Examples:
        88:5000     Connect to CID 88, port 5000
        3:4000      Connect to CID 3, port 4000
        16:8080     Connect to CID 16, port 8080

    Notes:
        - CID must be reachable (enclave must be running)
        - Target vsock port must be listening
        - Connection established per inbound connection

-h, --help
    Print help information and exit

-V, --version
    Print version information and exit
```

### Examples

#### Basic Usage

```bash
# Listen on all interfaces port 3000, forward to vsock 88:5000
ip-to-vsock --ip-addr 0.0.0.0:3000 --vsock-addr 88:5000
```

#### Short Form

```bash
ip-to-vsock -i 0.0.0.0:3000 -v 88:5000
```

#### Localhost Only

```bash
# More secure - only accept localhost connections
ip-to-vsock -i 127.0.0.1:8080 -v 3:4000
```

#### Specific Interface

```bash
# Bind to specific network interface
ip-to-vsock -i 192.168.1.10:9000 -v 88:9000
```

#### IPv6

```bash
# Listen on IPv6
ip-to-vsock -i [::]:8080 -v 88:8080
ip-to-vsock -i [::1]:3000 -v 88:3000  # Localhost only
```

#### Multiple Services

```bash
# Proxy multiple ports to different enclave services
ip-to-vsock -i 0.0.0.0:80 -v 88:80 &
ip-to-vsock -i 0.0.0.0:443 -v 88:443 &
ip-to-vsock -i 127.0.0.1:9000 -v 88:9000 &
```

### Use Cases

1. **Web Service:** Host accessing enclave web application
   ```bash
   ip-to-vsock -i 0.0.0.0:8080 -v 88:8080
   ```

2. **API Gateway:** External clients accessing enclave API
   ```bash
   ip-to-vsock -i 0.0.0.0:443 -v 88:443
   ```

3. **Database Proxy:** Host database client to enclave database
   ```bash
   ip-to-vsock -i 127.0.0.1:5432 -v 88:5432
   ```

4. **Internal Service:** Service mesh communication
   ```bash
   ip-to-vsock -i 192.168.1.10:9000 -v 88:9000
   ```

### Error Messages

```
Error: Failed to bind listener: malformed listening address:port
  → Invalid IP address or port format
  → Check IP:PORT syntax

Error: failed to connect to vsock endpoint
  → Target vsock not available
  → Check enclave is running
  → Verify vsock CID and port

Error: Address already in use
  → Port already bound by another process
  → Use different port or stop conflicting service

Error: Permission denied
  → Insufficient privileges to bind port
  → Ports < 1024 require root/admin
  → Use sudo or higher port number

Error: invalid vsock address, should contain one colon [:] sign
  → Vsock address format incorrect
  → Must be CID:PORT format
```

### Output

```
Listening on: "0.0.0.0:3000"
Proxying to: VsockAddr { cid: 88, port: 5000 }
Proxying to: VsockAddr { cid: 88, port: 5000 }
ip to vsock IO copy done, from "192.168.1.5:52314" to VsockAddr { cid: 88, port: 5000 }
vsock to ip IO copy done, from VsockAddr { cid: 88, port: 5000 } to "192.168.1.5:52314"
Failed to transfer data: error=...
```

---

## vsock-to-ip-transparent

### Synopsis

```bash
vsock-to-ip-transparent [OPTIONS]
```

### Description

Transparent proxy from vsock to IP. Reads the original destination address from the vsock stream and connects to it. The destination is sent by the paired `ip-to-vsock-transparent` proxy.

### Options

```
-v, --vsock-addr <VSOCK_ADDR>
    Vsock address of the listener side

    Format: CID:PORT

    Where:
        CID  = Context ID to listen on
        PORT = Port number to listen on

    Examples:
        3:1200      Listen on vsock CID 3, port 1200
        88:8080     Listen on vsock CID 88, port 8080
        16:1200     Listen on vsock CID 16, port 1200

    Notes:
        - Usually listens on host CID (commonly 3)
        - Port should match ip-to-vsock-transparent configuration
        - Original destination read from vsock stream

-h, --help
    Print help information and exit

-V, --version
    Print version information and exit
```

### Protocol Details

The proxy expects the original destination to be sent at the start of the vsock stream:

#### IPv4 Format (7 bytes)
```
Byte 0:     4 (u8)              IP version indicator
Bytes 1-4:  IP address (u32_le) IPv4 address in little-endian
Bytes 5-6:  Port (u16_le)       Port number in little-endian
```

#### IPv6 Format (19 bytes)
```
Byte 0:      6 (u8)               IP version indicator
Bytes 1-16:  IP address (u128_le) IPv6 address in little-endian
Bytes 17-18: Port (u16_le)        Port number in little-endian
```

### Examples

#### Basic Usage

```bash
# Listen on vsock 3:1200 for transparent connections
vsock-to-ip-transparent --vsock-addr 3:1200
```

#### Short Form

```bash
vsock-to-ip-transparent -v 3:1200
```

#### Different CID

```bash
vsock-to-ip-transparent -v 88:8080
```

#### Paired with ip-to-vsock-transparent

```bash
# On host (run this first):
vsock-to-ip-transparent -v 3:1200

# In enclave (setup iptables and run proxy):
sudo iptables -t nat -A OUTPUT -p tcp --dport 80 -j REDIRECT --to-ports 1200
ip-to-vsock-transparent -i 127.0.0.1:1200 -v 3:1200

# Now enclave can access any IP:
# curl http://example.com (transparently routed)
```

### Use Cases

1. **Transparent Internet Access:** Enclave unrestricted internet access
   ```bash
   vsock-to-ip-transparent -v 3:1200
   ```

2. **Dynamic Routing:** Destination determined at runtime in enclave
   ```bash
   vsock-to-ip-transparent -v 88:8080
   ```

3. **Multi-Destination Proxy:** Single proxy for multiple destinations
   ```bash
   vsock-to-ip-transparent -v 3:1200
   ```

### Error Messages

```
Error: Failed to bind listener to vsock: incorrect CID:port
  → Invalid vsock address or CID:PORT format
  → Check vsock address syntax

Error: Can't retrieve original_dst from vsock stream
  → Malformed destination bytes received
  → Check paired proxy is ip-to-vsock-transparent
  → Verify protocol compatibility

Error: failed to connect to TCP endpoint
  → Original destination not reachable
  → Check network connectivity
  → Verify destination IP/port

Error: could not fetch inbound address from vsock stream
  → Connection closed before reading address
  → Check paired proxy implementation
```

### Output

```
Listening on: VsockAddr { cid: 3, port: 1200 }
Proxying to: 93.184.216.34:80
vsock to ip IO copy done, from "3:1200" to 93.184.216.34:80
ip to vsock IO copy done, from 93.184.216.34:80 to "3:1200"
Proxying to: 192.168.1.50:443
vsock to ip IO copy done, from "3:1200" to 192.168.1.50:443
ip to vsock IO copy done, from 192.168.1.50:443 to "3:1200"
```

### Security Notes

- Receives destination from vsock stream (trusted source)
- No validation of destination addresses
- Can connect to any reachable IP
- Use firewall rules to restrict outbound connections if needed

---

## ip-to-vsock-transparent

### Synopsis

```bash
ip-to-vsock-transparent [OPTIONS]
```

### Description

Transparent proxy from IP to vsock. Retrieves the original destination using `SO_ORIGINAL_DST` (Linux only) and forwards it to vsock. Must be used with iptables REDIRECT rules.

### Platform Requirements

- **Linux only** (uses `SO_ORIGINAL_DST` socket option)
- iptables with NAT support
- Kernel with Netfilter support

### Options

```
-i, --ip-addr <IP_ADDR>
    IP address of the listener side

    Format: IP:PORT

    Where:
        IP   = IP address to bind (usually 127.0.0.1)
        PORT = Port for iptables to redirect to

    Examples:
        127.0.0.1:1200      Localhost, port 1200
        0.0.0.0:1200        All interfaces, port 1200
        192.168.1.10:1200   Specific interface, port 1200

    Notes:
        - Must match iptables REDIRECT target port
        - Use 127.0.0.1 for enclave-local redirection
        - Port should be unused and non-privileged (>1024)

-v, --vsock-addr <VSOCK_ADDR>
    Vsock address of the upstream side

    Format: CID:PORT

    Where:
        CID  = Target enclave CID (usually host CID like 3)
        PORT = Target vsock port

    Examples:
        3:1200      Connect to host CID 3, port 1200
        88:8080     Connect to enclave CID 88, port 8080
        16:1200     Connect to enclave CID 16, port 1200

    Notes:
        - Usually the other side of transparent proxy pair
        - Port should match vsock-to-ip-transparent listener
        - CID must be reachable

-h, --help
    Print help information and exit

-V, --version
    Print version information and exit
```

### Protocol Details

The proxy sends the original destination at the start of the vsock stream:

#### IPv4 Format (7 bytes)
```
Byte 0:     4 (u8)              IP version indicator
Bytes 1-4:  IP address (u32_le) IPv4 address in little-endian
Bytes 5-6:  Port (u16_le)       Port number in little-endian
```

#### IPv6 Format (19 bytes)
```
Byte 0:      6 (u8)               IP version indicator
Bytes 1-16:  IP address (u128_le) IPv6 address in little-endian
Bytes 17-18: Port (u16_le)        Port number in little-endian
```

### iptables Setup

#### Basic Redirect

```bash
# Redirect all TCP traffic to proxy port
sudo iptables -t nat -A OUTPUT -p tcp -j REDIRECT --to-ports 1200
```

#### Port-Specific Redirect

```bash
# Redirect only HTTP traffic
sudo iptables -t nat -A OUTPUT -p tcp --dport 80 -j REDIRECT --to-ports 1200

# Redirect HTTPS traffic
sudo iptables -t nat -A OUTPUT -p tcp --dport 443 -j REDIRECT --to-ports 1200

# Redirect multiple ports
sudo iptables -t nat -A OUTPUT -p tcp -m multiport --dports 80,443 -j REDIRECT --to-ports 1200
```

#### Source-Based Redirect

```bash
# Redirect from specific subnet
sudo iptables -t nat -A OUTPUT -p tcp -s 192.168.1.0/24 -j REDIRECT --to-ports 1200

# Redirect from specific IP
sudo iptables -t nat -A OUTPUT -p tcp -s 192.168.1.100 -j REDIRECT --to-ports 1200
```

#### Destination-Based Redirect

```bash
# Redirect to specific destination
sudo iptables -t nat -A OUTPUT -p tcp -d 93.184.216.34 -j REDIRECT --to-ports 1200

# Redirect to destination subnet
sudo iptables -t nat -A OUTPUT -p tcp -d 10.0.0.0/8 -j REDIRECT --to-ports 1200
```

#### Application-Based Redirect

```bash
# Redirect from specific user
sudo iptables -t nat -A OUTPUT -p tcp -m owner --uid-owner 1000 -j REDIRECT --to-ports 1200

# Redirect from specific group
sudo iptables -t nat -A OUTPUT -p tcp -m owner --gid-owner 1000 -j REDIRECT --to-ports 1200
```

#### PREROUTING for Incoming Traffic

```bash
# Redirect incoming traffic (gateway/router scenario)
sudo iptables -t nat -A PREROUTING -p tcp --dport 80 -j REDIRECT --to-ports 1200
```

#### Remove iptables Rules

```bash
# List rules with line numbers
sudo iptables -t nat -L OUTPUT --line-numbers

# Delete specific rule by number
sudo iptables -t nat -D OUTPUT 1

# Delete by specification
sudo iptables -t nat -D OUTPUT -p tcp --dport 80 -j REDIRECT --to-ports 1200

# Flush all NAT OUTPUT rules (careful!)
sudo iptables -t nat -F OUTPUT
```

### Examples

#### Basic Usage

```bash
# Setup iptables redirect
sudo iptables -t nat -A OUTPUT -p tcp --dport 80 -j REDIRECT --to-ports 1200

# Run proxy
ip-to-vsock-transparent --ip-addr 127.0.0.1:1200 --vsock-addr 3:1200
```

#### Short Form

```bash
sudo iptables -t nat -A OUTPUT -p tcp -j REDIRECT --to-ports 1200
ip-to-vsock-transparent -i 127.0.0.1:1200 -v 3:1200
```

#### HTTPS Interception

```bash
sudo iptables -t nat -A OUTPUT -p tcp --dport 443 -j REDIRECT --to-ports 1200
ip-to-vsock-transparent -i 127.0.0.1:1200 -v 3:1200
```

#### Multi-Port Setup

```bash
sudo iptables -t nat -A OUTPUT -p tcp -m multiport --dports 80,443,8080 -j REDIRECT --to-ports 1200
ip-to-vsock-transparent -i 127.0.0.1:1200 -v 3:1200
```

#### Complete Transparent Proxy Chain

```bash
# In enclave:
# 1. Setup iptables
sudo iptables -t nat -A OUTPUT -p tcp --dport 80 -j REDIRECT --to-ports 1200

# 2. Run ip-to-vsock-transparent
ip-to-vsock-transparent -i 127.0.0.1:1200 -v 3:1200

# On host:
# 3. Run vsock-to-ip-transparent
vsock-to-ip-transparent -v 3:1200

# Now traffic transparently flows:
# Enclave app → iptables → ip-to-vsock-transparent → vsock →
# → vsock-to-ip-transparent → internet
```

### Use Cases

1. **HTTP/HTTPS Interception:** Monitor or modify enclave web traffic
   ```bash
   sudo iptables -t nat -A OUTPUT -p tcp -m multiport --dports 80,443 -j REDIRECT --to-ports 1200
   ip-to-vsock-transparent -i 127.0.0.1:1200 -v 3:1200
   ```

2. **Policy Enforcement:** Control enclave network access
   ```bash
   sudo iptables -t nat -A OUTPUT -p tcp -d 10.0.0.0/8 -j REDIRECT --to-ports 1200
   ip-to-vsock-transparent -i 127.0.0.1:1200 -v 3:1200
   ```

3. **Traffic Logging:** Log all enclave outbound connections
   ```bash
   sudo iptables -t nat -A OUTPUT -p tcp -j REDIRECT --to-ports 1200
   ip-to-vsock-transparent -i 127.0.0.1:1200 -v 3:1200
   ```

4. **Content Filtering:** Filter enclave internet access
   ```bash
   sudo iptables -t nat -A OUTPUT -p tcp -j REDIRECT --to-ports 1200
   ip-to-vsock-transparent -i 127.0.0.1:1200 -v 3:1200
   ```

### Error Messages

```
Error: Failed to bind listener: malformed listening address:port
  → Invalid IP:PORT format
  → Check listener address

Error: Failed to retrieve original destination from TCP stream
  → No iptables redirect rule
  → Rule not matching traffic
  → Non-Linux system

Error: failed to connect to vsock endpoint
  → Vsock target not reachable
  → Check vsock-to-ip-transparent is running
  → Verify vsock CID and port

Error: Non Linux system, no support for SO_ORIGINAL_DST
  → Running on non-Linux platform
  → Transparent proxies require Linux
```

### Output

```
Listening on: "127.0.0.1:1200"
Proxying to: VsockAddr { cid: 3, port: 1200 }
Original destination: 93.184.216.34:80
Proxying to: VsockAddr { cid: 3, port: 1200 }
ip to vsock IO copy done, from "127.0.0.1:45678" to VsockAddr { cid: 3, port: 1200 }, with original_dst=93.184.216.34:80 from inbound TCP stream
vsock to ip IO copy done, from VsockAddr { cid: 3, port: 1200 } to "127.0.0.1:45678", with original_dst=93.184.216.34:80 from inbound TCP stream
```

### Troubleshooting

```bash
# Verify iptables rule exists
sudo iptables -t nat -L OUTPUT -n -v

# Check rule is matching traffic
sudo iptables -t nat -L OUTPUT -n -v | grep 1200

# Test connection (should show redirect)
curl -v http://example.com

# Monitor connections
sudo netstat -tnp | grep 1200

# Check SO_ORIGINAL_DST support
sudo sysctl net.netfilter.nf_conntrack_max
```

---

## transparent-port-to-vsock

### Synopsis

```bash
transparent-port-to-vsock [OPTIONS]
```

### Description

Transparent proxy that forwards to vsock using the original destination port. Retrieves the original destination using `SO_ORIGINAL_DST` (Linux only), extracts the port, and connects to vsock at specified CID with that port.

### Platform Requirements

- **Linux only** (uses `SO_ORIGINAL_DST` socket option)
- iptables with NAT support
- Kernel with Netfilter support

### Options

```
-i, --ip-addr <IP_ADDR>
    IP address of the listener side

    Format: IP:PORT

    Where:
        IP   = IP address to bind (usually 127.0.0.1)
        PORT = Port for iptables to redirect to

    Examples:
        127.0.0.1:1200      Localhost, port 1200
        0.0.0.0:1200        All interfaces, port 1200
        192.168.1.10:1200   Specific interface, port 1200

    Notes:
        - Must match iptables REDIRECT target port
        - All traffic redirected to this single port
        - Original port used for vsock destination

-v, --vsock <CID>
    CID from vsock address of the upstream side

    Format: CID (unsigned 32-bit integer)

    Where:
        CID = Context ID of target enclave

    Examples:
        88      Connect to enclave CID 88
        3       Connect to host CID 3
        16      Connect to enclave CID 16

    Notes:
        - Port determined from SO_ORIGINAL_DST
        - Final vsock address: CID:original_port
        - CID must be reachable

-h, --help
    Print help information and exit

-V, --version
    Print version information and exit
```

### How It Works

```
1. Accept connection on listener port (e.g., 1200)
2. Retrieve original destination using SO_ORIGINAL_DST
   Example: 93.184.216.34:80
3. Extract original port: 80
4. Connect to vsock: CID:80
   Example: VsockAddr { cid: 88, port: 80 }
5. Forward bidirectional traffic
```

### iptables Setup

#### Basic Redirect

```bash
# Redirect all TCP traffic to proxy port
sudo iptables -t nat -A OUTPUT -p tcp -j REDIRECT --to-ports 1200
```

#### Multi-Port Redirect

```bash
# Redirect multiple service ports
sudo iptables -t nat -A OUTPUT -p tcp -m multiport --dports 80,443,8080 -j REDIRECT --to-ports 1200
```

#### Port Range Redirect

```bash
# Redirect port range
sudo iptables -t nat -A OUTPUT -p tcp --dport 8000:9000 -j REDIRECT --to-ports 1200
```

#### Service-Specific Redirect

```bash
# HTTP only
sudo iptables -t nat -A OUTPUT -p tcp --dport 80 -j REDIRECT --to-ports 1200

# HTTPS only
sudo iptables -t nat -A OUTPUT -p tcp --dport 443 -j REDIRECT --to-ports 1200

# Custom ports
sudo iptables -t nat -A OUTPUT -p tcp --dport 8080 -j REDIRECT --to-ports 1200
```

#### PREROUTING for Incoming Traffic

```bash
# Intercept incoming traffic (gateway scenario)
sudo iptables -t nat -A PREROUTING -p tcp --dport 80 -j REDIRECT --to-ports 1200
```

### Examples

#### Basic Usage

```bash
# Setup iptables
sudo iptables -t nat -A OUTPUT -p tcp -j REDIRECT --to-ports 1200

# Run proxy to enclave CID 88
transparent-port-to-vsock --ip-addr 127.0.0.1:1200 --vsock 88
```

#### Short Form

```bash
sudo iptables -t nat -A OUTPUT -p tcp -j REDIRECT --to-ports 1200
transparent-port-to-vsock -i 127.0.0.1:1200 -v 88
```

#### Multi-Service Port Forwarding

```bash
# Enclave runs:
# - Frontend: vsock 88:80
# - Backend API: vsock 88:8080
# - Admin: vsock 88:9000

# Setup iptables for specific ports
sudo iptables -t nat -A OUTPUT -p tcp -m multiport --dports 80,8080,9000 -j REDIRECT --to-ports 1200

# Run proxy - port determines service
transparent-port-to-vsock -i 127.0.0.1:1200 -v 88

# Traffic routing:
# curl http://example.com:80   → vsock 88:80
# curl http://api.example.com:8080 → vsock 88:8080
# curl http://admin.example.com:9000 → vsock 88:9000
```

#### Gateway Mode

```bash
# Forward incoming traffic to enclave
sudo iptables -t nat -A PREROUTING -p tcp --dport 80 -j REDIRECT --to-ports 1200
transparent-port-to-vsock -i 0.0.0.0:1200 -v 88
```

#### Port Range Forwarding

```bash
# Forward range of ports
sudo iptables -t nat -A OUTPUT -p tcp --dport 8000:8999 -j REDIRECT --to-ports 1200
transparent-port-to-vsock -i 127.0.0.1:1200 -v 88

# Port 8000 → vsock 88:8000
# Port 8001 → vsock 88:8001
# ... etc
```

### Use Cases

1. **Microservices Architecture:** Route to enclave services by port
   ```bash
   sudo iptables -t nat -A OUTPUT -p tcp -m multiport --dports 80,443,8080,9000 -j REDIRECT --to-ports 1200
   transparent-port-to-vsock -i 127.0.0.1:1200 -v 88
   ```

2. **Service Discovery:** Port-based service identification
   ```bash
   sudo iptables -t nat -A OUTPUT -p tcp --dport 3000:3999 -j REDIRECT --to-ports 1200
   transparent-port-to-vsock -i 127.0.0.1:1200 -v 88
   ```

3. **Load Balancing:** Port-based routing to enclave instances
   ```bash
   sudo iptables -t nat -A PREROUTING -p tcp --dport 80 -j REDIRECT --to-ports 1200
   transparent-port-to-vsock -i 0.0.0.0:1200 -v 88
   ```

4. **Development Environment:** Easy multi-service testing
   ```bash
   sudo iptables -t nat -A OUTPUT -p tcp -j REDIRECT --to-ports 1200
   transparent-port-to-vsock -i 127.0.0.1:1200 -v 88
   ```

### Error Messages

```
Error: Failed to bind listener: malformed listening address:port
  → Invalid IP:PORT format
  → Check listener address syntax

Error: Failed to retrieve original destination from TCP stream
  → No iptables redirect rule
  → Rule not matching traffic
  → Non-Linux system

Error: failed to connect to vsock endpoint
  → Vsock target not reachable
  → Check enclave service is listening on extracted port
  → Verify enclave CID

Error: could not fetch inbound address from TCP stream
  → Connection closed prematurely
  → Network error
```

### Output

```
Listening on: "127.0.0.1:1200"
Proxying to: 88
Original destination: 93.184.216.34:80
Proxying to: VsockAddr { cid: 88, port: 80 }
port to vsock IO copy done, from "127.0.0.1:45678" to VsockAddr { cid: 88, port: 80 }, with original_dst=93.184.216.34:80, ip=93.184.216.34, port=80, from inbound TCP stream
vsock to port IO copy done, from VsockAddr { cid: 88, port: 80 } to "127.0.0.1:45678", with original_dst=93.184.216.34:80, ip=93.184.216.34, port=80, from inbound TCP stream
```

### Architecture Example

```
┌─────────────────────────────────────────────────────────────┐
│                   Multi-Service Setup                       │
└─────────────────────────────────────────────────────────────┘

Host Application             Proxy                    Enclave
    (port 80)                 ↓                    (vsock 88:80)
        │                     │                          │
        ├─ TCP:80 ─────→ iptables REDIRECT               │
        │              to 127.0.0.1:1200                 │
        │                     │                          │
        │             transparent-port-to-vsock          │
        │                     │                          │
        │             Extract port=80                    │
        │                     │                          │
        │             Connect vsock 88:80 ──────→  Service :80
        │                     │                          │
        └─────────────── Bidirectional Traffic ──────────┘

Same for ports 443, 8080, etc. - each routes to corresponding vsock port
```

### Troubleshooting

```bash
# Verify iptables rules
sudo iptables -t nat -L OUTPUT -n -v

# Check packet counts
sudo iptables -t nat -L OUTPUT -n -v | grep 1200

# Test original destination retrieval
# (make a connection and check proxy output)

# List listening vsock ports in enclave
netstat -ln | grep vsock

# Monitor connections
sudo netstat -tnp | grep 1200

# Check specific port forwarding
# Example: test port 80
curl -v http://example.com:80
# Check proxy output for "Original destination: ...:80"
```

---

## Address Formats

### Vsock Address Format

**Syntax:** `CID:PORT`

**Components:**
- **CID (Context ID):** Unsigned 32-bit integer (0 to 4,294,967,295)
- **PORT:** Unsigned 32-bit integer (typically 0 to 65,535)

**Special CID Values:**
```
VMADDR_CID_HYPERVISOR  = 0     Reserved for hypervisor
VMADDR_CID_LOCAL       = 1     Local communication
VMADDR_CID_HOST        = 2     Host system
VMADDR_CID_ANY         = -1U   Bind to any CID
```

**Valid Examples:**
```
88:4000           CID 88, port 4000
3:1200            CID 3, port 1200
16:8080           CID 16, port 8080
2:5000            Host CID, port 5000
```

**Invalid Examples:**
```
88-4000           Wrong separator (use :)
88/4000           Wrong separator (use :)
88:4000:3000      Too many colons
88                Missing port
:4000             Missing CID
localhost:4000    Not IP address format
```

### IP Address Format

**Syntax:** `IP:PORT`

**IPv4 Examples:**
```
127.0.0.1:8080    Localhost
0.0.0.0:3000      All interfaces
192.168.1.10:9000 Specific IP
10.0.0.1:443      Private network
```

**IPv6 Examples:**
```
[::1]:8080                    Localhost
[::]:3000                     All interfaces
[2001:db8::1]:9000            Specific IPv6
[fe80::1%eth0]:8080           Link-local with scope
```

**Invalid Examples:**
```
localhost:8080    Use IP, not hostname
8080              Missing IP
192.168.1.10      Missing port
192.168.1.10:     Missing port number
192.168.1.10:abc  Port not numeric
```

---

## Error Codes

### Exit Codes

```
0   Success - proxy running
1   Error - see stderr for details
```

### Common Errors

| Error Message | Cause | Solution |
|---------------|-------|----------|
| Failed to bind listener to vsock | Invalid CID:PORT or permissions | Check vsock format, verify CID |
| Failed to bind listener | Invalid IP:PORT or port in use | Check IP:PORT format, use different port |
| failed to connect to vsock endpoint | Target vsock not available | Check enclave running, verify CID:PORT |
| failed to connect to TCP endpoint | Target IP not reachable | Check IP service running, verify address |
| Failed to retrieve original destination | No iptables rule or non-Linux | Add iptables REDIRECT rule, use Linux |
| invalid vsock address, should contain one colon | Wrong vsock format | Use CID:PORT format |
| failed to parse cid as a u32 | CID not a number | Use numeric CID |
| failed to parse port as a u32 | Port not a number | Use numeric port |
| Permission denied | Insufficient privileges | Use sudo for ports < 1024 |
| Address already in use | Port bound by another process | Stop other process or use different port |

---

## Environment Variables

PF-Proxy binaries do not use environment variables for configuration. All configuration is done via command-line arguments.

### Tokio Runtime

The Tokio async runtime may respect these environment variables:

```bash
# Number of worker threads (default: number of CPUs)
export TOKIO_WORKER_THREADS=4

# Enable Tokio console debugging
export TOKIO_CONSOLE_ENABLE=1
```

### Rust Logging

If recompiled with logging crate:

```bash
# Set log level
export RUST_LOG=debug      # Most verbose
export RUST_LOG=info       # Informational
export RUST_LOG=warn       # Warnings only
export RUST_LOG=error      # Errors only

# Module-specific logging
export RUST_LOG=pf_proxy=debug,tokio=info
```

---

## Output Format

### Standard Output

All proxies output informational messages to stdout:

#### Startup Messages
```
Listening on: <ADDR>
Proxying to: <ADDR>
```

#### Connection Messages
```
Proxying to: <ADDR>
Original destination: <ADDR>
```

#### Completion Messages
```
vsock to ip IO copy done, from <SRC> to <DST>
ip to vsock IO copy done, from <SRC> to <DST>
port to vsock IO copy done, from <SRC> to <DST>, with original_dst=<ADDR>
vsock to port IO copy done, from <SRC> to <DST>, with original_dst=<ADDR>
```

#### Error Messages
```
Failed to transfer data: error=<ERROR>
```

### Standard Error

Error messages are output to stderr:

```
Error: <ERROR_MESSAGE>
  → <ADDITIONAL_CONTEXT>
  → <SUGGESTION>
```

### Log Format Examples

#### vsock-to-ip
```
Listening on: VsockAddr { cid: 88, port: 4000 }
Proxying to: "127.0.0.1:8080"
Proxying to: "127.0.0.1:8080"
vsock to ip IO copy done, from "88:4000" to "127.0.0.1:8080"
ip to vsock IO copy done, from "127.0.0.1:8080" to "88:4000"
```

#### ip-to-vsock
```
Listening on: "0.0.0.0:3000"
Proxying to: VsockAddr { cid: 88, port: 5000 }
Proxying to: VsockAddr { cid: 88, port: 5000 }
ip to vsock IO copy done, from "192.168.1.5:52314" to VsockAddr { cid: 88, port: 5000 }
vsock to ip IO copy done, from VsockAddr { cid: 88, port: 5000 } to "192.168.1.5:52314"
```

#### vsock-to-ip-transparent
```
Listening on: VsockAddr { cid: 3, port: 1200 }
Proxying to: 93.184.216.34:80
vsock to ip IO copy done, from "3:1200" to 93.184.216.34:80
ip to vsock IO copy done, from 93.184.216.34:80 to "3:1200"
```

#### ip-to-vsock-transparent
```
Listening on: "127.0.0.1:1200"
Proxying to: VsockAddr { cid: 3, port: 1200 }
Original destination: 93.184.216.34:80
Proxying to: VsockAddr { cid: 3, port: 1200 }
ip to vsock IO copy done, from "127.0.0.1:45678" to VsockAddr { cid: 3, port: 1200 }, with original_dst=93.184.216.34:80 from inbound TCP stream
vsock to ip IO copy done, from VsockAddr { cid: 3, port: 1200 } to "127.0.0.1:45678", with original_dst=93.184.216.34:80 from inbound TCP stream
```

#### transparent-port-to-vsock
```
Listening on: "127.0.0.1:1200"
Proxying to: 88
Original destination: 93.184.216.34:80
Proxying to: VsockAddr { cid: 88, port: 80 }
port to vsock IO copy done, from "127.0.0.1:45678" to VsockAddr { cid: 88, port: 80 }, with original_dst=93.184.216.34:80, ip=93.184.216.34, port=80, from inbound TCP stream
vsock to port IO copy done, from VsockAddr { cid: 88, port: 80 } to "127.0.0.1:45678", with original_dst=93.184.216.34:80, ip=93.184.216.34, port=80, from inbound TCP stream
```

---

## Advanced Usage

### Running as Systemd Service

Create systemd service file:

```ini
# /etc/systemd/system/pf-proxy-vsock-to-ip.service
[Unit]
Description=PF-Proxy vsock-to-ip
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/vsock-to-ip -v 88:4000 -i 127.0.0.1:8080
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable pf-proxy-vsock-to-ip
sudo systemctl start pf-proxy-vsock-to-ip
sudo systemctl status pf-proxy-vsock-to-ip
```

### Running in Background

```bash
# Using nohup
nohup vsock-to-ip -v 88:4000 -i 127.0.0.1:8080 > /var/log/pf-proxy.log 2>&1 &

# Using screen
screen -dmS pf-proxy vsock-to-ip -v 88:4000 -i 127.0.0.1:8080

# Using tmux
tmux new-session -d -s pf-proxy 'vsock-to-ip -v 88:4000 -i 127.0.0.1:8080'
```

### Process Management

```bash
# Find proxy process
ps aux | grep vsock-to-ip

# Kill proxy
pkill vsock-to-ip

# Graceful restart
pkill -TERM vsock-to-ip && sleep 1 && vsock-to-ip -v 88:4000 -i 127.0.0.1:8080 &
```

### Monitoring

```bash
# Connection count
netstat -tn | grep -c 88:4000

# Active connections
watch -n1 'netstat -tn | grep 88:4000'

# Process stats
top -p $(pgrep vsock-to-ip)

# Network stats
iftop -i eth0
sar -n DEV 1
```

---

**Version:** 0.8.2

**Last Updated:** 2025-11-07
