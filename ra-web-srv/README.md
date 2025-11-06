# Remote Attestation Web Server for Sentient Enclaves Framework

## Overview

This is a Rust-based HTTPS web server that provides remote attestation services for AWS Nitro Enclaves. The server generates cryptographic attestation documents, VRF (Verifiable Random Function) proofs, and SHA3-512 hashes for files, with optional NATS-based persistence.

## Features

- **File Hashing**: SHA3-512 hash generation for files and directories
- **VRF Proofs**: Cryptographic proofs using Elliptic Curve VRF
- **Attestation Documents**: AWS Nitro Enclave attestation document generation
- **NATS Integration**: Optional persistent storage using NATS JetStream KV
- **Certificate Verification**: Full X.509 certificate chain validation
- **HTTPS Server**: TLS-secured REST API with automatic HTTP-to-HTTPS redirect

## Prerequisites

- Rust 1.91.0 or later
- AWS Nitro Enclave environment (or NSM emulator for testing)
- TLS certificates (cert.pem and skey.pem)
- Configuration file (`.config/ra-web-srv.config.toml`)
- (Optional) NATS server for persistence

## Configuration

Create a configuration file at `./.config/ra-web-srv.config.toml`:

```toml
[ports]
http = 8080
https = 8443

[keys]
# Leave empty for auto-generation
sk4proofs = ""
sk4docs = ""

vrf_cipher_suite = "SECP256R1_SHA256_TAI"

[nats]
nats_persistency_enabled = 1
nats_url = "nats://127.0.0.1:4222"
hash_bucket_name = "fs_hashes"
att_docs_bucket_name = "fs_att_docs"
persistent_client_name = "ra_web_srv"
```

### Configuration Options

- **ports**: HTTP and HTTPS listening ports
- **keys**: Private keys for proofs and documents (hex-encoded, auto-generated if empty)
- **vrf_cipher_suite**: Elliptic curve cipher suite (e.g., SECP256R1_SHA256_TAI)
- **nats**: NATS persistence configuration (optional)

## TLS Certificates

Place your TLS certificates in `./certs/` (or path specified by `CERT_DIR` env var):
- `cert.pem`: Server certificate
- `skey.pem`: Private key

## Running the Server

```bash
# Set certificate directory (optional)
export CERT_DIR="./certs/"

# Run the server
cargo run --release
```

The server will:
1. Initialize the NSM (Nitro Security Module) device
2. Generate or load cryptographic keys
3. Start HTTPS server on configured port
4. Start HTTP redirect server
5. (Optional) Connect to NATS and initialize persistence layer

---

## Web API Documentation

Base URL: `https://127.0.0.1:8443`

### 1. Generate Attestation Documents

**Endpoint**: `/generate`

**Method**: `POST`

**Content-Type**: `application/json`

**Description**: Recursively processes files/directories to generate hashes, VRF proofs, and attestation documents.

**Request Body**:
```json
{
  "path": "/path/to/file/or/directory"
}
```

**Response**:
- Status: `202 Accepted`
- Body: `"Started processing directory"` or `"Started processing file"`

**cURL Example**:
```bash
curl -k -X POST https://127.0.0.1:8443/generate \
  -H "Content-Type: application/json" \
  -d '{"path": "/app/data"}'
```

---

### 2. Check Processing Status

**Endpoint**: `/ready/`

**Method**: `GET`

**Description**: Check if a specific file has been processed.

**Query Parameters**:
- `path` (required): File path

**Response Example**:
```json
{
  "file_path": "/app/data/file.txt",
  "sha3_hash": "a1b2c3...",
  "status": "Ready"
}
```

**Status Codes**:
- `200 OK`: File ready
- `102 Processing`: File being processed
- `404 Not Found`: File not found
- `400 Bad Request`: Missing path parameter

**cURL Example**:
```bash
curl -k "https://127.0.0.1:8443/ready/?path=/app/data/file.txt"
```

---

### 3. Get File Hash

**Endpoint**: `/hash/`

**Method**: `GET`

**Description**: Retrieve SHA3-512 hash for a specific file.

**Query Parameters**:
- `path` (required): File path

**Response Example**:
```json
{
  "file_path": "/app/data/file.txt",
  "sha3_hash": "a1b2c3d4e5f6..."
}
```

**cURL Example**:
```bash
curl -k "https://127.0.0.1:8443/hash/?path=/app/data/file.txt"
```

---

### 4. Get Multiple Hashes

**Endpoint**: `/hashes/`

**Method**: `GET`

**Description**: Retrieve hashes for all files matching a path prefix.

**Query Parameters**:
- `path` (required): Path prefix to filter

**Response Example**:
```json
{
  "file_path": "/app/data/file1.txt",
  "sha3_hash": "a1b2c3..."
}
{
  "file_path": "/app/data/file2.txt",
  "sha3_hash": "d4e5f6..."
}
```

**cURL Example**:
```bash
curl -k "https://127.0.0.1:8443/hashes/?path=/app/data"
```

---

### 5. Get VRF Proof

**Endpoint**: `/proof/`

**Method**: `GET`

**Description**: Retrieve VRF proof for a specific file.

**Query Parameters**:
- `path` (required): File path

**Response Example**:
```json
{
  "file_path": "/app/data/file.txt",
  "sha3_hash": "a1b2c3...",
  "vrf_proof": "0123abcd...",
  "vrf_cipher_suite": "SECP256R1_SHA256_TAI"
}
```

**cURL Example**:
```bash
curl -k "https://127.0.0.1:8443/proof/?path=/app/data/file.txt"
```

---

### 6. Get Attestation Document

**Endpoint**: `/doc/`

**Method**: `GET`

**Description**: Retrieve attestation document for a specific file.

**Query Parameters**:
- `path` (required): File path
- `view` (optional): Format of attestation document
  - `json_hex` (default): JSON with hex-encoded values
  - `hex`: Raw hex-encoded COSE document
  - `json_str`: JSON with string values
  - `pcr`/`pcrs`: PCR registers only

**Response Example**:
```json
{
  "file_path": "/app/data/file.txt",
  "sha3_hash": "a1b2c3...",
  "vrf_proof": "0123abcd...",
  "vrf_cipher_suite": "SECP256R1_SHA256_TAI",
  "att_doc": {
    "protected_header": {...},
    "payload": {...},
    "signature": "..."
  }
}
```

**cURL Example**:
```bash
curl -k "https://127.0.0.1:8443/doc/?path=/app/data/file.txt&view=json_hex"
```

---

### 7. Get Public Keys

**Endpoint**: `/pubkeys/`

**Method**: `GET`

**Description**: Retrieve server's public keys.

**Query Parameters**:
- `view` (optional): Output format (`hex`, `json`, `string`, `text`)
- `fmt` (optional): Key format (`pem`, `der`)

**Response Example**:
```json
{
  "pubkey4proofs": "3059301306072a8648ce3d020106082a8648ce3d...",
  "pubkey4docs": "30819b301006072a8648ce3d020106052b8104..."
}
```

**cURL Example**:
```bash
curl -k "https://127.0.0.1:8443/pubkeys/?view=hex&fmt=pem"
```

---

### 8. Get PCR Registers

**Endpoint**: `/pcrs/`

**Method**: `GET`

**Description**: Retrieve actual PCR registers from running enclave.

**Response Example**:
```
Actual (run-time) PCR registers of running enclave:
0: a1b2c3d4e5f6...
1: 1234567890ab...
...
```

**cURL Example**:
```bash
curl -k "https://127.0.0.1:8443/pcrs/"
```

---

### 9. Verify Hash

**Endpoint**: `/verify_hash/`

**Method**: `POST`

**Content-Type**: `application/json`

**Description**: Verify if a provided hash matches the actual file hash.

**Request Body**:
```json
{
  "file_path": "/app/data/file.txt",
  "sha3_hash": "a1b2c3d4e5f6..."
}
```

**Response**: Text describing validation result

**cURL Example**:
```bash
curl -k -X POST https://127.0.0.1:8443/verify_hash/ \
  -H "Content-Type: application/json" \
  -d '{"file_path": "/app/data/file.txt", "sha3_hash": "a1b2c3..."}'
```

---

### 10. Verify VRF Proof

**Endpoint**: `/verify_proof/`

**Method**: `POST`

**Content-Type**: `application/json`

**Description**: Verify VRF proof using public key.

**Request Body**:
```json
{
  "user_data": "7b2266696c655f70617468223a...",
  "public_key": "3059301306072a8648ce3d..."
}
```

**Response**: `"VRF proof is valid!"` or error message

**cURL Example**:
```bash
curl -k -X POST https://127.0.0.1:8443/verify_proof/ \
  -H "Content-Type: application/json" \
  -d '{"user_data": "7b22...", "public_key": "3059..."}'
```

---

### 11. Verify Attestation Document Signature

**Endpoint**: `/verify_doc/`

**Method**: `POST`

**Content-Type**: `application/json`

**Description**: Verify attestation document signature against certificate public key.

**Request Body**:
```json
{
  "cose_doc_bytes": "d28443a10126a0..."
}
```

**Response**: Text describing verification result

**cURL Example**:
```bash
curl -k -X POST https://127.0.0.1:8443/verify_doc/ \
  -H "Content-Type: application/json" \
  -d '{"cose_doc_bytes": "d28443a10126..."}'
```

---

### 12. Verify Certificate Validity

**Endpoint**: `/verify_cert_valid/`

**Method**: `POST`

**Content-Type**: `application/json`

**Description**: Verify certificate signature and validity period.

**Request Body**:
```json
{
  "cose_doc_bytes": "d28443a10126a0..."
}
```

**Response**: Text describing certificate validation result

**cURL Example**:
```bash
curl -k -X POST https://127.0.0.1:8443/verify_cert_valid/ \
  -H "Content-Type: application/json" \
  -d '{"cose_doc_bytes": "d28443a10126..."}'
```

---

### 13. Verify Certificate Bundle

**Endpoint**: `/verify_cert_bundle/`

**Method**: `POST`

**Content-Type**: `application/json`

**Description**: Verify certificate against root and intermediate certificates.

**Request Body**:
```json
{
  "cose_doc_bytes": "d28443a10126a0..."
}
```

**Response**: Text describing certificate chain verification result

**cURL Example**:
```bash
curl -k -X POST https://127.0.0.1:8443/verify_cert_bundle/ \
  -H "Content-Type: application/json" \
  -d '{"cose_doc_bytes": "d28443a10126..."}'
```

---

### 14. Verify PCRs

**Endpoint**: `/verify_pcrs/`

**Method**: `POST`

**Content-Type**: `application/json`

**Description**: Compare provided PCRs with actual enclave PCRs.

**Request Body**:
```json
{
  "pcrs": "0: a1b2c3...\n1: d4e5f6...\n..."
}
```

**Response**: Text describing PCR comparison result

**cURL Example**:
```bash
curl -k -X POST https://127.0.0.1:8443/verify_pcrs/ \
  -H "Content-Type: application/json" \
  -d '{"pcrs": "0: a1b2c3...\n1: d4e5f6..."}'
```

---

### 15. Get NSM Description

**Endpoint**: `/nsm_desc`

**Method**: `GET`

**Description**: Retrieve Nitro Security Module description.

**Response Example**:
```
NSM description: [ major: 1, minor: 0, patch: 0, module_id: i-1234567890abcdef0-enc01234567890abc, max_pcrs: 32, locked_pcrs: [], digest: SHA384 ]
```

**cURL Example**:
```bash
curl -k "https://127.0.0.1:8443/nsm_desc"
```

---

### 16. Get Random Sequence

**Endpoint**: `/rng_seq`

**Method**: `GET`

**Description**: Generate cryptographically secure random sequence from NSM.

**Query Parameters**:
- `length` (optional): Byte length (default: 512)

**Response**: Hex-encoded random bytes

**cURL Example**:
```bash
curl -k "https://127.0.0.1:8443/rng_seq?length=256"
```

---

## Error Responses

All endpoints may return:
- `400 Bad Request`: Invalid parameters or malformed request
- `404 Not Found`: Resource not found
- `500 Internal Server Error`: Server-side error

## Notes

- All `curl` examples use `-k` flag to skip certificate verification (for self-signed certs)
- For production, use proper CA-signed certificates
- The server automatically generates keys if not provided in configuration
- NATS persistence is optional but recommended for production use
