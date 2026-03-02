## Roadmap: what's in progress, in active development and upcoming features, release schedule

### WIP (already in development):

- **0.18.0** - Enclave's Engine with web API (Docker API like) and YAML based configurations for CVM instances provisioning
               (multi-threaded run-time with web API for provisioning, based on uploading of YAML configurations for CVM instances and provisioning tasks),
               for enclaves provisioning (AWS EC2 or other cloud instances configuration, including NUMA, Huge Memory Pages allocation, etc.),
               enclaves building (integration with current reproducible builds system, rbuilds build system, of Enclaves Framework),
               enclaves deployment, and monitoring via logs aggregation from Enclave's Init System.
               Integration with AWS SQS/MQ and NATS for deployment tasks tracking and backend systems. Integration with CI (GitHub Actions, etc.).

- **0.19.0** - Enclave's Engine support for provisioning of MS Azure and GCP cloud instances (via Cloud Init APIs).

- **0.20.0** - Enclave's Engine support for KVM and QEMU VMM, and porting of framework components to QEMU VM, with qCoW and EIF images support for running,
               qCoW images reproducible building, block (disk) devices and PCI devices (PCI-E NVMe and PCI-E GPU) support and its attestation,
               FS monitoring and per file attestation for qCoW images FS (including whole base image attestation during its reproducible build).

- **0.21.0** - cryptography stack, for buffer level SLC and content encryption.

- **0.22.0** - integration with KMS for key storage, TPM module usage for local key storage, for secure boot, CVM disk encryption and CVM attestation.

### The following is a subject to change, i.e. the order of releases and version numbers.

### In design stage:

- **0.23.0** - Encalve's Engine support for Firecracker VMM and micro VMs, with PCI bus support, block (disk) devices support and attestation.

- **0.24.0** - Proxy re-encryption and delegated decryption cryptography scheme for enclaves secure mesh data transferring.
               Enclave's VPN and multi-hop data transferring with re-encryption.
