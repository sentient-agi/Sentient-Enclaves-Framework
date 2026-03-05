# Cloud Providers with TEE-Enabled Hardware for Running Your Own Confidential VMs

## Introduction

Major cloud providers (AWS, Azure, GCP) offer managed confidential VM services, but these come with significant limitations: vendor lock-in, inability to customize the hypervisor stack, and no control over attestation infrastructure. For organizations requiring full control over their confidential computing stack—running their own QEMU/KVM-based CVMs—alternative providers offer bare metal or dedicated hardware with TEE capabilities enabled.

This article provides a comprehensive guide to cloud providers where you can deploy your own confidential VM infrastructure using QEMU/KVM with AMD SEV-SNP or Intel TDX, without being locked into a specific cloud vendor's CVM orchestration layer.

## The Vendor Lock-in Problem

### What Major Clouds DON'T Offer

| Provider | What They Offer | What They DON'T Offer |
|----------|-----------------|----------------------|
| **AWS** | Managed SEV-SNP CVMs (limited regions) | Bare metal with TEE for own CVMs |
| **Azure** | Managed CVMs (SEV-SNP, TDX, SGX) | Bare metal TEE hardware |
| **GCP** | Managed Confidential VMs | Bare metal with full TEE control |

These providers lock you into their CVM orchestration layer, preventing you from:
- Running your own hypervisor/QEMU stack
- Controlling attestation infrastructure
- Customizing the TEE configuration
- Migrating CVM infrastructure to other providers
- Implementing custom security policies at the hypervisor level

## Provider Tiers

### Tier 1: Full CPU TEE + GPU TEE Support

These providers offer complete confidential computing stacks with both CPU and GPU TEE:

| Provider | CPU TEE | GPU TEE | Run QEMU/KVM |
|----------|---------|---------|--------------|
| **Phala Cloud** | Intel TDX | NVIDIA H100/H200 CC Mode | ✅ Yes |
| **OpenMetal** | Intel TDX, SGX | H100 via PCIe passthrough | ✅ Yes |

### Tier 2: CPU TEE Bare Metal (No GPU TEE)

Full hardware control with CPU TEE, but without GPU confidential computing:

| Provider | CPU TEE Technology | Hardware |
|----------|-------------------|----------|
| **OVHcloud Scale** | AMD SEV/SEV-SNP | EPYC 9004 (Genoa) |
| **Hetzner** | AMD SEV (potential) | EPYC 9454P (Genoa) |
| **IP-Projects** | AMD SEV-SNP | EPYC Zen 5 |
| **Cherry Servers** | AMD SEV (potential) | EPYC Milan/Genoa |

## Detailed Provider Analysis

### 1. Phala Cloud

**Best for**: Full-stack TEE with GPU acceleration

Phala Cloud is currently the leading provider for combined CPU and GPU TEE deployments with full user control.

**Hardware Specifications:**
```
CPU TEE: Intel TDX (5th Gen Xeon)
GPU TEE: NVIDIA H100 (80GB HBM3), H200 (141GB HBM3e)
Attestation: Dual (Intel TDX + NVIDIA CC)
Deployment: Docker-based with SSH access available
```

**Pricing (includes TEE protection):**
| GPU Model | On-Demand | Reserved (3+ months) |
|-----------|-----------|---------------------|
| H100 | $3.08/GPU/hr | $2.50/GPU/hr |
| H200 | $3.50/GPU/hr | $2.30/GPU/hr |

**Key Features:**
- Full Intel TDX + NVIDIA Confidential Computing stack
- Deploy custom Docker containers with SSH access
- Public PCCS server for quote verification: `https://pccs.phala.network`
- Open-source attestation tools (dstack)
- Scale from 1 to 8 GPUs per instance

**Run Your Own CVM**: Yes - full control with Docker/QEMU support via dstack infrastructure

---

### 2. OpenMetal

**Best for**: Full hardware control with Intel TDX

OpenMetal provides pre-configured bare metal servers with Intel TDX support, designed specifically for confidential computing deployments.

**Hardware Configurations:**
```
Medium v4: 2x Xeon 4510, 256GB DDR5
Large v4:  2x Xeon Gold 65xx, higher RAM
XL v4:     TDX-ready, production workloads
XXL v4:    H100 GPU support via PCIe passthrough
```

**Requirements for TDX/SGX:**
- 8 DIMMs per CPU socket (symmetric configuration)
- 1TB RAM recommended for production
- BIOS pre-configured for TDX

**Key Features:**
- Pre-configured Intel TDX support out of the box
- QEMU/KVM compatible - launch your own TDX-enabled VMs
- H100 GPU via PCIe passthrough to TDX VMs
- Dual 10 Gbps networking, NVMe storage
- Full BIOS-to-VM control
- Rapid deployment (~45 seconds for private clouds)

**Run Your Own CVM**: Yes - explicitly designed for this use case

**Example Use Cases:**
- Confidential AI training with TDX + GPU passthrough
- Blockchain validators in secure enclaves
- Healthcare data processing with HIPAA compliance
- Financial modeling with data sovereignty requirements

---

### 3. OVHcloud Scale Servers

**Best for**: Cost-effective AMD SEV-SNP bare metal

OVHcloud explicitly markets their Scale servers for confidential computing workloads.

**Scale Server Line (AMD EPYC Genoa/Bergamo):**
```
SCALE-a1 to SCALE-a6: Single socket, 16-96 cores
SCALE-a7: 2x EPYC 9654 (192 cores/384 threads)
SCALE-a8: 2x EPYC 9754 (256 cores/512 threads)
RAM: 128GB to 1TB DDR5
```

**Security Features (AMD Infinity Guard):**
- SEV (Secure Encrypted Virtualization)
- SEV-SNP (Secure Nested Paging)
- SME (Secure Memory Encryption)
- Shadow Stack
- Secure Boot

**Key Features:**
- Explicitly marketed for "confidential computing"
- Full root access to configure QEMU/KVM with SEV-SNP
- Unmetered bandwidth (1-10 Gbps public, up to 25 Gbps private)
- Global datacenter presence (44+ locations)
- Predictable monthly pricing

**Pricing:** Starting ~$513/month for SCALE-a3 (32 cores)

**Run Your Own CVM**: Yes - with manual BIOS/kernel configuration

---

### 4. Hetzner AX Series

**Best for**: Budget-conscious TEE deployment

Hetzner offers competitive pricing on AMD EPYC hardware that is SEV-capable.

**AX162 Series (AMD EPYC Genoa):**
```
CPU: AMD EPYC 9454P (48 cores/96 threads)
AX162-R: 256GB DDR5 ECC, 2x 1.92TB NVMe
AX162-S: 128GB DDR5 ECC, 2x 3.84TB NVMe
AMD-V virtualization supported
```

**Important Caveats:**
- Hardware is SEV-capable (EPYC Genoa has SEV-SNP)
- BIOS configuration for SEV-SNP may require support ticket
- Not explicitly marketed as confidential computing
- Verify BIOS settings with support before ordering

**Pricing:** Very competitive (~€100-200/month range)

**Run Your Own CVM**: Potentially yes - verify BIOS settings with support

---

### 5. IP-Projects (Germany)

**Best for**: European data sovereignty with SEV-SNP

German provider explicitly supporting confidential computing with focus on GDPR compliance.

**AMD EPYC Servers:**
```
Architecture: Zen 5 (EPYC 9005 series)
Cores: Up to 256 cores / 512 threads
RAM: Up to 2TB DDR5 ECC
Storage: NVMe options
```

**Explicitly Supported:**
- AMD Infinity Guard
- SEV-SNP
- GDPR compliance focus
- European datacenter locations

**Run Your Own CVM**: Yes - explicitly supports confidential computing

---

### 6. Cherry Servers

**Best for**: Quick deployment with AMD EPYC

**Available Hardware:**
```
AMD EPYC 7443P (Milan - 3rd Gen)
AMD EPYC 9554P (Genoa - 4th Gen)
Various RAM/storage configurations
```

**Caveats:**
- Hardware is SEV-capable
- TEE features not explicitly marketed
- May require BIOS configuration verification

**Run Your Own CVM**: Potentially yes - contact support for BIOS settings

---

## Technical Requirements for Running Your Own CVMs

### AMD SEV-SNP Host Setup

```bash
# Host Requirements:
# - AMD EPYC Milan (3rd Gen) or newer for SEV-SNP
# - BIOS: SME/SEV/SEV-SNP enabled
# - Kernel: 6.8+ recommended (or AMDESE patched kernel)
# - QEMU: 8.0+ with SEV-SNP support
# - OVMF: TianoCore with SEV-SNP patches

# Verify SEV-SNP on host:
dmesg | grep -i sev
# Expected output: SEV-SNP API:1.55 build:XX

# Check KVM support:
cat /sys/module/kvm_amd/parameters/sev_snp
# Expected: Y

# QEMU command for SEV-SNP guest:
qemu-system-x86_64 \
  -machine q35,confidential-guest-support=sev0 \
  -object sev-snp-guest,id=sev0,cbitpos=51,reduced-phys-bits=1 \
  -cpu EPYC-v4 \
  -bios /path/to/OVMF.fd \
  -m 4G \
  -smp 4 \
  -drive file=guest.qcow2,if=virtio \
  -nographic
```

### Intel TDX Host Setup

```bash
# Host Requirements:
# - Intel Xeon 4th/5th Gen Scalable (Sapphire Rapids/Emerald Rapids)
# - BIOS: TDX, TME-MK enabled
# - 8 DIMMs per socket (symmetric configuration)
# - Kernel: 6.7+ with TDX patches
# - QEMU: 8.0+ with TDX support

# Verify TDX on host:
dmesg | grep tdx
# Expected output: virt/tdx: module initialized

# Check TDX capability:
cat /sys/firmware/tdx/tdx_module/status
# Expected: initialized

# QEMU command for TDX guest:
qemu-system-x86_64 \
  -accel kvm \
  -cpu host \
  -object tdx-guest,id=tdx0 \
  -machine q35,confidential-guest-support=tdx0 \
  -bios /path/to/OVMF_TDX.fd \
  -m 4G \
  -smp 4 \
  -drive file=guest.qcow2,if=virtio \
  -nographic
```

### Required Software Stack

| Component | AMD SEV-SNP Version | Intel TDX Version |
|-----------|--------------------|--------------------|
| **Linux Kernel** | 6.8+ (or AMDESE fork) | 6.7+ (with TDX patches) |
| **QEMU** | 8.0+ | 8.0+ |
| **OVMF/EDK2** | SEV-SNP branch | TDX branch |
| **libvirt** | 9.0+ | 9.0+ |
| **Guest Kernel** | 6.8+ | 6.7+ |

## Comparison Matrix

| Provider | CPU TEE | GPU TEE | Run QEMU/KVM | Attestation Control | Price Range |
|----------|---------|---------|--------------|---------------------|-------------|
| **Phala Cloud** | Intel TDX | H100/H200 CC | ✅ Yes | ✅ Full (PCCS provided) | $2.50-3.50/GPU/hr |
| **OpenMetal** | Intel TDX | H100 passthrough | ✅ Yes | ✅ Full | Custom pricing |
| **OVHcloud Scale** | AMD SEV-SNP | ❌ No | ✅ Yes | ✅ Self-managed | $513+/month |
| **Hetzner AX** | AMD SEV (verify) | ❌ No | ⚠️ Manual config | ⚠️ Verify BIOS | ~€150/month |
| **IP-Projects** | AMD SEV-SNP | ❌ No | ✅ Yes | ✅ Self-managed | Contact |
| **Cherry Servers** | AMD SEV (verify) | ❌ No | ⚠️ Manual config | ⚠️ Verify BIOS | $300+/month |

## Attestation Infrastructure

### Self-Managed Attestation (AMD SEV-SNP)

```bash
# On guest VM, request attestation report:
snpguest report attestation-report.bin request-data.txt

# Fetch certificates from AMD KDS:
snpguest fetch vcek pem ./ attestation-report.bin

# Verify certificate chain:
openssl verify --CAfile ./cert_chain.pem vcek.pem

# Verify attestation report:
snpguest verify attestation attestation-report.bin
```

### Self-Managed Attestation (Intel TDX)

```bash
# TDX attestation requires DCAP infrastructure:
# 1. Install Intel DCAP packages
# 2. Configure Quote Generation Service (QGS)
# 3. Set up PCCS (Provisioning Certificate Caching Service)

# For Phala Cloud, use their public PCCS:
# https://pccs.phala.network

# Generate TDX quote:
# (Inside TD guest)
tdx_attest --report-data <64-bytes-hex>
```

## Recommendations by Use Case

### GPU TEE AI/ML Workloads
**Recommendation**: Phala Cloud
- Only provider with full-stack CPU+GPU TEE
- Open attestation infrastructure
- Competitive GPU pricing with TEE included

### Full Control + GPU Passthrough
**Recommendation**: OpenMetal
- Intel TDX + H100 via PCIe passthrough
- Complete BIOS-to-VM control
- Dedicated hardware with predictable performance

### Cost-Effective AMD SEV-SNP
**Recommendation**: OVHcloud Scale
- Explicit SEV-SNP support
- Predictable monthly pricing
- Unmetered bandwidth

### European Data Sovereignty
**Recommendation**: IP-Projects or OVHcloud
- GDPR focus
- EU datacenter locations
- Explicit confidential computing support

### Budget Bare Metal (DIY TEE Setup)
**Recommendation**: Hetzner
- Very competitive pricing
- Verify BIOS settings with support first
- May require manual configuration

## Migration Considerations

### From Major Cloud CVMs to Self-Managed

Moving from Azure/GCP/AWS managed CVMs to self-managed infrastructure requires:

1. **Attestation Infrastructure**: Set up your own PCCS/verification services
2. **Key Management**: Implement secret provisioning workflow
3. **Guest Image Preparation**: Build SEV-SNP/TDX compatible images
4. **Measurement Baseline**: Establish known-good measurements for verification
5. **Monitoring**: Deploy CVM-aware monitoring and logging

### Benefits of Self-Managed CVMs

- **No Vendor Lock-in**: Migrate between providers easily
- **Full Customization**: Control hypervisor, guest kernel, attestation
- **Cost Optimization**: Avoid cloud provider CVM premiums
- **Compliance Control**: Meet specific regulatory requirements
- **Attestation Sovereignty**: Own your verification infrastructure

## Conclusion

The confidential computing market has a significant gap: major cloud providers offer managed CVMs but prevent running your own QEMU/KVM-based confidential infrastructure. The providers listed in this article fill this gap, with **Phala Cloud** and **OpenMetal** being the only options currently providing both CPU TEE and GPU TEE hardware where you can run your own confidential computing stack without vendor lock-in.

For organizations requiring:
- Complete control over their CVM infrastructure
- GPU TEE capabilities
- Custom attestation workflows
- Multi-cloud or hybrid deployment flexibility

These alternative providers offer a path forward that major cloud vendors currently do not support.

---

*Last updated: January 2026*
