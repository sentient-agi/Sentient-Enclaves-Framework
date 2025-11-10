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

**Response Format**: `text/plain`

**Success Response**:
- **Status**: `202 Accepted`
- **Body**:
  ```
  "Started processing directory"
  ```
  or
  ```
  "Started processing file"
  ```

**Error Responses**:
- **Status**: `404 Not Found`
- **Body**:
  ```
  Path not found: <error_details>
  ```

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

**Response Format**: `application/json`

**Success Response (when file processing is complete)**:
- **Status**: `200 OK`
- **Body**:
  ```json
  {
    "file_path": "/app/data/file.txt",
    "sha3_hash": "a1b2c3d4e5f6789...",
    "status": "Ready"
  }
  ```

**Success Response (when file is being processed)**:
- **Status**: `102 Processing`
- **Body**:
  ```json
  {
    "file_path": "/app/data/file.txt",
    "status": "Processing"
  }
  ```

**Error Response (when file is not found)**:
- **Status**: `404 Not Found`
- **Body**:
  ```json
  {
    "file_path": "/app/data/file.txt",
    "status": "Not found"
  }
  ```

**Error Response (when path parameter is missing)**:
- **Status**: `400 Bad Request`
- **Body**:
  ```
  'Path' parameter is missing. Set the requested 'path' first.
  ```

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

**Response Format**: `application/json` or `text/plain`

**Success Response**:
- **Status**: `200 OK`
- **Body**:
  ```json
  {
    "file_path": "/app/data/file.txt",
    "sha3_hash": "a1b2c3d4e5f6789abcdef0123456789..."
  }
  ```

**Processing Response**:
- **Status**: `202 Accepted`
- **Body**: `Processing`

**Error Responses**:
- **Status**: `404 Not Found`
- **Body**: `Not found`

- **Status**: `400 Bad Request`
- **Body**: `'Path' parameter is missing. Set the requested 'path' first.`

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

**Response Format**: Newline-separated JSON objects

**Success Response**:
- **Status**: `200 OK`
- **Body**:
  ```json
  {
    "file_path": "/app/data/file1.txt",
    "sha3_hash": "a1b2c3d4e5f6..."
  }
  {
    "file_path": "/app/data/file2.txt",
    "sha3_hash": "d4e5f6789abc..."
  }
  ```

**Error Response**:
- **Status**: `400 Bad Request`
- **Body**: `'Path' parameter is missing. Set the requested 'path' first.`

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

**Response Format**: `application/json` or `text/plain`

**Success Response**:
- **Status**: `200 OK`
- **Body**:
  ```json
  {
    "file_path": "/app/data/file.txt",
    "sha3_hash": "a1b2c3d4e5f6...",
    "vrf_proof": "0123abcdef456789...",
    "vrf_cipher_suite": "SECP256R1_SHA256_TAI"
  }
  ```

**Processing Response**:
- **Status**: `202 Accepted`
- **Body**: `Processing`

**Error Responses**:
- **Status**: `404 Not Found`
- **Body**: `Not found`

- **Status**: `400 Bad Request`
- **Body**: `'Path' parameter is missing. Set the requested 'path' first.`

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
  - `att_doc_user_data`: User data and public key only
  - `json_debug`, `debug`, `debug_pretty_print`: Various debug formats

**Response Format**: `application/json` or `text/plain`

**Success Response (json_hex)**:
- **Status**: `200 OK`
- **Body**:
  ```json
  {
    "file_path": "/app/data/file.txt",
    "sha3_hash": "a1b2c3...",
    "vrf_proof": "0123abcd...",
    "vrf_cipher_suite": "SECP256R1_SHA256_TAI",
    "att_doc": {
      "protected_header": {
        "01: 26"
      },
      "unprotected_header": {...},
      "payload": {
        "module_id": "i-1234567890abcdef0-enc01234567890abc",
        "digest": "SHA384",
        "timestamp": "1234567890000",
        "PCRs": {
          "0: a1b2c3...",
          "1: d4e5f6..."
        },
        "certificate": "308201...",
        "ca_bundle": [...],
        "public_key": "3059301...",
        "user_data": {
          "file_path": "/app/data/file.txt",
          "sha3_hash": "a1b2c3...",
          "vrf_proof": "0123abcd...",
          "vrf_cipher_suite": "SECP256R1_SHA256_TAI"
        },
        "nonce": "0a1b2c3d..."
      },
      "signature": "304502..."
    }
  }
  ```

**Processing Response**:
- **Status**: `202 Accepted`
- **Body**: `Processing`

**Error Responses**:
- **Status**: `404 Not Found`
- **Body**: `Not found`

- **Status**: `400 Bad Request`
- **Body**: `'Path' parameter is missing. Set the requested 'path' first.`

**cURL Example**:
```bash
curl -k "https://127.0.0.1:8443/doc/?path=/app/data/file.txt&view=json_hex"
```

---

### 7. Get Public Keys

**Endpoint**: `/pubkeys/`

**Method**: `GET`

**Description**: Retrieve server's public keys for VRF proof verification and document signing.

**Query Parameters**:
- `view` (optional): Output format
  - `hex` (default): Hex-encoded keys
  - `json`: Same as hex
  - `string`/`text`: String format (PEM)
- `fmt` (optional): Key format
  - `pem` (default): PEM format
  - `der`: DER format

**Response Format**: `application/json` or `text/plain`

**Success Response (hex/json)**:
- **Status**: `200 OK`
- **Body**:
  ```json
  {
    "pubkey4proofs": "3059301306072a8648ce3d020106082a8648ce3d030107034200...",
    "pubkey4docs": "30819b301006072a8648ce3d020106052b810400230381860004..."
  }
  ```

**Success Response (string/text)**:
- **Status**: `200 OK`
- **Body**:
  ```
  {
    "pubkey4proofs": "-----BEGIN PUBLIC KEY-----\nMFkwEwYHKoZIzj0C...\n-----END PUBLIC KEY-----",
    "pubkey4docs": "-----BEGIN PUBLIC KEY-----\nMIGbMBAGByqGSM49...\n-----END PUBLIC KEY-----"
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

**Response Format**: `text/plain`

**Success Response**:
- **Status**: `200 OK`
- **Body**:
  ```
  Actual (run-time) PCR registers of running enclave, retrieved from enclave's common attestation document:
  0: a1b2c3d4e5f6789abcdef0123456789abcdef0123456789abcdef012345678
  1: 1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef
  2: fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210
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
  "sha3_hash": "a1b2c3d4e5f6789abcdef0123456789..."
}
```

**Response Format**: `text/plain`

**Success Response (for valid hash)**:
- **Status**: `200 OK`
- **Body**:
  ```
  File present in FS and hash provided in JSON request is equal to actual file hash. Hash is VALID!
  'file_path' string from JSON request: "/app/data/file.txt"
  'sha3_hash' string from JSON request: "a1b2c3..."
  computed actual 'sha3_hash' string for file: "a1b2c3..."
  ```

**Success Response (for invalid hash)**:
- **Status**: `200 OK`
- **Body**:
  ```
  File present in FS, but hash provided in JSON request is NOT equal to actual file hash - hashes are different! Hash is INVALID!
  'file_path' string from JSON request: "/app/data/file.txt"
  'sha3_hash' string from JSON request: "a1b2c3..."
  computed actual 'sha3_hash' string for file: "d4e5f6..."
  ```

**Error Responses**:
- **Status**: `404 Not Found`
- **Body**: `File path not found: <error_details>`

- **Status**: `400 Bad Request`
- **Body**:
  ```
  'file_path' field in a JSON request is a directory. Should be a file.
  'file_path' string from JSON request: "/app/data"
  'sha3_hash' string from JSON request: "a1b2c3..."
  ```

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
  "user_data": "7b2266696c655f70617468223a222f6170702f646174612f66696c652e747874222c...",
  "public_key": "3059301306072a8648ce3d020106082a8648ce3d03010703420004..."
}
```

**Note**: Both `user_data` and `public_key` must be hex-encoded strings.

**Response Format**: `text/plain`

**Success Response (for valid proof)**:
- **Status**: `200 OK`
- **Body**: `"VRF proof is valid!"`

**Success Response (for invalid proof)**:
- **Status**: `200 OK`
- **Body**: `"VRF proof is not valid!"` or `"VRF proof is not valid! Error: <error_details>"`

**Error Response**:
- **Status**: `400 Bad Request`
- **Body**:
  ```
  Malformed 'user_data' input as a JSON field: <error_details>
  Please use GET 'att_doc_user_data' endpoint to request correct JSON
  ```

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
  "cose_doc_bytes": "d28443a10126a05905..."
}
```

**Note**: `cose_doc_bytes` must be a hex-encoded string of the COSE document.

**Response Format**: `text/plain`

**Success Response (for valid signature)**:
- **Status**: `200 OK`
- **Body**:
  ```
  Attestation document signature verification: "Successful"
  Attestation document signature is VALID!
  Attestation document signature verification against attestation document certificate public key is successful!
  ```

**Success Response (for invalid signature)**:
- **Status**: `200 OK`
- **Body**:
  ```
  Attestation document signature verification: "NOT successful"
  Attestation document signature is INVALID!
  Attestation document signature verification against attestation document certificate public key is NOT successful!
  ```

**Error Response**:
- **Status**: `400 Bad Request`
- **Body**: `Verification failed. An error returned during attestation document signature verification check: <error_details>`

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

**Description**: Verify certificate signature and validity period (not expired, not yet valid).

**Request Body**:
```json
{
  "cose_doc_bytes": "d28443a10126a05905..."
}
```

**Response Format**: `text/plain`

**Success Response (for valid certificate)**:
- **Status**: `200 OK`
- **Body**:
  ```
  Attestation document certificate signature verification result:
    "Attestation document certificate signature verification against its public key is successful, signature is VALID! Certificate information: <cert_details>"

  Attestation document certificate validity check (validation) result:
    "Attestation document certificate validity check (validation) is SUCCESSFUL! Certificate is VALID! Certificate information: <cert_details>"
  ```

**Error Response (for invalid certificate - signature verification failure)**:
- **Status**: `400 Bad Request`
- **Body**:
  ```
  Attestation document certificate signature verification result:
    "Attestation document certificate signature verification against its public key is NOT successful, signature is INVALID! Certificate information: <cert_details>"

  Attestation document certificate validity check (validation) result:
    "Attestation document certificate validity check (validation) is SUCCESSFUL! Certificate is VALID! Certificate information: <cert_details>"
  ```

**Error Response (for invalid certificate - validity period failure)**:
- **Status**: `400 Bad Request`
- **Body**:
  ```
  Attestation document certificate signature verification result:
    "Attestation document certificate signature verification against its public key is successful, signature is VALID! Certificate information: <cert_details>"

  Attestation document certificate validity check (validation) result:
    "Attestation document certificate is not yet valid or expired. <cert_details>"
  ```

**Error Response (for invalid certificate - both failures)**:
- **Status**: `400 Bad Request`
- **Body**:
  ```
  Attestation document certificate signature verification result:
    "Attestation document certificate signature verification against its public key FAILED! Error returned: <openssl_error>\nCertificate information: <cert_details>"

  Attestation document certificate validity check (validation) result:
    "Attestation document certificate validity check (validation) FAILED! Error returned: <validation_error>\nCertificate information: <cert_details>"
  ```

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

**Description**: Verify certificate against root and intermediate certificates in the CA bundle.

**Request Body**:
```json
{
  "cose_doc_bytes": "d28443a10126a05905..."
}
```

**Response Format**: `text/plain`

**Success Response (for valid certificate chain)**:
- **Status**: `200 OK`
- **Body**:
  ```
  Attestation document certificate verification: "Successful"
  Attestation document certificate is VALID!
  Attestation document certificate verification against attestation document certificates bundle (root certificate and intermediate certificates) is successful!
  ```

**Success Response (for invalid certificate chain)**:
- **Status**: `200 OK`
- **Body**:
  ```
  Attestation document certificate verification: "NOT successful"
  Attestation document certificate is INVALID!
  Attestation document certificate verification against attestation document certificates bundle (root certificate and intermediate certificates) is NOT successful!
  Verification context: "OpenSSL error: <error_string>"
  ```

**Error Response (for certificate verification failure - leaf certificate issues)**:
- **Status**: `400 Bad Request`
- **Body**:
  ```
  Attestation document certificate signature verification against its public key is not successful, signature is invalid. Certificate information: <cert_details>
  ```

**Error Response (for certificate verification failure - root certificate issues)**:
- **Status**: `400 Bad Request`
- **Body**:
  ```
  Attestation document root certificate from CA bundle chain signature verification against its public key is not successful, signature is invalid. Certificate information: <cert_details>
  ```

**Error Response (for certificate verification failure - intermediate certificate issues)**:
- **Status**: `400 Bad Request`
- **Body**:
  ```
  Attestation document intermediate certificate from CA bundle chain signature verification against its public key is not successful, signature is invalid. Certificate information: <cert_details>
  ```

**Error Response (for certificate validity period failure)**:
- **Status**: `400 Bad Request`
- **Body**:
  ```
  Attestation document certificate is not yet valid or expired. Error returned: <validation_error>
  ```

**Error Response (for malformed certificate bundle)**:
- **Status**: `400 Bad Request`
- **Body**:
  ```
  Malformed 'cabundle' in attestation document, incorrect 'cose_doc_bytes' input as a JSON field:
  Please use GET 'cose_doc' endpoint to request correct JSON
  ```

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
  "pcrs": "0: a1b2c3d4e5f6789abcdef0123456789abcdef0123456789abcdef012345678\n1: d4e5f6789abc0123456789abcdef0123456789abcdef0123456789abcdef0123\n2: fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210"
}
```

**Response Format**: `text/plain`

**Success Response (for matching PCRs)**:
- **Status**: `200 OK`
- **Body**:
  ```
  PCRs provided in JSON request are equal to actual PCRs retrieved from enclave's attestation document.
  PCR registers of base image and running enclave from base image are equal and are VALID!

  PCRs from JSON request:
    "0: a1b2c3d4e5f6789abcdef0123456789abcdef0123456789abcdef012345678,
     1: d4e5f6789abc0123456789abcdef0123456789abcdef0123456789abcdef0123,
     2: fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210"

  PCRs retrieved from enclave's attestation document:
    "0: a1b2c3d4e5f6789abcdef0123456789abcdef0123456789abcdef012345678,
     1: d4e5f6789abc0123456789abcdef0123456789abcdef0123456789abcdef0123,
     2: fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210"
  ```

**Success Response (for non-matching PCRs)**:
- **Status**: `200 OK`
- **Body**:
  ```
  PCRs provided in JSON request are NOT equal to actual PCRs retrieved from enclave's attestation document.
  PCR registers of base image and running enclave from base image are NOT equal and are INVALID!

  PCRs from JSON request:
    "0: a1b2c3d4e5f6789abcdef0123456789abcdef0123456789abcdef012345678,
     1: d4e5f6789abc0123456789abcdef0123456789abcdef0123456789abcdef0123,
     2: fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210"

  PCRs retrieved from enclave's attestation document:
    "0: 1111111111111111111111111111111111111111111111111111111111111111,
     1: 2222222222222222222222222222222222222222222222222222222222222222,
     2: 3333333333333333333333333333333333333333333333333333333333333333"
  ```

**cURL Example**:
```bash
curl -k -X POST https://127.0.0.1:8443/verify_pcrs/ \
  -H "Content-Type: application/json" \
  -d '{"pcrs": "0: a1b2c3...\n1: d4e5f6...\n2: fedcba..."}'
```

---

### 15. Get NSM Description

**Endpoint**: `/nsm_desc`

**Method**: `GET`

**Description**: Retrieve Nitro Security Module description.

**Response Format**: `text/plain`

**Success Response**:
- **Status**: `200 OK`
- **Body**:
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

**Response Format**: `text/plain` (hex-encoded)

**Success Response**:
- **Status**: `200 OK`
- **Body**: `a1b2c3d4e5f6789abcdef0123456789abcdef...` (hex string of specified length)

**cURL Example**:
```bash
curl -k "https://127.0.0.1:8443/rng_seq?length=256"
```

---

## Common HTTP Status Codes

- `200 OK`: Request successful
- `202 Accepted`: Request accepted, processing asynchronously
- `102 Processing`: File is currently being processed
- `400 Bad Request`: Invalid parameters or malformed request
- `404 Not Found`: Resource not found
- `500 Internal Server Error`: Server-side error

## Notes

- All `curl` examples use `-k` flag to skip certificate verification (for self-signed certs)
- For production, use proper CA-signed certificates
- The server automatically generates keys if not provided in configuration
- NATS persistence is optional but recommended for production use
- All hex-encoded values in responses are lowercase
- Timestamps are Unix timestamps in milliseconds
