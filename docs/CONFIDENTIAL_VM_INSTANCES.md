# Confidential VM Instance Reference

Complete list of Confidential Virtual Machine (CVM) instances with hardware-based Trusted Execution Environment (TEE) support for Azure and Google Cloud Platform.

---

## Table of Contents

- [Quick Reference](#quick-reference)
- [Azure Confidential VMs](#azure-confidential-vms)
  - [CPU TEE Instances](#azure-cpu-tee-instances)
  - [GPU TEE Instances](#azure-gpu-tee-instances)
- [GCP Confidential VMs](#gcp-confidential-vms)
  - [CPU TEE Instances](#gcp-cpu-tee-instances)
  - [GPU TEE Instances](#gcp-gpu-tee-instances)
- [Technology Comparison](#technology-comparison)
- [Feature Differences](#feature-differences)

---

## Quick Reference

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                    CONFIDENTIAL VM QUICK REFERENCE                              │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  AZURE CPU TEE                          GCP CPU TEE                             │
│  ─────────────                          ───────────                             │
│  DCasv5  │ AMD SEV-SNP │ 2-96 vCPU     N2D    │ AMD SEV/SNP │ 2-224 vCPU        │
│  DCadsv5 │ AMD SEV-SNP │ 2-96 vCPU     C2D    │ AMD SEV     │ 2-112 vCPU        │
│  ECasv5  │ AMD SEV-SNP │ 2-96 vCPU     C3D    │ AMD SEV     │ 4-360 vCPU        │
│  ECadsv5 │ AMD SEV-SNP │ 2-96 vCPU     C4D    │ AMD SEV     │ 2-192 vCPU        │
│  DCesv5  │ Intel TDX   │ 2-96 vCPU     C3     │ Intel TDX   │ 4-176 vCPU        │
│  DCedsv5 │ Intel TDX   │ 2-96 vCPU                                              │
│  ECesv5  │ Intel TDX   │ 2-128 vCPU                                             │
│  ECedsv5 │ Intel TDX   │ 2-128 vCPU                                             │
│                                                                                 │
│  AZURE GPU TEE                          GCP GPU TEE                             │
│  ─────────────                          ───────────                             │
│  NCCadsH100v5 │ SEV-SNP + H100          a3-highgpu-1g │ TDX + H100              │
│               │ 1 GPU, 40 vCPU                        │ 1 GPU, 26 vCPU          │
│               │ 320 GiB RAM                           │ 234 GB RAM              │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Azure Confidential VMs

### Azure CPU TEE Instances

#### AMD SEV-SNP Series (3rd Gen AMD EPYC Milan)

**DCasv5-series** - General Purpose (No Local Disk)

```
┌────────────────────┬────────┬────────────┬───────────┬──────────────┬─────────────┐
│     Instance       │ vCPUs  │ Memory     │ Max Disks │ Network      │ Max IOPS    │
│                    │        │ (GiB)      │           │ (Mbps)       │             │
├────────────────────┼────────┼────────────┼───────────┼──────────────┼─────────────┤
│ Standard_DC2as_v5  │   2    │     8      │     4     │    3,000     │   3,750     │
│ Standard_DC4as_v5  │   4    │    16      │     8     │    5,000     │   6,400     │
│ Standard_DC8as_v5  │   8    │    32      │    16     │    5,000     │  12,800     │
│ Standard_DC16as_v5 │  16    │    64      │    32     │   10,000     │  25,600     │
│ Standard_DC32as_v5 │  32    │   128      │    32     │   12,500     │  51,200     │
│ Standard_DC48as_v5 │  48    │   192      │    32     │   15,000     │  76,800     │
│ Standard_DC64as_v5 │  64    │   256      │    32     │   20,000     │  80,000     │
│ Standard_DC96as_v5 │  96    │   384      │    32     │   20,000     │  80,000     │
└────────────────────┴────────┴────────────┴───────────┴──────────────┴─────────────┘

Processor: AMD EPYC 7763v (Milan) @ 3.5 GHz boost
TEE: AMD SEV-SNP
Memory Ratio: 4 GiB per vCPU
Local Storage: None
```

**DCadsv5-series** - General Purpose (With Local NVMe)

```
┌─────────────────────┬────────┬────────────┬─────────────────┬───────────┬──────────┐
│     Instance        │ vCPUs  │ Memory     │ Temp Storage    │ Network   │ Max IOPS │
│                     │        │ (GiB)      │ (GiB)           │ (Mbps)    │          │
├─────────────────────┼────────┼────────────┼─────────────────┼───────────┼──────────┤
│ Standard_DC2ads_v5  │   2    │     8      │      75         │   3,000   │   3,750  │
│ Standard_DC4ads_v5  │   4    │    16      │     150         │   5,000   │   6,400  │
│ Standard_DC8ads_v5  │   8    │    32      │     300         │   5,000   │  12,800  │
│ Standard_DC16ads_v5 │  16    │    64      │     600         │  10,000   │  25,600  │
│ Standard_DC32ads_v5 │  32    │   128      │   1,200         │  12,500   │  51,200  │
│ Standard_DC48ads_v5 │  48    │   192      │   1,800         │  15,000   │  76,800  │
│ Standard_DC64ads_v5 │  64    │   256      │   2,400         │  20,000   │  80,000  │
│ Standard_DC96ads_v5 │  96    │   384      │   3,600         │  20,000   │  80,000  │
└─────────────────────┴────────┴────────────┴─────────────────┴───────────┴──────────┘

Processor: AMD EPYC 7763v (Milan) @ 3.5 GHz boost
TEE: AMD SEV-SNP
Memory Ratio: 4 GiB per vCPU
Local Storage: NVMe SSD (encrypted)
```

**ECasv5-series** - Memory Optimized (No Local Disk)

```
┌────────────────────┬────────┬────────────┬───────────┬──────────────┬─────────────┐
│     Instance       │ vCPUs  │ Memory     │ Max Disks │ Network      │ Max IOPS    │
│                    │        │ (GiB)      │           │ (Mbps)       │             │
├────────────────────┼────────┼────────────┼───────────┼──────────────┼─────────────┤
│ Standard_EC2as_v5  │   2    │    16      │     4     │    3,000     │   3,750     │
│ Standard_EC4as_v5  │   4    │    32      │     8     │    5,000     │   6,400     │
│ Standard_EC8as_v5  │   8    │    64      │    16     │    5,000     │  12,800     │
│ Standard_EC16as_v5 │  16    │   128      │    32     │   10,000     │  25,600     │
│ Standard_EC32as_v5 │  32    │   256      │    32     │   12,500     │  51,200     │
│ Standard_EC48as_v5 │  48    │   384      │    32     │   15,000     │  76,800     │
│ Standard_EC64as_v5 │  64    │   512      │    32     │   20,000     │  80,000     │
│ Standard_EC96as_v5 │  96    │   672      │    32     │   20,000     │  80,000     │
└────────────────────┴────────┴────────────┴───────────┴──────────────┴─────────────┘

Processor: AMD EPYC 7763v (Milan) @ 3.5 GHz boost
TEE: AMD SEV-SNP
Memory Ratio: 8 GiB per vCPU (Memory Optimized)
Local Storage: None
```

**ECadsv5-series** - Memory Optimized (With Local NVMe)

```
┌─────────────────────┬────────┬────────────┬─────────────────┬───────────┬──────────┐
│     Instance        │ vCPUs  │ Memory     │ Temp Storage    │ Network   │ Max IOPS │
│                     │        │ (GiB)      │ (GiB)           │ (Mbps)    │          │
├─────────────────────┼────────┼────────────┼─────────────────┼───────────┼──────────┤
│ Standard_EC2ads_v5  │   2    │    16      │      75         │   3,000   │   3,750  │
│ Standard_EC4ads_v5  │   4    │    32      │     150         │   5,000   │   6,400  │
│ Standard_EC8ads_v5  │   8    │    64      │     300         │   5,000   │  12,800  │
│ Standard_EC16ads_v5 │  16    │   128      │     600         │  10,000   │  25,600  │
│ Standard_EC32ads_v5 │  32    │   256      │   1,200         │  12,500   │  51,200  │
│ Standard_EC48ads_v5 │  48    │   384      │   1,800         │  15,000   │  76,800  │
│ Standard_EC64ads_v5 │  64    │   512      │   2,400         │  20,000   │  80,000  │
│ Standard_EC96ads_v5 │  96    │   672      │   3,600         │  20,000   │  80,000  │
└─────────────────────┴────────┴────────────┴─────────────────┴───────────┴──────────┘

Processor: AMD EPYC 7763v (Milan) @ 3.5 GHz boost
TEE: AMD SEV-SNP
Memory Ratio: 8 GiB per vCPU (Memory Optimized)
Local Storage: NVMe SSD (encrypted)
```

#### Intel TDX Series (4th Gen Intel Xeon Sapphire Rapids)

**DCesv5-series** - General Purpose (No Local Disk)

```
┌────────────────────┬────────┬────────────┬───────────┬──────────────┬─────────────┐
│     Instance       │ vCPUs  │ Memory     │ Max Disks │ Network      │ Max IOPS    │
│                    │        │ (GiB)      │           │ (Mbps)       │             │
├────────────────────┼────────┼────────────┼───────────┼──────────────┼─────────────┤
│ Standard_DC2es_v5  │   2    │     8      │     4     │    3,000     │   3,750     │
│ Standard_DC4es_v5  │   4    │    16      │     8     │    5,000     │   6,400     │
│ Standard_DC8es_v5  │   8    │    32      │    16     │    5,000     │  12,800     │
│ Standard_DC16es_v5 │  16    │    64      │    32     │   10,000     │  25,600     │
│ Standard_DC32es_v5 │  32    │   128      │    32     │   12,500     │  51,200     │
│ Standard_DC48es_v5 │  48    │   192      │    32     │   15,000     │  76,800     │
│ Standard_DC64es_v5 │  64    │   256      │    32     │   20,000     │  80,000     │
│ Standard_DC96es_v5 │  96    │   384      │    32     │   20,000     │  80,000     │
└────────────────────┴────────┴────────────┴───────────┴──────────────┴─────────────┘

Processor: Intel Xeon Scalable (Sapphire Rapids) @ 2.1 GHz base / 2.9 GHz turbo
TEE: Intel TDX (Trust Domain Extensions)
Memory Ratio: 4 GiB per vCPU
Features: Intel AMX for AI acceleration
Local Storage: None
```

**DCedsv5-series** - General Purpose (With Local NVMe)

```
┌─────────────────────┬────────┬────────────┬─────────────────┬───────────┬──────────┐
│     Instance        │ vCPUs  │ Memory     │ Temp Storage    │ Network   │ Max IOPS │
│                     │        │ (GiB)      │ (GiB)           │ (Mbps)    │          │
├─────────────────────┼────────┼────────────┼─────────────────┼───────────┼──────────┤
│ Standard_DC2eds_v5  │   2    │     8      │      75         │   3,000   │   3,750  │
│ Standard_DC4eds_v5  │   4    │    16      │     150         │   5,000   │   6,400  │
│ Standard_DC8eds_v5  │   8    │    32      │     300         │   5,000   │  12,800  │
│ Standard_DC16eds_v5 │  16    │    64      │     600         │  10,000   │  25,600  │
│ Standard_DC32eds_v5 │  32    │   128      │   1,200         │  12,500   │  51,200  │
│ Standard_DC48eds_v5 │  48    │   192      │   1,800         │  15,000   │  76,800  │
│ Standard_DC64eds_v5 │  64    │   256      │   2,400         │  20,000   │  80,000  │
│ Standard_DC96eds_v5 │  96    │   384      │   2,800         │  20,000   │  80,000  │
└─────────────────────┴────────┴────────────┴─────────────────┴───────────┴──────────┘

Processor: Intel Xeon Scalable (Sapphire Rapids) @ 2.1 GHz base / 2.9 GHz turbo
TEE: Intel TDX (Trust Domain Extensions)
Memory Ratio: 4 GiB per vCPU
Features: Intel AMX for AI acceleration
Local Storage: NVMe SSD (encrypted)
```

**ECesv5-series** - Memory Optimized (No Local Disk)

```
┌────────────────────┬────────┬────────────┬───────────┬──────────────┬─────────────┐
│     Instance       │ vCPUs  │ Memory     │ Max Disks │ Network      │ Max IOPS    │
│                    │        │ (GiB)      │           │ (Mbps)       │             │
├────────────────────┼────────┼────────────┼───────────┼──────────────┼─────────────┤
│ Standard_EC2es_v5  │   2    │    16      │     4     │    3,000     │   3,750     │
│ Standard_EC4es_v5  │   4    │    32      │     8     │    5,000     │   6,400     │
│ Standard_EC8es_v5  │   8    │    64      │    16     │    5,000     │  12,800     │
│ Standard_EC16es_v5 │  16    │   128      │    32     │   10,000     │  25,600     │
│ Standard_EC32es_v5 │  32    │   256      │    32     │   12,500     │  51,200     │
│ Standard_EC48es_v5 │  48    │   384      │    32     │   15,000     │  76,800     │
│ Standard_EC64es_v5 │  64    │   512      │    32     │   20,000     │  80,000     │
│ Standard_EC96es_v5 │  96    │   672      │    32     │   20,000     │  80,000     │
│ Standard_EC128es_v5│ 128    │   768      │    32     │   25,000     │  80,000     │
└────────────────────┴────────┴────────────┴───────────┴──────────────┴─────────────┘

Processor: Intel Xeon Scalable (Sapphire Rapids) @ 2.1 GHz base / 2.9 GHz turbo
TEE: Intel TDX (Trust Domain Extensions)
Memory Ratio: 6-8 GiB per vCPU (Memory Optimized)
Features: Intel AMX for AI acceleration
Local Storage: None
```

**ECedsv5-series** - Memory Optimized (With Local NVMe)

```
┌─────────────────────┬────────┬────────────┬─────────────────┬───────────┬──────────┐
│     Instance        │ vCPUs  │ Memory     │ Temp Storage    │ Network   │ Max IOPS │
│                     │        │ (GiB)      │ (GiB)           │ (Mbps)    │          │
├─────────────────────┼────────┼────────────┼─────────────────┼───────────┼──────────┤
│ Standard_EC2eds_v5  │   2    │    16      │      75         │   3,000   │   3,750  │
│ Standard_EC4eds_v5  │   4    │    32      │     150         │   5,000   │   6,400  │
│ Standard_EC8eds_v5  │   8    │    64      │     300         │   5,000   │  12,800  │
│ Standard_EC16eds_v5 │  16    │   128      │     600         │  10,000   │  25,600  │
│ Standard_EC32eds_v5 │  32    │   256      │   1,200         │  12,500   │  51,200  │
│ Standard_EC48eds_v5 │  48    │   384      │   1,800         │  15,000   │  76,800  │
│ Standard_EC64eds_v5 │  64    │   512      │   2,400         │  20,000   │  80,000  │
│ Standard_EC96eds_v5 │  96    │   672      │   2,800         │  20,000   │  80,000  │
│ Standard_EC128eds_v5│ 128    │   768      │   3,800         │  25,000   │  80,000  │
└─────────────────────┴────────┴────────────┴─────────────────┴───────────┴──────────┘

Processor: Intel Xeon Scalable (Sapphire Rapids) @ 2.1 GHz base / 2.9 GHz turbo
TEE: Intel TDX (Trust Domain Extensions)
Memory Ratio: 6-8 GiB per vCPU (Memory Optimized)
Features: Intel AMX for AI acceleration
Local Storage: NVMe SSD (encrypted)
```

---

### Azure GPU TEE Instances

#### NCCadsH100v5-series (AMD SEV-SNP + NVIDIA H100)

```
┌──────────────────────────┬────────┬────────────┬──────────┬─────────────┬───────────┐
│     Instance             │ vCPUs  │ Memory     │ GPUs     │ GPU Memory  │ Network   │
│                          │        │ (GiB)      │          │ (GB)        │ (Gbps)    │
├──────────────────────────┼────────┼────────────┼──────────┼─────────────┼───────────┤
│ Standard_NCC40ads_H100_v5│   40   │    320     │  1xH100  │     94      │    80     │
└──────────────────────────┴────────┴────────────┴──────────┴─────────────┴───────────┘

┌─────────────────────────────────────────────────────────────────────────────────────┐
│                     NCCadsH100v5 SPECIFICATIONS                                     │
├─────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                     │
│  CPU TEE                                                                            │
│  ───────                                                                            │
│  Processor    : AMD EPYC (Genoa) 4th Generation                                     │
│  TEE Tech     : AMD SEV-SNP                                                         │
│  vCPUs        : 40 (non-multithreaded)                                              │
│  Memory       : 320 GiB                                                             │
│  Memory Encrypt: AES-256 (hardware)                                                 │
│                                                                                     │
│  GPU TEE                                                                            │
│  ───────                                                                            │
│  GPU Model    : NVIDIA H100 NVL (Hopper)                                            │
│  GPU Count    : 1                                                                   │
│  GPU Memory   : 94 GB HBM3 (encrypted)                                              │
│  GPU TEE      : NVIDIA Confidential Computing                                       │
│  PCIe         : Encrypted via SPDM/AES-GCM                                          │
│                                                                                     │
│  STORAGE                                                                            │
│  ───────                                                                            │
│  Local NVMe   : 1x 960 GB (ephemeral)                                               │
│  Max Disks    : 8                                                                   │
│  Max IOPS     : 80,000                                                              │
│                                                                                     │
│  NETWORK                                                                            │
│  ───────                                                                            │
│  Bandwidth    : 80 Gbps                                                             │
│  NICs         : Up to 8                                                             │
│                                                                                     │
│  USE CASES                                                                          │
│  ─────────                                                                          │
│  • Confidential AI/ML training and inference                                        │
│  • Secure LLM fine-tuning (Llama2, Falcon, etc.)                                    │
│  • Protected image generation (Stable Diffusion)                                    │
│  • Private healthcare AI models                                                     │
│  • Financial fraud detection with sensitive data                                    │
│                                                                                     │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

---

## GCP Confidential VMs

### GCP CPU TEE Instances

#### AMD SEV Series

**N2D Machine Series** (AMD EPYC Milan - SEV and SEV-SNP)

```
┌─────────────────────┬────────┬────────────┬────────────┬──────────────┐
│     Machine Type    │ vCPUs  │ Memory     │ Memory/    │ Confidential │
│                     │        │ (GB)       │ vCPU (GB)  │ Technology   │
├─────────────────────┼────────┼────────────┼────────────┼──────────────┤
│ n2d-standard-2      │   2    │     8      │    4       │ SEV, SEV-SNP │
│ n2d-standard-4      │   4    │    16      │    4       │ SEV, SEV-SNP │
│ n2d-standard-8      │   8    │    32      │    4       │ SEV, SEV-SNP │
│ n2d-standard-16     │  16    │    64      │    4       │ SEV, SEV-SNP │
│ n2d-standard-32     │  32    │   128      │    4       │ SEV, SEV-SNP │
│ n2d-standard-48     │  48    │   192      │    4       │ SEV, SEV-SNP │
│ n2d-standard-64     │  64    │   256      │    4       │ SEV, SEV-SNP │
│ n2d-standard-80     │  80    │   320      │    4       │ SEV, SEV-SNP │
│ n2d-standard-96     │  96    │   384      │    4       │ SEV, SEV-SNP │
│ n2d-standard-128    │ 128    │   512      │    4       │ SEV, SEV-SNP │
│ n2d-standard-224    │ 224    │   896      │    4       │ SEV, SEV-SNP │
├─────────────────────┼────────┼────────────┼────────────┼──────────────┤
│ n2d-highmem-2       │   2    │    16      │    8       │ SEV, SEV-SNP │
│ n2d-highmem-4       │   4    │    32      │    8       │ SEV, SEV-SNP │
│ n2d-highmem-8       │   8    │    64      │    8       │ SEV, SEV-SNP │
│ n2d-highmem-16      │  16    │   128      │    8       │ SEV, SEV-SNP │
│ n2d-highmem-32      │  32    │   256      │    8       │ SEV, SEV-SNP │
│ n2d-highmem-48      │  48    │   384      │    8       │ SEV, SEV-SNP │
│ n2d-highmem-64      │  64    │   512      │    8       │ SEV, SEV-SNP │
│ n2d-highmem-80      │  80    │   640      │    8       │ SEV, SEV-SNP │
│ n2d-highmem-96      │  96    │   768      │    8       │ SEV, SEV-SNP │
├─────────────────────┼────────┼────────────┼────────────┼──────────────┤
│ n2d-highcpu-2       │   2    │     2      │    1       │ SEV, SEV-SNP │
│ n2d-highcpu-4       │   4    │     4      │    1       │ SEV, SEV-SNP │
│ n2d-highcpu-8       │   8    │     8      │    1       │ SEV, SEV-SNP │
│ n2d-highcpu-16      │  16    │    16      │    1       │ SEV, SEV-SNP │
│ n2d-highcpu-32      │  32    │    32      │    1       │ SEV, SEV-SNP │
│ n2d-highcpu-48      │  48    │    48      │    1       │ SEV, SEV-SNP │
│ n2d-highcpu-64      │  64    │    64      │    1       │ SEV, SEV-SNP │
│ n2d-highcpu-80      │  80    │    80      │    1       │ SEV, SEV-SNP │
│ n2d-highcpu-96      │  96    │    96      │    1       │ SEV, SEV-SNP │
│ n2d-highcpu-128     │ 128    │   128      │    1       │ SEV, SEV-SNP │
│ n2d-highcpu-224     │ 224    │   224      │    1       │ SEV, SEV-SNP │
└─────────────────────┴────────┴────────────┴────────────┴──────────────┘

Processor: AMD EPYC Milan (3rd Gen) @ 2.25-3.3 GHz
TEE: AMD SEV (all regions) or AMD SEV-SNP (select regions)
SEV-SNP Regions: asia-southeast1, us-central1, europe-west3, europe-west4
Live Migration: Supported (SEV only, not SEV-SNP)
Custom Machine Types: Supported
```

**C2D Machine Series** (AMD EPYC Milan - SEV Only)

```
┌─────────────────────┬────────┬────────────┬────────────┬──────────────┐
│     Machine Type    │ vCPUs  │ Memory     │ Memory/    │ Confidential │
│                     │        │ (GB)       │ vCPU (GB)  │ Technology   │
├─────────────────────┼────────┼────────────┼────────────┼──────────────┤
│ c2d-standard-2      │   2    │     8      │    4       │ SEV          │
│ c2d-standard-4      │   4    │    16      │    4       │ SEV          │
│ c2d-standard-8      │   8    │    32      │    4       │ SEV          │
│ c2d-standard-16     │  16    │    64      │    4       │ SEV          │
│ c2d-standard-32     │  32    │   128      │    4       │ SEV          │
│ c2d-standard-56     │  56    │   224      │    4       │ SEV          │
│ c2d-standard-112    │ 112    │   448      │    4       │ SEV          │
├─────────────────────┼────────┼────────────┼────────────┼──────────────┤
│ c2d-highmem-2       │   2    │    16      │    8       │ SEV          │
│ c2d-highmem-4       │   4    │    32      │    8       │ SEV          │
│ c2d-highmem-8       │   8    │    64      │    8       │ SEV          │
│ c2d-highmem-16      │  16    │   128      │    8       │ SEV          │
│ c2d-highmem-32      │  32    │   256      │    8       │ SEV          │
│ c2d-highmem-56      │  56    │   448      │    8       │ SEV          │
│ c2d-highmem-112     │ 112    │   896      │    8       │ SEV          │
├─────────────────────┼────────┼────────────┼────────────┼──────────────┤
│ c2d-highcpu-2       │   2    │     4      │    2       │ SEV          │
│ c2d-highcpu-4       │   4    │     8      │    2       │ SEV          │
│ c2d-highcpu-8       │   8    │    16      │    2       │ SEV          │
│ c2d-highcpu-16      │  16    │    32      │    2       │ SEV          │
│ c2d-highcpu-32      │  32    │    64      │    2       │ SEV          │
│ c2d-highcpu-56      │  56    │   112      │    2       │ SEV          │
│ c2d-highcpu-112     │ 112    │   224      │    2       │ SEV          │
└─────────────────────┴────────┴────────────┴────────────┴──────────────┘

Processor: AMD EPYC Milan (3rd Gen) @ 3.5 GHz max boost
TEE: AMD SEV
Live Migration: Not Supported
Memory Integrity: Not Supported (SEV only)
```

**C3D Machine Series** (AMD EPYC Genoa - SEV Only)

```
┌─────────────────────┬────────┬────────────┬────────────┬──────────────┐
│     Machine Type    │ vCPUs  │ Memory     │ Memory/    │ Confidential │
│                     │        │ (GB)       │ vCPU (GB)  │ Technology   │
├─────────────────────┼────────┼────────────┼────────────┼──────────────┤
│ c3d-standard-4      │   4    │    16      │    4       │ SEV          │
│ c3d-standard-8      │   8    │    32      │    4       │ SEV          │
│ c3d-standard-16     │  16    │    64      │    4       │ SEV          │
│ c3d-standard-30     │  30    │   120      │    4       │ SEV          │
│ c3d-standard-60     │  60    │   240      │    4       │ SEV          │
│ c3d-standard-90     │  90    │   360      │    4       │ SEV          │
│ c3d-standard-180    │ 180    │   720      │    4       │ SEV          │
│ c3d-standard-360    │ 360    │  1440      │    4       │ SEV          │
├─────────────────────┼────────┼────────────┼────────────┼──────────────┤
│ c3d-highmem-4       │   4    │    32      │    8       │ SEV          │
│ c3d-highmem-8       │   8    │    64      │    8       │ SEV          │
│ c3d-highmem-16      │  16    │   128      │    8       │ SEV          │
│ c3d-highmem-30      │  30    │   240      │    8       │ SEV          │
│ c3d-highmem-60      │  60    │   480      │    8       │ SEV          │
│ c3d-highmem-90      │  90    │   720      │    8       │ SEV          │
│ c3d-highmem-180     │ 180    │  1440      │    8       │ SEV          │
│ c3d-highmem-360     │ 360    │  2880      │    8       │ SEV          │
├─────────────────────┼────────┼────────────┼────────────┼──────────────┤
│ c3d-highcpu-4       │   4    │     8      │    2       │ SEV          │
│ c3d-highcpu-8       │   8    │    16      │    2       │ SEV          │
│ c3d-highcpu-16      │  16    │    32      │    2       │ SEV          │
│ c3d-highcpu-30      │  30    │    60      │    2       │ SEV          │
│ c3d-highcpu-60      │  60    │   120      │    2       │ SEV          │
│ c3d-highcpu-90      │  90    │   180      │    2       │ SEV          │
│ c3d-highcpu-180     │ 180    │   360      │    2       │ SEV          │
│ c3d-highcpu-360     │ 360    │   720      │    2       │ SEV          │
└─────────────────────┴────────┴────────────┴────────────┴──────────────┘

Processor: AMD EPYC Genoa (4th Gen) @ 3.7 GHz max boost
TEE: AMD SEV
Titanium: Powered by Google Titanium
Live Migration: Not Supported
Max vCPUs: 360 (largest confidential instance)
```

**C4D Machine Series** (AMD EPYC Turin - SEV Only)

```
┌─────────────────────┬────────┬────────────┬────────────┬──────────────┐
│     Machine Type    │ vCPUs  │ Memory     │ Memory/    │ Confidential │
│                     │        │ (GB)       │ vCPU (GB)  │ Technology   │
├─────────────────────┼────────┼────────────┼────────────┼──────────────┤
│ c4d-standard-2      │   2    │     8      │   3.875    │ SEV          │
│ c4d-standard-4      │   4    │    16      │   3.875    │ SEV          │
│ c4d-standard-8      │   8    │    31      │   3.875    │ SEV          │
│ c4d-standard-16     │  16    │    62      │   3.875    │ SEV          │
│ c4d-standard-32     │  32    │   124      │   3.875    │ SEV          │
│ c4d-standard-48     │  48    │   186      │   3.875    │ SEV          │
│ c4d-standard-96     │  96    │   372      │   3.875    │ SEV          │
│ c4d-standard-192    │ 192    │   744      │   3.875    │ SEV          │
└─────────────────────┴────────┴────────────┴────────────┴──────────────┘

Processor: AMD EPYC Turin (5th Gen) @ 4.1 GHz max boost
TEE: AMD SEV
Titanium: Powered by Google Titanium
Performance: ~30% improvement over C3D
```

#### Intel TDX Series

**C3 Machine Series** (Intel Sapphire Rapids - TDX)

```
┌─────────────────────┬────────┬────────────┬────────────┬──────────────┐
│     Machine Type    │ vCPUs  │ Memory     │ Memory/    │ Confidential │
│                     │        │ (GB)       │ vCPU (GB)  │ Technology   │
├─────────────────────┼────────┼────────────┼────────────┼──────────────┤
│ c3-standard-4       │   4    │    16      │    4       │ Intel TDX    │
│ c3-standard-8       │   8    │    32      │    4       │ Intel TDX    │
│ c3-standard-22      │  22    │    88      │    4       │ Intel TDX    │
│ c3-standard-44      │  44    │   176      │    4       │ Intel TDX    │
│ c3-standard-88      │  88    │   352      │    4       │ Intel TDX    │
│ c3-standard-176     │ 176    │   704      │    4       │ Intel TDX    │
├─────────────────────┼────────┼────────────┼────────────┼──────────────┤
│ c3-highmem-4        │   4    │    32      │    8       │ Intel TDX    │
│ c3-highmem-8        │   8    │    64      │    8       │ Intel TDX    │
│ c3-highmem-22       │  22    │   176      │    8       │ Intel TDX    │
│ c3-highmem-44       │  44    │   352      │    8       │ Intel TDX    │
│ c3-highmem-88       │  88    │   704      │    8       │ Intel TDX    │
│ c3-highmem-176      │ 176    │  1408      │    8       │ Intel TDX    │
├─────────────────────┼────────┼────────────┼────────────┼──────────────┤
│ c3-highcpu-4        │   4    │     8      │    2       │ Intel TDX    │
│ c3-highcpu-8        │   8    │    16      │    2       │ Intel TDX    │
│ c3-highcpu-22       │  22    │    44      │    2       │ Intel TDX    │
│ c3-highcpu-44       │  44    │    88      │    2       │ Intel TDX    │
│ c3-highcpu-88       │  88    │   176      │    2       │ Intel TDX    │
│ c3-highcpu-176      │ 176    │   352      │    2       │ Intel TDX    │
└─────────────────────┴────────┴────────────┴────────────┴──────────────┘

Processor: Intel Xeon Scalable (Sapphire Rapids, 4th Gen)
TEE: Intel TDX (Trust Domain Extensions)
TDX Regions: asia-southeast1, us-central1, europe-west4
Titanium: Powered by Google Titanium
Memory Integrity: Supported (MAC-based)
Live Migration: Not Supported
```

---

### GCP GPU TEE Instances

#### A3 High Machine Series (Intel TDX + NVIDIA H100)

```
┌──────────────────────┬────────┬────────────┬──────────┬─────────────┬───────────┐
│     Machine Type     │ vCPUs  │ Memory     │ GPUs     │ GPU Memory  │ Network   │
│                      │        │ (GB)       │          │ (GB)        │ (Gbps)    │
├──────────────────────┼────────┼────────────┼──────────┼─────────────┼───────────┤
│ a3-highgpu-1g        │   26   │    234     │  1xH100  │     80      │   200     │
│ a3-highgpu-2g        │   52   │    468     │  2xH100  │    160      │   200     │
│ a3-highgpu-4g        │  104   │    936     │  4xH100  │    320      │   200     │
│ a3-highgpu-8g        │  208   │  1,872     │  8xH100  │    640      │   200     │
└──────────────────────┴────────┴────────────┴──────────┴─────────────┴───────────┘

Notes:
- Only a3-highgpu-1g currently supports Confidential VM with GPU
- TEE: Intel TDX + NVIDIA Confidential Computing
```

**Confidential GPU Instance Details**

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                     a3-highgpu-1g (CONFIDENTIAL) SPECIFICATIONS                     │
├─────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                     │
│  CPU TEE                                                                            │
│  ───────                                                                            │
│  Processor    : Intel Xeon Scalable (Sapphire Rapids, 4th Gen)                      │
│  TEE Tech     : Intel TDX (Trust Domain Extensions)                                 │
│  vCPUs        : 26                                                                  │
│  Memory       : 234 GB DDR5                                                         │
│  Memory Encrypt: AES-XTS via TME-MK                                                 │
│  Titanium     : Google Titanium security chip                                       │
│                                                                                     │
│  GPU TEE                                                                            │
│  ───────                                                                            │
│  GPU Model    : NVIDIA H100 SXM (Hopper)                                            │
│  GPU Count    : 1                                                                   │
│  GPU Memory   : 80 GB HBM3 (encrypted)                                              │
│  GPU TEE      : NVIDIA Confidential Computing                                       │
│  GPU-CPU Link : NVLink + PCIe (encrypted)                                           │
│                                                                                     │
│  STORAGE                                                                            │
│  ───────                                                                            │
│  Local SSD    : 2x 375 GB (ephemeral)                                               │
│  Boot Disk    : Balanced PD (NVMe interface required)                               │
│                                                                                     │
│  NETWORK                                                                            │
│  ───────                                                                            │
│  Bandwidth    : Up to 200 Gbps                                                      │
│  GPUDirect    : TCPX/TCPXO supported                                                │
│                                                                                     │
│  LIMITATIONS                                                                        │
│  ───────────                                                                        │
│  • Limited regional availability                                                    │
│  • No multi-node cluster support                                                    │
│  • No reservations supported                                                        │
│  • No Hyperdisk Extreme                                                             │
│                                                                                     │
│  USE CASES                                                                          │
│  ─────────                                                                          │
│  • Confidential LLM inference                                                       │
│  • Secure model fine-tuning                                                         │
│  • Private AI research                                                              │
│  • Healthcare/Financial AI workloads                                                │
│                                                                                     │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

---

## Technology Comparison

### TEE Technology Matrix

```
┌────────────────────┬─────────────────────────┬─────────────────────────────────────┐
│     Feature        │       AMD SEV-SNP       │          Intel TDX                  │
├────────────────────┼─────────────────────────┼─────────────────────────────────────┤
│ Vendor             │ AMD                     │ Intel                               │
│ CPU Generation     │ 3rd Gen EPYC (Milan)+   │ 4th Gen Xeon (Sapphire Rapids)+     │
│ Isolation Level    │ Full VM                 │ Trust Domain (TD)                   │
│ Encryption         │ AES-256                 │ AES-128/256 (TME-MK)                │
│ Integrity          │ RMP (Reverse Map Table) │ SEPT + MAC                          │
│ Key Management     │ AMD Secure Processor    │ TDX Module (SEAM mode)              │
│ Attestation        │ VCEK (chip-unique)      │ ECDSA via SGX Quoting Enclave       │
│ Root of Trust      │ AMD SP (PSP)            │ Intel TDX Module                    │
│ Exception Type     │ #VC (VMM Communication) │ #VE (Virtualization Exception)      │
│ Memory Pages       │ Private vs Shared       │ Private vs Shared                   │
│ Page Tracking      │ RMP table               │ Secure EPT (SEPT)                   │
│ Anti-Rollback      │ ✓                       │ ✓                                   │
│ Anti-Replay        │ ✓                       │ ✓                                   │
│ Live Migration     │ ✗                       │ ✗                                   │
└────────────────────┴─────────────────────────┴─────────────────────────────────────┘
```

### Provider Comparison

```
┌────────────────────────────┬──────────────────────────┬──────────────────────────┐
│         Feature            │          Azure           │           GCP            │
├────────────────────────────┼──────────────────────────┼──────────────────────────┤
│ AMD SEV (Basic)            │            ✗             │     ✓ (N2D, C2D, C3D)    │
│ AMD SEV-SNP                │  ✓ (DCasv5, ECasv5)      │     ✓ (N2D only)         │
│ Intel TDX                  │  ✓ (DCesv5, ECesv5)      │     ✓ (C3 only)          │
│ Intel SGX (Enclaves)       │  ✓ (DCsv3)               │            ✗             │
├────────────────────────────┼──────────────────────────┼──────────────────────────┤
│ Max vCPUs (SEV-SNP)        │           96             │           224            │
│ Max Memory (SEV-SNP)       │        672 GiB           │         896 GB           │
│ Max vCPUs (TDX)            │          128             │           176            │
│ Max Memory (TDX)           │        768 GiB           │        1408 GB           │
├────────────────────────────┼──────────────────────────┼──────────────────────────┤
│ GPU TEE                    │     ✓ (NCCadsH100v5)     │     ✓ (a3-highgpu-1g)    │
│ GPU TEE Technology         │  AMD SEV-SNP + H100 NVL  │  Intel TDX + H100 SXM    │
│ Max GPU Memory             │          94 GB           │          80 GB           │
│ GPU Count (Confidential)   │            1             │            1             │
├────────────────────────────┼──────────────────────────┼──────────────────────────┤
│ Confidential OS Disk       │     ✓ (PMK/CMK)          │     ✓ (vTPM sealing)     │
│ Hardware vTPM              │  ✓ (in TEE)              │  ✓ (managed/hardware)    │
│ Attestation Service        │  Azure Attestation + ITA │  GCP Attestation + ITA   │
│ Live Migration (SEV)       │            ✗             │     ✓ (N2D SEV only)     │
├────────────────────────────┼──────────────────────────┼──────────────────────────┤
│ Local NVMe Options         │     ✓ (ads variants)     │     ✓ (lssd variants)    │
│ Nested Virtualization      │            ✗             │            ✗             │
│ Reservations               │            ✗             │            ✗             │
└────────────────────────────┴──────────────────────────┴──────────────────────────┘
```

---

## Feature Differences

### Azure vs GCP Detailed Comparison

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                        AZURE vs GCP FEATURE DIFFERENCES                             │
├─────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                     │
│  AZURE ADVANTAGES                                                                   │
│  ────────────────                                                                   │
│  • Intel SGX enclave support (DCsv3 series) - smallest TCB option                   │
│  • Larger GPU memory (94 GB vs 80 GB HBM3)                                          │
│  • Unified attestation service (Azure Attestation)                                  │
│  • Confidential disk encryption with Azure Key Vault integration                    │
│  • Managed HSM with FIPS 140-2 Level 3 for CMK                                      │
│  • More regions with SEV-SNP support                                                │
│  • ARM template deployment support                                                  │
│                                                                                     │
│  GCP ADVANTAGES                                                                     │
│  ──────────────                                                                     │
│  • AMD SEV (basic) support - more machine series options                            │
│  • Live migration support (N2D with SEV only)                                       │
│  • Larger instance sizes (up to 360 vCPUs with C3D)                                 │
│  • More memory per instance (up to 2.88 TB with C3D)                                │
│  • Google Titanium integration for host verification                                │
│  • Custom machine type support (N2D)                                                │
│  • Signed UEFI firmware with verifiable measurements                                │
│  • AMD SEV on newer CPU generations (Genoa, Turin)                                  │
│                                                                                     │
│  COMMON LIMITATIONS                                                                 │
│  ──────────────────                                                                 │
│  • No VM reservations for confidential instances                                    │
│  • No nested virtualization support                                                 │
│  • Limited GPU TEE options (1 GPU max)                                              │
│  • No live migration for SNP/TDX                                                    │
│  • Extended shutdown times                                                          │
│  • kdump not supported                                                              │
│                                                                                     │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

### Memory Encryption Comparison

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                         MEMORY ENCRYPTION COMPARISON                                │
├─────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                     │
│  AMD SEV (GCP N2D, C2D, C3D, C4D)                                                   │
│  ────────────────────────────────                                                   │
│  • Memory encryption only (AES-128)                                                 │
│  • No memory integrity protection                                                   │
│  • No hardware attestation                                                          │
│  • Live migration supported (N2D only)                                              │
│  • Lower overhead                                                                   │
│                                                                                     │
│  AMD SEV-SNP (Azure DCasv5/ECasv5, GCP N2D)                                         │
│  ──────────────────────────────────────────                                         │
│  • Memory encryption (AES-256)                                                      │
│  • Memory integrity (RMP table)                                                     │
│  • Hardware attestation (VCEK signed)                                               │
│  • Anti-rollback and anti-replay                                                    │
│  • No live migration                                                                │
│                                                                                     │
│  Intel TDX (Azure DCesv5/ECesv5, GCP C3)                                            │
│  ───────────────────────────────────────                                            │
│  • Memory encryption (AES-XTS via TME-MK)                                           │
│  • Memory integrity (SEPT + MAC)                                                    │
│  • Hardware attestation (SGX Quoting Enclave)                                       │
│  • SEAM mode isolation                                                              │
│  • No live migration                                                                │
│                                                                                     │
│  SECURITY LEVEL RANKING                                                             │
│  ─────────────────────────                                                          │
│                                                                                     │
│  Highest ┌─────────────────────────────┐                                            │
│          │  Intel SGX (enclaves)       │  ← Smallest TCB (Azure only)               │
│          ├─────────────────────────────┤                                            │
│          │  AMD SEV-SNP / Intel TDX    │  ← Full VM protection + integrity          │
│          ├─────────────────────────────┤                                            │
│          │  AMD SEV                    │  ← Encryption only, no integrity           │
│  Lowest  └─────────────────────────────┘                                            │
│                                                                                     │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

### GPU TEE Comparison

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                            GPU TEE COMPARISON                                       │
├─────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                     │
│                    AZURE NCCadsH100v5          GCP a3-highgpu-1g                    │
│                    ─────────────────           ─────────────────                    │
│                                                                                     │
│  CPU TEE           AMD SEV-SNP                 Intel TDX                            │
│  CPU Model         AMD EPYC Genoa (4th)        Intel Sapphire Rapids (4th)          │
│  vCPUs             40                          26                                   │
│  System Memory     320 GiB                     234 GB                               │
│                                                                                     │
│  GPU Model         NVIDIA H100 NVL             NVIDIA H100 SXM                      │
│  GPU Memory        94 GB HBM3                  80 GB HBM3                           │
│  GPU Count         1                           1                                    │
│  GPU Link          PCIe Gen5                   NVLink + PCIe                        │
│  GPU Encryption    SPDM + AES-GCM              SPDM + AES-GCM                       │
│                                                                                     │
│  Local Storage     1x 960 GB NVMe              2x 375 GB Local SSD                  │
│  Network           80 Gbps                     200 Gbps                             │
│                                                                                     │
│  DIFFERENCES                                                                        │
│  ───────────                                                                        │
│  • Azure uses H100 NVL (94 GB) vs GCP H100 SXM (80 GB)                              │
│  • Azure has more vCPUs (40 vs 26) and memory (320 vs 234 GB)                       │
│  • GCP has higher network bandwidth (200 vs 80 Gbps)                                │
│  • GCP uses Intel TDX, Azure uses AMD SEV-SNP for CPU TEE                           │
│  • Both support NVIDIA Confidential Computing mode                                  │
│  • Both require combined CPU+GPU attestation                                        │
│                                                                                     │
│  ATTESTATION FLOW                                                                   │
│  ────────────────                                                                   │
│  Azure: AMD SP → VCEK Report + NVIDIA RoT → Combined Verification                   │
│  GCP:   TDX Module → SGX QE + NVIDIA RoT → Combined Verification                    │
│                                                                                     │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

---

## Instance Selection Guide

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                         INSTANCE SELECTION GUIDE                                    │
├─────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                     │
│  CHOOSE AMD SEV-SNP (Azure DCasv5/ECasv5, GCP N2D with SNP) WHEN:                   │
│  • You need memory integrity protection                                             │
│  • Hardware-rooted attestation is required                                          │
│  • You prefer AMD ecosystem                                                         │
│  • Maximum security without code changes                                            │
│                                                                                     │
│  CHOOSE INTEL TDX (Azure DCesv5/ECesv5, GCP C3) WHEN:                               │
│  • You need Intel-specific features (AMX for AI)                                    │
│  • Your workload is optimized for Intel                                             │
│  • You want SEAM-based isolation                                                    │
│  • Combined with GCP Confidential GPU                                               │
│                                                                                     │
│  CHOOSE AMD SEV ONLY (GCP C2D, C3D, C4D) WHEN:                                      │
│  • Memory encryption is sufficient                                                  │
│  • You need live migration (N2D only)                                               │
│  • Cost optimization is priority                                                    │
│  • Largest instance sizes needed (up to 360 vCPU)                                   │
│  • Lower overhead is required                                                       │
│                                                                                     │
│  CHOOSE GPU TEE WHEN:                                                               │
│  • AI/ML workloads with sensitive data                                              │
│  • Confidential model training/inference                                            │
│  • Healthcare or financial AI                                                       │
│  • Regulatory compliance requires GPU protection                                    │
│                                                                                     │
│  AZURE VS GCP DECISION                                                              │
│  ─────────────────────                                                              │
│  • Need SGX enclaves → Azure (DCsv3)                                                │
│  • Need live migration → GCP (N2D SEV)                                              │
│  • Need largest instances → GCP (C3D 360 vCPU)                                      │
│  • Need most GPU memory → Azure (94 GB vs 80 GB)                                    │
│  • Multi-cloud strategy → Both support similar workloads                            │
│                                                                                     │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

---

## References

- [Azure Confidential Computing Documentation](https://learn.microsoft.com/en-us/azure/confidential-computing/)
- [Azure VM Sizes - DC Family](https://learn.microsoft.com/en-us/azure/virtual-machines/sizes/general-purpose/dc-family)
- [Azure NCCadsH100v5 Series](https://learn.microsoft.com/en-us/azure/virtual-machines/sizes/gpu-accelerated/nccadsh100v5-series)
- [GCP Confidential VM Documentation](https://cloud.google.com/confidential-computing/confidential-vm/docs)
- [GCP Supported Configurations](https://cloud.google.com/confidential-computing/confidential-vm/docs/supported-configurations)
- [GCP GPU Machine Types](https://cloud.google.com/compute/docs/gpus)
- [AMD SEV-SNP Whitepaper](https://www.amd.com/system/files/TechDocs/SEV-SNP-strengthening-vm-isolation-with-integrity-protection-and-more.pdf)
- [Intel TDX Documentation](https://www.intel.com/content/www/us/en/developer/tools/trust-domain-extensions/documentation.html)
- [NVIDIA Confidential Computing](https://developer.nvidia.com/blog/confidential-computing-on-h100-gpus-for-secure-and-trustworthy-ai/)

---

## License

This documentation is provided for educational and informational purposes. Confidential computing capabilities and instance type specifications are based on publicly available documentation from AMD, Intel, NVidia, Microsoft Azure, and Google Cloud Platform.

This project documentation is licensed under the **Apache 2.0 License**. See the [`LICENSE-APACHE`](LICENSE-APACHE) file for the details.

---

*Last Updated: January 2026*
