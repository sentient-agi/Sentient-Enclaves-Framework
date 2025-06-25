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

- **0.7.0** - web protocol for RA, with VRF proofs for attestation docs, mt-runtime, mass-production of attestation docs,
        hot cache and cold DB (Sled) integration for storing attestation docs.
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
