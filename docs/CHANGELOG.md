## Changelog: what's already done and implemented, release notes

### Done:

- **0.1.0** - Pipeline secure local channel protocol, for host to enclave (and vise versa, enclave to host) data transferring,
              and for remote commands execution (with execution results output to logs),
              for managing enclave's environment, providing container-like experience.

- **0.2.0** - Reproducible builds framework for building customized enclave images (EIF),
              and provide experience for "just use your app dockerfile and build enclave".

- **0.3.0** - Forward proxies, transparent forward proxies, port forwarding proxies, for TCP (HTTP/HTTPs, etc.) and UDP (DNS, etc.) protocols.
              Providing networking stack for applications in enclave, to forward traffic from apps, from inside the enclave,
              use high-level networking protocols (network/cloud data storage, web, etc.), supported over VSock.

- **0.4.0** - New updated reproducible builds system for Sentient Enclaves Framework for building customized enclave images (EIF).

- **0.5.0** - Reverse proxies, transparent reverse proxies (including transparent port forwarding to vsock),
              to support request forwarding into enclave's apps, for providing services (mostly web and other network protocols),
              hosted inside enclave (in isolated memory region and isolated environment from host system).

- **0.6.0** - Set of reference applications, built with framework - inference server (will include Dobby model),
              fine-tuning server (includes fine-tuning OML library), X agent (chat bot app).

- **0.6.1** - Fix bindgen dynamic bindings compilation issue. Documenting bindgen setup into system with LLVM, CLang and its dev libs.
              Fix rustls panic and OpenSSL UAF vulnerability.

- **0.6.2** - Fix `eif_build` hash for git checkout as apps build dependency in `rbuilds.sh`,
              update `Cargo.lock` for `nixpkgs` of `eif_build` and `eif_extract`, all to fix OpenSSL UAF vulnerability.

- **0.7.0** - web protocol for RA, with VRF proofs for attestation docs, mt-runtime, mass-production of attestation docs,
              hot cache and cold DB (NATS KV storage and event bus) integration for storing attestation docs.
              Providing per file attestation of enclave's file system upon web request to attest exact file or attest files in requested directory or
              directories in enclave's file system.
              Providing the control of files and file system integrity via providing per file hashing.
              These file integrity hashes used to generate proofs (based on VRF, for not to rely on enclave or system entropy)
              and per file attestation docs, that include file proofs, based on file integrity hashes.

- **0.7.1** - Remote Attestation Web Server: Implementation of verifier endpoints (verificators) for next generation remote attestation web server,
              to verify hashes for files (hashes act as a runtime ramdisk FS CoW metadata), verify VRF proofs from file+hash pair,
              verify attestation document signature itself via attestation document certificate's public key,
              perform attestation document certificate signature verification and validity checks (validation) by date range,
              and against CA bundle chain of root and intermediate certificates public keys,
              and perform exhaustive validity checks for certificates signatures
              and by date range validity for each certificate in CA bundle chain (for root certificate and intermediate certificates).

- **0.8.0** - file system monitor, for automagic unconditional unattended mass-production of attestation docs,
              with mt-runtime integration as well. Act as a data provider for attestation server and protocol,
              tracking FS content via `inotify` kernel FS events and providing hashes for granular changes in enclave's run-time ramdisk file system.
              Providing CoW layer above the base enclave file system layer in enclave's run-time for immutably tracking the
              whole file system changes per file and control integrity via providing per file hashing.
              These file integrity hashes used to generate proofs (based on VRF, for not to rely on enclave or system entropy)
              and per file attestation docs, that include file proofs, based on file integrity hashes.

- **0.8.1** - Persistent storage layer for Attestation Web Server (`ra-web-srv`) and integration with enclave's service bus (based on NATS)
              and FS Monitor as FS metadata layer provider.
              Integrate persistent storage (NATS KV bucket) into pipeline of attestation documents generation (through `make_attestation_docs` function),
              respecting application configuration.
              Made generation of attestation documents from Walker and Watcher tasks, walking through KV bucket and watching for KV bucket changes,
              consuming data provided by FS Monitor as FS CoW metadata (hashes, for FS integrity control) layer provider,
              modified Producer task for generating of attestation docs. Made clean chain of responsibility through NATS Orchestrator task.
              Respect application configuration.

- **0.8.2** - New version of Enclaves Framework, which includes NATS Server as enclave's service bus (internal and external) and integration of RA Web-Server and FS-Monitor.
              New version of Enclaves Framework, which includes NATS Server as enclave's service bus,
              for integration of services inside enclave (and outside of enclave, via enclave's network proxy and external NATS servers,
              with support of NATS clusterization for cross-enclave integration), mainly for integration of RA Web-Server and FS-Monitor for now.
              It also includes NATS KV JetStream buckets (and NATS JetStream objetcts storage/buckets) as persistency layer for services,
              RA Web-Server and FS-Monitor at the moment.
              FS-Monitor act as a CoW FS metadata layer (missing part of enclave's ramdisk FS) and data provider for RA Web-Server
              to generate customized attestation documents per file in a granular way, to attest every corner of enclave's initramfs/ramdisk
              in enclave's runtime, and cover any run-time FS changes with enclave's attestation.

              What's Changed:
                * Introduction of NATS Server as enclave's service bus
                * Integration of services inside enclave and outside of enclave (cross-enclave integration) in a SOA manner (or in an actor based model)
                * Integration of RA Web-Server and FS-Monitor, as a CoW FS metadata layer data provider for RA Web-Server
                * NATS KV JetStream buckets (and NATS JetStream objetcts storage/buckets) as persistency layer for services in enclave
                * Customized attestation documents per file in a granular way, to cover whole enclave's initramfs/ramdisk for any run-time FS changes with enclave's attestation

- **0.9.0** - New Enclave's Init System, written in Rust, for services and processes management in the enclave (from inside the enclave and also from host through VSock) and for managing enclave state.
              And covering all crates and Enclaves Framework components with exhaustive comprehensive documentation.

              The Enclave Init System is a minimal, production-ready init system (PID 1) designed to run inside secure enclaves. It provides process supervision, automatic
              service restarts, service dependency management, comprehensive logging, dual-protocol control interfaces (Unix socket and VSOCK), and system-wide process management capabilities.

              Key Characteristics and Features:
                - **Minimal footprint**: Small binary size optimized for enclave environments
                - **Reliable**: Written in Rust with comprehensive error handling
                - **Non-crashing**: All errors are logged but never crash the init system
                - **Service supervision**: Automatic process monitoring and restart policies
                - **Dependency management**: Systemd-style service dependencies with startup ordering
                - **Runtime control**: Manage services without restarting the enclave
                - **Dual protocol support**: Control via Unix socket (local) or VSOCK (remote)
                - **Process management**: List, monitor, and control all system processes
                - **Enable/Disable**: Dynamic service activation control
                - **Persistent logging**: Per-service log files with automatic rotation
                - **Configurable**: YAML-based configuration for all aspects of the system
                - **Flexible**: Configuration file path configurable via CLI and environment
                - **Remote management**: Control enclave services from host via VSOCK

              What's Changed:
                * New Enclave's Init System, written in Rust, for services and processes management in the enclave
                  (from inside the enclave and also from host through VSock) and for managing enclave state.
                * Covering all crates and Enclaves Framework components with exhaustive comprehensive documentation.

- **0.9.1** - recursive directory transfer implementation for Pipeline SLC.
              Old tech debt closed in one of the most earlier components - Pipeline Secure Local Channel implementation:
                - Added recursive directory transfer implementation (through `Pipeline SLC` `VSock` binary protocol)
                  with reporducibility of directory tree structure.
                - Added new updated comprehensive `README.md` and `CLI-REFERENCE.md` with exhaustive documentation and CLI reference guide
                  for `Pipeline Secure Local Channel` implmentation, covering new feature of recursive directory transfer implementation from/to enclave.

              Previously directory transfers been done by Bash script and Pipeline SLC CLI tool:
                https://github.com/sentient-agi/Sentient-Enclaves-Framework/blob/main/.bin/pipeline-dir
                https://github.com/sentient-agi/Sentient-Enclaves-Framework/blob/main/.bin/pipeline-dir.sh

- **0.10.0** - Documentation and papers for:
               * Multi-hop encryption/re-encryption and delegated decryption scheme.
               * Vision document about future changes and applicability of the Enclaves Framework.
               * Features document about core features and advantages of the Enclaves Framework.
               * UMA, Discrete, Coherent memory architectures for CVMs and future Enclaves Engine.

- **0.11.0** - Enclave Engine initial implementation.

- **0.12.0** - Proper error handling and structural logging with tracing for Pipeline-SLC, PF-Proxies, changing configuration format for these components from TOML to YAML.

- **0.13.0** - Dynamic buffers set via configuration for Pipeline-SLC
               (this unbound it from system stack size and increase performance for transferring and caching really huge files)

- **0.14.0** - Mmodular RA Web-Server.
               Proper error handling and structural logging with tracing for RA Web-Server.
               Changing configuration format for RA Web-Server from TOML to YAML.

- **0.15.0** - Enclaves remote debugging and logs streaming via **Enclave's Init System** (aggregated logs redirection to VSock).
               `Initctl` listening on VSock for redirected logs streaming and output it to stdout and/or output it to file on host.

               Added logs aggregation, redirection and streaming for enclave's remote debugging and logging thorugh VSock.
               This will improve logs aggregation, especially in enclave's production mode (without debug console),
               for remote debugging of enclaves and apps in enclaves, for use in monitoring and log aggregation systems,
               to understand exact places where issues/bugs appeared, to reveal and fix them fast.
