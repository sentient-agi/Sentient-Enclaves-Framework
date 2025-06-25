## **Web API:**

Web API protocol provided by attestation web server hosted inside the enclave.

Web API can be requested by CURL (or any other web2 API tool) from host system (or remotely via exposed HTTPS ports by the host system) to host reverse proxy endpoint with requests redirection into enclave, to proxy and web server that are listening inside the enclave.

## **Specification:**

The following is a Web API requests format formal specification with description for every supported endpoint:

### **Attestation documents generator endpoints:**

```rust
// experimental endpoint for API parameters parsing and requests testing
GET /echo/?key=value

// experimental endpoint for path and view (representation) API parameters parsing and requests testing
GET /hello/?path=path&view=((bin | raw) | hex | (fmt | str) | json)

// Request NSM device parameters
GET /nsm_desc

// Request entropy bytes from NSM device.
// Length set how many bytes of entropy will be retrieved.
GET /rng_seq?length=bytes(default: 512 bytes)

// Request directory or file for hashing and generation/re-generation of proofs and attestation docs
POST /generate?json{ path: (file or directory path) }

// Request results readiness and tasks presence (from tasks and results pools accordingly) for generation/re-generation of hashes, proofs and attestation docs via directory path or file exact path as search template
GET /readiness/?path=file_path
// Request result readiness or task presence (from tasks and results pools accordingly) for generation/re-generation of hash, proof and attestation doc for exact file via its file path as exact search template
GET /ready/?path=file_path

// Request generated hashes for directory or file via exact path
GET /hashes/?path=(file or directory path)
// Request generated hashes for exact file via file path
GET /hash/?path=file_path

// Request generated hashes and proofs for directory or file via exact path
GET /proofs/?path=(file or directory path)
// Request generated hashes and proofs for exact file via file path
GET /proof/?path=file_path

// Request generated hashes, proofs and attestation documents for directory or file via exact path
GET /docs/?path=(file or directory path)&view=(bin_hex | json_hex | json_str | json_debug | debug | debug_pretty_print)
// Request generated hashes, proofs and attestation documents for exact file via file path
GET /doc/?path=file_path&view=(bin_hex | json_hex | json_str | json_debug | debug | debug_pretty_print)

// Request public key for VRF proofs generation from file's hashes (and enclave's public key for signing attestation docs - this possibility remained for the future releases as for now attestation docs are signed by AWS platform private key through NSM device and hypervisor and signature of attestation documents verified by public key in AWS certificate and AWS CA bundle certificates chain and AWS CA root certificate)
GET /pubkeys/?view=(hex | (string | text))&fmt=(pem | der)
```

### **Attestation documents and VRF proofs verificator endpoints:**

```rust
// VRF proofs verification endpoint.
// VRF proofs will be retrieved from attestation document or request parameters.
POST /verify_proof/?json{ doc: byte_hex_string || proof: byte_hex_string, pubkey: byte_hex_string, cipher_suite: string }

// Attestation documents signature verification endpoint.
// Public key will be retrieved from certificate in attestation doc.
POST /verify_doc/?json{ doc: byte_hex_string }
POST /verify_doc_sign/?json{ doc: byte_hex_string }

// Endpoint to verify base image static (build time) PCR hashes against Nitro Enclave's runtime PCR computed parameters.
// PCR hashes of EIF static FS image (computed on build time) will be retrieved from received json_string or from received attestation doc and compared against Nitro Enclave's runtime PCR hashes retrieved from standalone attestation document, received from enclave's NSM device upon and during this web request.
POST /verify_base_image/?json{ PCRs: json_string || doc: byte_hex_string }
POST /verify_pcrs/?json{ PCRs: json_string || doc: byte_hex_string }

// Verify certificate signature validity via public keys from certificates chain and root certificate.
// Certificate and CA bundle with certificates chain and root CA certificate will be retrieved from attestation doc.
POST /verify_cert/?json{ doc: byte_hex_string }
POST /verify_cert_valid/?json{ doc: byte_hex_string }
POST /verify_cert_sign/?json{ doc: byte_hex_string }
POST /verify_cert_sign_valid/?json{ doc: byte_hex_string }

// Endpoint to verify certificate signature validity via public keys from certificates chain and root certificate,
// and then verify attestation document signature using public key retrieved from certificate.
// Certificate and CA bundle with certificates chain and root CA certificate will be retrieved from attestation doc.
POST /verify_all/?json{ doc: byte_hex_string }
POST /verify_doc_and_cert_sign/?json{ doc: byte_hex_string }
POST /verify_doc_sign_and_cert_sign/?json{ doc: byte_hex_string }
```
