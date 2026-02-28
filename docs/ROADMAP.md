## Roadmap: what's in progress, in active development and upcoming features, release schedule

### WIP (already in development):

- **0.16.0** - cryptography stack, for buffer level SLC and content encryption.

- **0.17.0** - integration with KMS for key storage, probably TPM module usage for local key storage.

- **0.18.0** - Enclaves engine service with web API (Docker API like),
               for enclaves provisioning (EC2 or other cloud instances configuration, including NUMA, Huge Memory Pages allocation, etc.),
               enclaves building (integration with current reproducible builds system, rbuilds build system), enclaves deployment, monitoring.
               Integration with AWS SQS/MQ for deployment tasks tracking and backend systems. Integration with CI (GitHub Actions mostly).

- **0.19.0** - port of framework to QEMU VM with qCoW and EIF images support for running,
               qCoW images reproducible building, block (disk) devices and PCI (PCI-E NVMe) devices support and its attestation,
               FS monitoring and per file attestation for qCoW images (including whole base image attestation during its reproducible build).

### The following is a subject to change, i.e. the order of releases and version numbers.

### In design stage:

- **0.20.0** - proxy re-encryption and delegated decryption cryptography scheme for enclaves secure mesh data transferring.
               Enclave's VPN and multi-hop data transferring with re-encryption.
