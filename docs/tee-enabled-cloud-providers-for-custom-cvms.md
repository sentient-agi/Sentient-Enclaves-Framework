# Cloud Providers for Running Custom Confidential VMs with CPU and GPU TEE Support

> A comprehensive guide to cloud operators providing TEE-enabled bare metal infrastructure for deploying your own QEMU/KVM-based Confidential Virtual Machines

---

## Executive Summary

Major cloud providers like Azure, GCP, and AWS offer confidential computing instances, but they provide **pre-built CVMs** rather than infrastructure where customers can run their **own** QEMU/KVM-based Confidential Virtual Machines. This creates vendor lock-in and prevents infrastructure migration.

This article identifies cloud providers that offer TEE-enabled bare metal or dedicated servers where you can deploy your own confidential computing stack with both CPU TEE (AMD SEV-SNP, Intel TDX) and GPU TEE (NVIDIA H100/H200 Confidential Computing) support.

---

## Table of Contents

1. [The Vendor Lock-in Problem](#the-vendor-lock-in-problem)
2. [Cloud Providers Comparison](#cloud-providers-comparison)
3. [Technical Requirements for Custom CVMs](#technical-requirements-for-custom-cvms)
4. [Provider Deep Dives](#provider-deep-dives)
5. [Recommended Providers by Use Case](#recommended-providers-by-use-case)
6. [Important Caveats](#important-caveats)
7. [Conclusion](#conclusion)

---

## The Vendor Lock-in Problem

### Why Major Clouds Don't Offer This

Major cloud providers (Azure, GCP, AWS) provide confidential *instances* but generally don't offer bare metal or infrastructure where customers can run their **own** QEMU/KVM-based CVMs with CPU TEE and GPU TEE support. The reasons include:

- **Commercial Interest**: Vendor lock-in ensures customers remain on their platform
- **Infrastructure Control**: Running customer hypervisors would require exposing low-level hardware access
- **Support Complexity**: Custom CVM deployments introduce variables outside provider control
- **Security Boundaries**: Nested virtualization with TEE creates complex trust relationships

### What You Actually Need

To run your own CVMs, you need:

1. **Bare metal access** or dedicated hardware with BIOS/UEFI control
2. **TEE-enabled CPUs** with proper firmware and BIOS settings
3. **Linux host** with recent kernel (6.8+) and patched QEMU/KVM
4. **Optional**: NVIDIA GPUs with Confidential Computing capability

---

## Cloud Providers Comparison

### Tier 1: Full TEE Stack (CPU + GPU TEE)

| Provider | CPU TEE | GPU TEE | Run Own CVMs | Pricing | Notes |
|----------|---------|---------|--------------|---------|-------|
| **Phala Cloud** | AMD SEV-SNP, Intel TDX | H100/H200 | Yes (Docker-based) | $2.30-3.50/GPU/hr | Only provider with full-stack TEE |
| **OpenMetal** | Intel TDX, AMD SEV | H100 (PCIe passthrough) | Yes (bare metal) | Enterprise pricing | Pre-configured TDX servers |

### Tier 2: CPU TEE Only (Bare Metal)

| Provider | CPU TEE | GPU TEE | Run Own CVMs | Pricing | Notes |
|----------|---------|---------|--------------|---------|-------|
| **OVHcloud Scale** | AMD SEV-SNP (EPYC 9004) | No CC GPUs | Yes | ~$100-500/mo | Scale-a5/a6 with AMD Infinity Guard |
| **Hetzner** | AMD EPYC (potential SEV-SNP) | No | Possible | ~€129/mo | AX162 with EPYC 9454P |
| **Equinix Metal** | Intel SGX, AMD SEV | No | Yes | Enterprise | API-driven bare metal |
| **VPSBG.eu** | AMD SEV-SNP | No | Yes | Budget | Small EU provider |
| **Vultr Bare Metal** | AMD EPYC (potential) | No | Uncertain | ~$120/mo+ | Hardware capable |

### Tier 3: Generic Bare Metal (Manual TEE Setup Required)

| Provider | Hardware | TEE Status | Notes |
|----------|----------|------------|-------|
| **TensorDock** | AMD EPYC + H100 | Unknown | KVM virtualization available |
| **Atlas Cloud** | AMD EPYC + H100/H200/B200 | Unknown | Bare metal GPU focus |
| **CUDO Compute** | Various + GB200/B300 | Unknown | Enterprise bare metal |

---

## Technical Requirements for Custom CVMs

### CPU TEE Requirements

#### AMD SEV-SNP

```
Hardware:
├── AMD EPYC 7xx3 (Milan) or newer
├── EPYC 9xx4 (Genoa) recommended for best performance
└── Motherboard with SEV-SNP BIOS support

BIOS Settings Required:
├── SMEE (Secure Memory Encryption Enable): ON
├── IOMMU: Enabled
├── RMP Coverage: Enabled (All memory or Custom)
├── SEV-SNP: Enabled
└── SEV-ES ASID Space Limit: Non-zero (e.g., 100 for 100 concurrent VMs)

Firmware:
└── Latest SEV firmware from https://developer.amd.com/sev
```

#### Intel TDX

```
Hardware:
├── Intel Xeon 4th Gen (Sapphire Rapids) or newer
├── 5th Gen Xeon recommended for production
└── OEM BIOS with TDX support

BIOS Settings Required:
├── Intel TDX: Enabled
├── TME-MT: Enabled
├── TDX Key Split: Configured (e.g., 1 for up to 64 TDX VMs)
└── SGX DCAP: Enabled (for attestation)

Software:
├── TDX Module: Latest version from Intel
└── Ubuntu 24.04+ or RHEL 9+ with TDX support
```

### Software Stack

```
Host OS Requirements:
├── Linux Kernel: 6.8 or newer (6.11+ recommended)
├── QEMU: 8.0+ with SEV-SNP/TDX patches
│   └── QEMU 10.1 has full TDX and SEV-SNP support
├── KVM: With confidential guest support
└── Libvirt: 9.0+ (optional, for management)

Guest OS Requirements:
├── Ubuntu 23.04+ / 24.04 LTS
├── RHEL 9.3+
├── SUSE 15 SP4+
└── Custom Linux with appropriate kernel patches
```

### GPU TEE Requirements (NVIDIA Confidential Computing)

```
Hardware:
├── NVIDIA H100 (PCIe or SXM)
├── NVIDIA H200 (141GB HBM3e)
└── NVIDIA B200 (coming soon, 192GB HBM3e)

Prerequisites:
├── Must run within CPU TEE (SEV-SNP or TDX)
├── GPU must be set to CC-On mode
├── IOMMU enabled for GPU passthrough
└── vfio-pci driver bound to GPU

Software:
├── NVIDIA driver with CC support
├── SPDM secure channel established
└── NVIDIA attestation service integration (NRAS)

Attestation:
├── Dual attestation required (CPU + GPU)
├── Intel: DCAP or Trust Authority
├── AMD: KDS (Key Distribution System)
└── NVIDIA: Remote Attestation Service
```

---

## Provider Deep Dives

### Phala Cloud

**Overview**: The only cloud provider offering a complete full-stack TEE solution combining Intel TDX (CPU/memory protection) with NVIDIA Confidential Computing (GPU encryption).

**Key Features**:
- Intel TDX + NVIDIA H100/H200 in single deployment
- Dual attestation reports (Intel + NVIDIA)
- Docker-based deployment model
- Private ML SDK for confidential AI

**Pricing**:
- H200: From $3.50/GPU/hr (on-demand), $2.30/GPU/hr (reserved)
- H100: From $3.08/GPU/hr (on-demand), $2.50/GPU/hr (reserved)

**Best For**: Confidential AI inference, private LLM deployment, GPU TEE workloads

**Limitations**: Not bare metal; uses their abstraction layer over TEE hardware

---

### OpenMetal

**Overview**: Bare metal infrastructure provider with pre-configured Intel TDX support and optional H100 GPU passthrough.

**Key Features**:
- True bare metal with full control
- Intel TDX out-of-the-box on v4 servers
- 5th Gen Xeon CPUs with up to 1TB RAM
- H100 GPU via PCIe passthrough
- 10 Gbps VLAN-isolated networking

**Server Configurations**:
- XL V4: Intel TDX, 1TB RAM
- XXL V4: Intel TDX, 1TB RAM + H100 GPU option

**Best For**: Running your own QEMU/KVM stack, custom CVM deployments, blockchain validators

---

### OVHcloud Scale

**Overview**: European hyperscaler offering bare metal servers with AMD EPYC 9004 series processors that include AMD Infinity Guard (SEV-SNP, SME, Shadow Stack).

**Key Features**:
- Scale-a5: 64 cores, up to 1TB DDR5
- Scale-a6: 96 cores, up to 1TB DDR5
- AMD SEV-SNP capable out of the box
- Full BIOS access for TEE configuration

**Pricing**:
- Scale-a5: ~$300/mo
- Scale-a6: ~$500/mo

**Best For**: Cost-effective SEV-SNP deployments, European data sovereignty

**Limitations**: No GPU TEE support; NVIDIA L4 available but not CC-enabled

---

### Hetzner

**Overview**: German hosting provider with AMD EPYC 9004 series servers at competitive prices.

**Key Features**:
- AX162: AMD EPYC 9454P (48 cores, 96 threads)
- 4th Gen EPYC with SEV-SNP capable silicon
- Very competitive pricing (~€129/mo)
- Gen4 NVMe storage

**Caveats**:
- BIOS access may be limited
- SEV-SNP enablement not officially documented
- May require support ticket to enable TEE features

**Best For**: Budget-conscious deployments where you can work with support for BIOS access

---

### Equinix Metal

**Overview**: API-driven bare metal platform with Intel SGX and AMD SEV support.

**Key Features**:
- Programmable infrastructure
- Intel SGX on select configurations
- AMD SEV on EPYC-based servers
- Enterprise-grade networking

**Best For**: DevOps-driven deployments, programmatic infrastructure management

---

### VPSBG.eu

**Overview**: Small Bulgarian provider offering AMD SEV-SNP on virtual dedicated servers.

**Key Features**:
- AMD EPYC CPUs with SEV-SNP
- Can enable SEV-SNP on request
- Privacy-focused (accepts Bitcoin)
- Custom ISO support

**Pricing**: Budget-friendly VDS options

**Best For**: Privacy-conscious users, small-scale TEE experimentation

---

## Recommended Providers by Use Case

### Confidential AI (CPU + GPU TEE)

| Use Case | Recommended Provider | Why |
|----------|---------------------|-----|
| LLM Inference | Phala Cloud | Full-stack TEE, turnkey GPU CC |
| AI Training | OpenMetal + H100 | Bare metal control, PCIe passthrough |
| Private RAG | Phala Cloud | Integrated attestation |

### Custom QEMU/KVM CVMs (CPU TEE Only)

| Use Case | Recommended Provider | Why |
|----------|---------------------|-----|
| Production SEV-SNP | OVHcloud Scale | Best documented, BIOS access |
| Intel TDX | OpenMetal | Pre-configured, enterprise support |
| Budget/Testing | Hetzner AX162 | Capable hardware, low cost |

### Blockchain/Web3

| Use Case | Recommended Provider | Why |
|----------|---------------------|-----|
| Validator Nodes | OpenMetal | TDX + bare metal isolation |
| Oracle Nodes | OVHcloud + SEV-SNP | Cost-effective, attestation support |
| Confidential DeFi | Phala Cloud | GPU TEE for complex computations |

---

## Important Caveats

### 1. BIOS Control

Most bare metal providers don't give BIOS access by default. You may need to:
- Submit a support ticket requesting TEE enablement
- Specify exact BIOS settings needed (SMEE, IOMMU, RMP, SEV-SNP)
- Verify enablement before signing contracts

### 2. GPU TEE Availability

**Very few providers** offer NVIDIA H100 in Confidential Computing mode outside of major clouds:
- Phala Cloud is essentially the only turnkey GPU TEE option
- OpenMetal offers H100 passthrough but requires manual CC configuration
- Major clouds (Azure NCCadsH100v5, GCP a3-highgpu) have GPU TEE but don't allow custom hypervisors

### 3. Attestation Infrastructure

Running your own CVMs requires setting up attestation services:

```
AMD SEV-SNP Attestation:
├── AMD Key Distribution System (KDS)
├── VCEK certificate retrieval
└── Attestation report verification

Intel TDX Attestation:
├── Intel DCAP (Data Center Attestation Primitives)
├── Intel Trust Authority (optional)
└── Quote verification service

NVIDIA GPU Attestation:
├── NVIDIA Remote Attestation Service (NRAS)
├── SPDM session establishment
└── GPU identity verification
```

### 4. Nested CVMs Limitation

If the provider runs their own hypervisor (not bare metal), you **cannot** run nested CVMs with TEE:
- The hardware TEE is consumed by the provider's virtualization layer
- SEV-SNP/TDX don't support secure nested virtualization (yet)
- Intel TDX 1.5 will add TD Partitioning (future)

### 5. Performance Overhead

Running CVMs incurs performance costs:

| Technology | Boot Time | VMEXIT Latency | Memory Overhead |
|------------|-----------|----------------|-----------------|
| AMD SEV-SNP | +100-230% | +240% | 2-5% |
| Intel TDX | +394% | +472% | 2-5% |
| GPU TEE | N/A | N/A | 5-15% |

---

## Conclusion

While major cloud providers lock customers into their confidential computing offerings, several alternatives exist for organizations needing to run their own CVMs:

**For Full-Stack TEE (CPU + GPU)**:
- **Phala Cloud** remains the only turnkey option for combined Intel TDX + NVIDIA CC

**For CPU TEE with Bare Metal Control**:
- **OVHcloud Scale** offers the best documented AMD SEV-SNP support
- **OpenMetal** provides Intel TDX with enterprise-grade infrastructure

**For Budget-Conscious Deployments**:
- **Hetzner** has capable hardware but may require working with support

The confidential computing landscape is evolving rapidly. Future developments like Intel TDX Connect, AMD SEV-TIO (Trusted I/O), and NVIDIA Blackwell's TEE-I/O will expand options for custom CVM deployments.

---

## References

- AMD SEV-SNP Documentation: https://developer.amd.com/sev
- Intel TDX Documentation: https://github.com/canonical/tdx
- NVIDIA Confidential Computing: https://docs.nvidia.com/cc-deployment-guide-tdx.pdf
- QEMU Confidential Guest Support: https://www.qemu.org/docs/master/system/i386/amd-memory-encryption.html
- Confidential Computing Consortium: https://confidentialcomputing.io

---

*Last Updated: January 2026*
