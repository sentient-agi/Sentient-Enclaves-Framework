# AWS Confidential Computing: A Comparison with Azure and GCP

## Introduction

Confidential computing is becoming a critical requirement for organizations handling sensitive data in cloud environments. While all major cloud providers offer some form of confidential computing, their approaches differ significantly in terms of technology, capabilities, and regional availability. This article provides a comprehensive comparison of AWS, Azure, and GCP confidential computing offerings, with a focus on CPU TEE and GPU TEE capabilities.

## AWS Confidential Computing: Two Distinct Approaches

AWS offers two separate confidential computing technologies that serve different use cases.

### 1. AMD SEV-SNP EC2 Instances (Real CVMs)

AWS provides true hardware-based confidential VMs using AMD SEV-SNP technology:

| Specification | Details |
|---------------|---------|
| **Instance Types** | M6a, C6a, R6a (AMD EPYC Milan processors) |
| **Regions** | US East (Ohio), Europe (Ireland) only |
| **Max Size** | c6a.16xlarge |
| **Pricing** | +10% hourly premium on instance cost |
| **OS Support** | AL2023, RHEL 9.3, SLES 15 SP4, Ubuntu 23.04+ |

**Features:**
- Instance-specific memory encryption keys
- AMD VLEK attestation (Versioned Loaded Endorsement Key)
- Hardware-based TEE with full VM isolation
- Launch measurement for boot integrity verification

**Limitations:**
- Cannot use Dedicated Hosts
- No hibernation support
- No Nitro Enclaves support on SEV-SNP instances
- No live migration
- Manual stop/restart required for maintenance events

### 2. AWS Nitro Enclaves

Nitro Enclaves provide a different model of isolation:

| Aspect | Details |
|--------|---------|
| **Architecture** | Isolated environment within EC2 host instance |
| **Technology** | Firecracker microVM |
| **Communication** | vsock only (no network, no storage) |
| **Attestation** | PCR-based (similar to TPM) |

**Key Differences from Full CVMs:**
- Not a full-VM TEE like SEV-SNP or Intel TDX
- Runs alongside the parent instance, not as a standalone VM
- Memory is carved out from the parent instance
- Limited to specific use cases: key management, cryptographic operations

**Use Cases:**
- Hardware security module (HSM) operations
- Secure key management with AWS KMS integration
- Certificate authority operations
- Tokenization services

## Azure Confidential Computing

Azure offers the most comprehensive confidential computing portfolio among major cloud providers.

### CPU TEE Technologies

| Technology | Instance Series | Processor | Key Features |
|------------|-----------------|-----------|--------------|
| **AMD SEV-SNP** | DCasv5, DCadsv5, ECasv5, ECadsv5 | AMD EPYC Milan | Full VM encryption, attestation |
| **Intel TDX** | DCesv5, DCedsv5, ECesv5, ECedsv5 | Intel Xeon 4th Gen | Trust Domain isolation |
| **Intel SGX** | DCsv3, DCdsv3 | Intel Xeon 3rd Gen | Enclave-level isolation |

### GPU TEE Support

Azure is one of only two major cloud providers offering GPU TEE:

| Instance | GPU | CPU TEE | GPU Memory | vCPUs | RAM |
|----------|-----|---------|------------|-------|-----|
| **NCCadsH100v5** | NVIDIA H100 NVL | AMD SEV-SNP | 94GB HBM3 | 40 | 320 GiB |

**Key Capabilities:**
- Full CPU-to-GPU encrypted channel
- NVIDIA Confidential Computing mode enabled
- Dual attestation (AMD + NVIDIA)
- Suitable for confidential AI/ML workloads

### Regional Availability

Azure confidential VMs are available in multiple regions worldwide, with broader availability than AWS SEV-SNP offerings.

## GCP Confidential Computing

Google Cloud provides robust confidential VM support with unique features.

### CPU TEE Technologies

| Technology | Machine Types | Features |
|------------|---------------|----------|
| **AMD SEV** | N2D, C2D, C3D | Basic memory encryption |
| **AMD SEV-SNP** | N2D | Full integrity protection + attestation |
| **Intel TDX** | C3-standard | Trust Domain isolation |

### GPU TEE Support

GCP offers confidential GPU instances:

| Machine Type | GPU | CPU TEE | GPU Memory | vCPUs | RAM |
|--------------|-----|---------|------------|-------|-----|
| **a3-highgpu-1g** | NVIDIA H100 SXM | Intel TDX | 80GB HBM3 | 26 | 234 GB |

### Unique Features

- **Live Migration (SEV only)**: GCP supports live migration for AMD SEV VMs, reducing maintenance disruption
- **Confidential GKE Nodes**: Native Kubernetes support for confidential computing
- **Shielded VM integration**: Combined with Secure Boot and vTPM

## Critical Gaps in AWS Offerings

### Technologies NOT Supported by AWS:

| Technology | AWS | Azure | GCP |
|------------|-----|-------|-----|
| Intel TDX | ❌ | ✅ | ✅ |
| Intel SGX | ❌ | ✅ | ❌ |
| GPU TEE | ❌ | ✅ | ✅ |
| Live Migration (CVM) | ❌ | ❌ | ✅ (SEV only) |

### Regional Availability Comparison

| Provider | SEV-SNP Regions | TDX Regions | GPU TEE Regions |
|----------|-----------------|-------------|-----------------|
| **AWS** | 2 (Ohio, Ireland) | N/A | N/A |
| **Azure** | Multiple | Multiple | Multiple |
| **GCP** | Multiple | Multiple | Multiple |

## Comprehensive Comparison Matrix

### CPU TEE Technologies

| Feature | AWS | Azure | GCP |
|---------|-----|-------|-----|
| **AMD SEV** | ❌ | ❌ | ✅ (N2D, C2D, C3D) |
| **AMD SEV-SNP** | ✅ (Limited) | ✅ (8 series) | ✅ (N2D) |
| **Intel TDX** | ❌ | ✅ (4 series) | ✅ (C3) |
| **Intel SGX** | ❌ | ✅ (DCsv3) | ❌ |
| **Region Count** | 2 | 10+ | 10+ |

### GPU TEE Capabilities

| Feature | AWS | Azure | GCP |
|---------|-----|-------|-----|
| **GPU TEE Available** | ❌ | ✅ | ✅ |
| **GPU Model** | N/A | H100 NVL | H100 SXM |
| **GPU Memory** | N/A | 94GB HBM3 | 80GB HBM3 |
| **CPU TEE Pairing** | N/A | AMD SEV-SNP | Intel TDX |
| **Dual Attestation** | N/A | ✅ | ✅ |

### Operational Features

| Feature | AWS | Azure | GCP |
|---------|-----|-------|-----|
| **Live Migration** | ❌ | ❌ | ✅ (SEV only) |
| **Nested Virtualization** | ❌ | Limited | Limited |
| **Kubernetes Integration** | Limited | AKS Confidential | Confidential GKE |
| **Attestation Service** | Self-managed | Azure Attestation | Self-managed |

## GPU TEE Technical Details

### How GPU TEE Works

NVIDIA H100 Confidential Computing requires a coordinated architecture:

```
┌─────────────────────────────────────────────────────────────────┐
│                    GPU TEE ARCHITECTURE                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────┐          ┌─────────────────┐               │
│  │  Confidential   │          │    NVIDIA H100  │               │
│  │       VM        │◄────────►│   (CC-On Mode)  │               │
│  │  (SEV-SNP/TDX)  │ Encrypted│                 │               │
│  └─────────────────┘  PCIe    └─────────────────┘               │
│                                                                 │
│  Requirements:                                                  │
│  ├── CPU TEE (AMD SEV-SNP or Intel TDX) for CVM                 │
│  ├── GPU in CC-On mode (Confidential Computing enabled)         │
│  ├── Encrypted bounce buffers for CPU-GPU data transfer         │
│  ├── Encrypted PCIe communication channel                       │
│  └── Remote attestation for both CPU and GPU                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Why AWS P5 Instances Are NOT GPU TEE

AWS P5 instances include NVIDIA H100 GPUs, but they are NOT configured for confidential computing:

- GPUs are not in CC-On mode
- No CPU TEE (SEV-SNP) integration with GPU workloads
- No encrypted CPU-GPU channel
- No GPU attestation capability

## Attestation Comparison

### AWS SEV-SNP Attestation

- Uses VLEK (Versioned Loaded Endorsement Key)
- AWS-specific key issued by AMD
- Certificate chain: AMD ARK → AMD ASK → VLEK
- Self-managed verification using snpguest utility

### Azure Attestation

- Managed Azure Attestation service available
- Supports MAA (Microsoft Azure Attestation)
- Integration with Azure Key Vault
- Policy-based attestation decisions

### GCP Attestation

- Self-managed verification
- Integration with Confidential Space
- Support for custom attestation workflows

## Use Case Recommendations

### Choose AWS Confidential Computing if:
- Your workloads are CPU-only and don't require GPU TEE
- You're already heavily invested in AWS ecosystem
- US East (Ohio) or Europe (Ireland) regions meet your requirements
- Nitro Enclaves suit your specific security model

### Choose Azure Confidential Computing if:
- You need GPU TEE for confidential AI/ML workloads
- Intel SGX enclave support is required
- Managed attestation service is preferred
- Broader regional availability is needed
- Multiple CPU TEE technology options are desired

### Choose GCP Confidential Computing if:
- Live migration for confidential VMs is important
- Intel TDX with GPU TEE is required
- Confidential GKE nodes are needed
- You prefer the Google Cloud ecosystem

## AWS Strategy Analysis

AWS's confidential computing strategy appears focused on:

1. **Nitro Enclaves**: Primary offering for most use cases, leveraging their proprietary Nitro system
2. **Limited SEV-SNP**: Minimal investment, only 2 regions, no GPU TEE integration
3. **No Intel Technologies**: Complete absence of TDX and SGX support

This approach suggests AWS prioritizes their proprietary Nitro technology over open standards like TDX and comprehensive SEV-SNP support. For organizations requiring:
- GPU TEE capabilities
- Broad regional availability for CVMs
- Intel TDX or SGX support

Azure or GCP are currently the only viable options among major cloud providers.

## Conclusion

AWS's confidential computing offerings lag significantly behind Azure and GCP in terms of technology breadth, GPU TEE support, and regional availability. While Nitro Enclaves provide a unique approach for specific use cases, organizations requiring full-VM confidential computing with GPU acceleration should consider Azure or GCP.

The confidential computing landscape continues to evolve rapidly, and AWS may expand its offerings in the future. However, as of early 2026, Azure and GCP provide more comprehensive solutions for enterprise confidential computing requirements.

---

*Last updated: January 2026*
