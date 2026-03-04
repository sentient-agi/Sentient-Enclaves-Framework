# Intel TDX vs AMD SEV-SNP: Attestation Architecture Deep Dive

## Introduction

Remote attestation is a critical component of confidential computing, enabling verification that workloads are running in genuine trusted execution environments (TEEs). Intel TDX and AMD SEV-SNP take fundamentally different approaches to attestation signing: Intel relies on SGX enclaves, while AMD uses dedicated hardware in the AMD Secure Processor. This article provides a comprehensive technical comparison of both architectures, examining their pros, cons, and security implications.

## Architectural Overview

### Intel TDX Attestation Flow (SGX-Dependent)

Intel TDX attestation relies on a multi-stage process involving SGX enclaves running on the host platform:

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                    INTEL TDX ATTESTATION ARCHITECTURE                           │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌─────────────────┐                                                            │
│  │   Trust Domain  │                                                            │
│  │      (TD)       │                                                            │
│  └────────┬────────┘                                                            │
│           │ TD Report (MAC'd)                                                   │
│           ▼                                                                     │
│  ┌─────────────────┐                                                            │
│  │   TDX Module    │                                                            │
│  │  (SEAM Mode)    │                                                            │
│  └────────┬────────┘                                                            │
│           │ EVERIFYREPORT2                                                      │
│           ▼                                                                     │
│  ┌─────────────────┐                                                            │
│  │  TD Quoting     │◄─── SGX Enclave on Host                                    │
│  │  Enclave (TDQE) │                                                            │
│  └────────┬────────┘                                                            │
│           │ ECDSA Sign (Attestation Key)                                        │
│           ▼                                                                     │
│  ┌─────────────────┐                                                            │
│  │   PCE Enclave   │◄─── SGX Enclave on Host                                    │
│  │  (Certify AK)   │                                                            │
│  └────────┬────────┘                                                            │
│           │                                                                     │
│           ▼                                                                     │
│  ┌─────────────────┐                                                            │
│  │   TD Quote      │                                                            │
│  │   (Signed)      │                                                            │
│  └─────────────────┘                                                            │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

**Step-by-step process:**

1. **TD Report Generation**: The Trust Domain requests a TD Report from the TDX Module (running in SEAM mode). This report is MAC'd using a hardware key only accessible to valid SGX enclaves.

2. **Quote Generation via SGX Enclaves**:
   - The TD Quoting Enclave (TDQE), an SGX enclave, receives the TD Report
   - TDQE verifies the report using the `EVERIFYREPORT2` instruction
   - TDQE signs the TD Report with an Attestation Key (AK) using ECDSA

3. **Key Certification Chain**:
   - The Provisioning Certification Enclave (PCE), another SGX enclave, certifies the AK
   - PCE uses the Provisioning Certification Key (PCK) derived from CPU hardware fuses
   - Intel issues PCK Certificates that chain back to Intel's root CA

4. **Quote Verification** can be done via two variants:
   - **Variant 1**: Software-only verification using Quote Verification Library (QVL)
   - **Variant 2**: SGX-protected verification using Quote Verification Enclave (QvE)

### AMD SEV-SNP Attestation Flow (Direct Hardware)

AMD SEV-SNP takes a simpler, hardware-only approach:

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                    AMD SEV-SNP ATTESTATION ARCHITECTURE                         │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌─────────────────┐                                                            │
│  │   SNP Guest VM  │                                                            │
│  │                 │                                                            │
│  └────────┬────────┘                                                            │
│           │ Attestation Request (/dev/sev-guest)                                │
│           ▼                                                                     │
│  ┌─────────────────┐                                                            │
│  │   AMD Secure    │                                                            │
│  │   Processor     │                                                            │
│  │    (AMD-SP)     │                                                            │
│  └────────┬────────┘                                                            │
│           │ Direct Hardware Signing (VCEK/VLEK)                                 │
│           ▼                                                                     │
│  ┌─────────────────┐                                                            │
│  │  Attestation    │                                                            │
│  │    Report       │                                                            │
│  │   (Signed)      │                                                            │
│  └─────────────────┘                                                            │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

**Step-by-step process:**

1. **Report Request**: The SNP Guest VM requests an attestation report from the AMD Secure Processor (AMD-SP) via the `/dev/sev-guest` interface

2. **Direct Hardware Signing**: The AMD-SP generates the report containing guest measurements and signs it directly using either:
   - **VCEK** (Versioned Chip Endorsement Key): Derived from chip-unique secrets + TCB version
   - **VLEK** (Versioned Loaded Endorsement Key): CSP-specific key provisioned by AMD

3. **Certificate Chain**:
   - AMD Root Key (ARK) → AMD Signing Key (ASK) → VCEK/VLEK
   - Certificates retrieved from AMD Key Distribution Service (KDS)

4. **Verification**: Standard certificate chain validation using OpenSSL or similar tools

## Trust Chain Composition Comparison

```
INTEL TDX:
Intel Root CA → PCK Cert → [PCE Enclave] → AK Cert → [TDQE Enclave] → TD Quote
                              ▲                          ▲
                              │                          │
                        SGX Enclave                SGX Enclave
                        (Software)                 (Software)

AMD SEV-SNP:
AMD ARK → AMD ASK → VCEK/VLEK → Attestation Report
                        ▲
                        │
                   Hardware Only
                   (AMD-SP Firmware)
```

## Detailed Pros and Cons

### Intel TDX (SGX-Based Attestation)

| Aspect | Pros | Cons |
|--------|------|------|
| **Architecture** | Leverages mature SGX infrastructure; unified DCAP framework for both SGX and TDX | Adds SGX as a dependency — TDX attestation requires SGX to be enabled on the platform |
| **TCB Size** | QvE provides cryptographically verified results within SGX enclave | Larger TCB: includes TDX Module + TDQE + PCE + SGX microcode + SGX infrastructure |
| **Flexibility** | Two verification variants (software-only or SGX-protected) | More complex deployment; requires SGX SDK, AESM service, EPC memory |
| **Key Management** | Attestation keys generated and managed within enclaves | Key derivation depends on multiple software components (QE, PCE) |
| **Attack Surface** | SGX provides process-level isolation for signing operations | Inherits all SGX vulnerabilities: side-channel attacks, speculative execution |
| **Updates** | TCB recovery process well-documented | Complex TCB recovery: must tear down all enclaves, empty EPC, run `EUPDATESVN` |
| **Verification** | Can verify quotes in SGX-protected environment (QvE) | Verification collateral requires network access to Intel PCS |
| **Audit** | Source code for DCAP components publicly available | Must trust Intel-signed binaries match audited source |

### AMD SEV-SNP (Direct Hardware Attestation)

| Aspect | Pros | Cons |
|--------|------|------|
| **Architecture** | Pure hardware solution — no software enclaves needed for signing | Less flexible; single attestation path |
| **TCB Size** | Minimal TCB: only AMD-SP firmware + CPU microcode | No enclave-based isolation for attestation infrastructure |
| **Simplicity** | Simple deployment; just enable SNP and use `/dev/sev-guest` | No optional enhanced verification (like Intel's QvE) |
| **Key Management** | Keys derived directly from hardware fuses (VCEK) or AMD-provisioned (VLEK) | VCEK exposes chip identity; VLEK requires AMD involvement |
| **Attack Surface** | Smaller attack surface — no SGX side-channel exposure | Relies entirely on AMD-SP security; if compromised, all signing is compromised |
| **Updates** | VCEK automatically versions with TCB updates | No runtime key re-provisioning without platform reset for VCEK |
| **Verification** | Standard OpenSSL certificate verification | Must fetch certificates from AMD KDS (network dependency) |
| **Privacy** | VLEK option hides individual chip identity | VCEK contains chip-unique hardware ID (privacy concern in some scenarios) |

## SGX Vulnerability Inheritance

A critical consideration for Intel TDX is that its attestation infrastructure inherits all SGX attack vectors:

| Attack Class | Examples | Impact on TDX Attestation |
|--------------|----------|---------------------------|
| **Side-Channel** | Prime+Probe, Flush+Reload | Could leak AK or PCK during signing |
| **Speculative Execution** | Foreshadow (L1TF), SGAxe, Plundervolt | Could extract enclave secrets |
| **Microarchitectural** | ZombieLoad, RIDL, MDS | Affects TDQE/PCE enclaves |
| **Memory Safety** | Enclave memory corruption | Could forge quotes if TDQE compromised |

AMD SEV-SNP attestation is immune to these SGX-specific attacks since signing happens in the AMD-SP (a separate ARM Cortex-A5 core) rather than in x86 enclaves.

## TCB Recovery Complexity

### Intel TDX

- Microcode updates require special handling
- To update attestation TCB: empty EPC → run `EUPDATESVN` → restart enclaves
- Attestation reflects TCB level at enclave creation time
- OSPL-based updates require OS software enabling

### AMD SEV-SNP

- VCEK automatically incorporates TCB version (firmware + microcode SVNs)
- Platform reboot updates VCEK version
- Simpler but requires reboot for TCB updates

## Key Derivation Models

| Aspect | Intel TDX | AMD SEV-SNP |
|--------|-----------|-------------|
| **Signing Key** | ECDSA Attestation Key (AK) | VCEK or VLEK |
| **Key Location** | Inside TDQE enclave (RAM/EPC) | Inside AMD-SP (separate core) |
| **Key Derivation** | QE Seal Key → AK derivation | Hardware fuses → CEK → VCEK |
| **Key Certification** | PCE signs AK using PCK | AMD signs VCEK/VLEK cert |
| **Key Versioning** | PCK versioned by TCB | VCEK versioned by TCB; VLEK versioned by CSP agreement |

## VCEK vs VLEK in AMD SEV-SNP

AMD provides two signing key options:

### VCEK (Versioned Chip Endorsement Key)
- Derived from chip-unique secrets stored in hardware fuses
- Unique to each physical processor
- Includes TCB version in derivation
- Certificate fetched from AMD KDS using chip ID + TCB version
- **Privacy concern**: Chip ID is exposed in attestation

### VLEK (Versioned Loaded Endorsement Key)
- Derived from a secret shared between AMD and a Cloud Service Provider
- Identifies the CSP rather than the specific chip
- Encrypted with per-device per-version wrapping key
- Decrypted and stored by AMD-SP at runtime
- **Privacy benefit**: Hides individual chip identity

## Summary Comparison Table

| Feature | Intel TDX (SGX-Based) | AMD SEV-SNP (Hardware) |
|---------|----------------------|------------------------|
| **Attestation Signing** | SGX Quoting Enclave (TDQE) | AMD Secure Processor |
| **Key Storage** | SGX enclave memory (EPC) | Hardware security module |
| **TCB Components** | TDX Module + SGX (QE, PCE, microcode) | AMD-SP firmware + microcode |
| **SGX Dependency** | Required on host | Not applicable |
| **Side-Channel Risk** | Inherits SGX vulnerabilities | Isolated from x86 attacks |
| **Deployment Complexity** | High (SGX SDK, AESM, PCCS) | Low (kernel driver only) |
| **Offline Verification** | Complex (need collateral caching) | Simpler (just cert chain) |
| **Privacy Options** | No chip-specific identity exposure | VCEK exposes; VLEK hides |
| **Verification Options** | Software (QVL) or SGX (QvE) | Software only |

## When to Prefer Each

### Prefer Intel TDX if:
- You need SGX enclave-protected quote verification (QvE)
- Your infrastructure already deploys SGX
- You want unified attestation for both SGX enclaves and TDX VMs
- You require fine-grained enclave isolation alongside VM isolation

### Prefer AMD SEV-SNP if:
- You want minimal attack surface for attestation
- You want to avoid SGX side-channel vulnerabilities
- You need simpler deployment without SGX dependencies
- Privacy concerns make chip-identity exposure problematic (use VLEK)
- You prefer a pure hardware root of trust

## Conclusion

The fundamental trade-off between Intel TDX and AMD SEV-SNP attestation architectures is **complexity vs. attack surface**. Intel's approach offers more flexibility and unified infrastructure but inherits SGX's vulnerabilities and adds complexity. AMD's approach is simpler and more isolated but offers fewer verification options.

For security-critical deployments where minimizing attack surface is paramount, AMD SEV-SNP's hardware-only attestation provides a cleaner trust model. For organizations already invested in SGX infrastructure or requiring the flexibility of enclave-protected verification, Intel TDX's SGX-based attestation may be more appropriate despite its larger TCB.

---

*Last updated: January 2026*
