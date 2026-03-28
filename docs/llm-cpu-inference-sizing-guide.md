# LLM Inference Acceleration on CPUs: A Comprehensive Sizing & Performance Guide

**Covering Llama 70B, Llama 405B, DeepSeek R1 671B, GPT-OSS 120B, and more — across SIMD-optimized CPU, APU, and GPU platforms**

---

## 1. Introduction

Running large language models (70B+ parameters) locally or on-premise is increasingly attractive for latency, privacy, and cost reasons. While GPUs dominate inference throughput, CPU-based inference — leveraging modern SIMD instruction set extensions like AVX-512, AMX, and ARM NEON — has matured into a viable option for certain deployment scenarios.

This guide provides a complete sizing model for LLM inference across hardware classes, covering the fundamental physics of why memory bandwidth is the bottleneck, what SIMD instructions actually buy you, real-world benchmark data, and projected performance across hardware tiers from consumer desktops to data center GPU clusters.

---

## 2. The Fundamental Bottleneck: Memory Bandwidth, Not Compute

LLM token generation (the "decode" phase where the model produces output one token at a time) is **memory-bandwidth-bound**, not compute-bound. For each token generated, the inference engine must read essentially the entire model weights from memory. This makes the governing equation deceptively simple:

```
Token Generation Rate ≈ Memory Bandwidth (GB/s) ÷ Model Size (GB) × Efficiency Factor
```

For example, a Llama 70B model quantized to Q4_K_M (~40 GB) running on hardware with 300 GB/s memory bandwidth at 65% efficiency yields:

```
300 ÷ 40 × 0.65 ≈ 4.9 tokens/second
```

This is why a consumer desktop with dual-channel DDR5 (~80 GB/s) generates tokens slowly for large models regardless of CPU clock speed, while server-class EPYC processors with 8–12 memory channels (~300–460 GB/s) perform considerably better, and GPUs with HBM (~2,000–8,000 GB/s) are dramatically faster.

**Prompt processing (prefill)** is the exception — it is compute-bound because many tokens are processed in parallel. This is where SIMD truly shines, delivering 3–10× higher throughput than token generation on CPUs.

---

## 3. Memory Hierarchy — Bandwidth vs Capacity Tradeoff

Every hardware platform sits on a curve trading bandwidth for capacity. LLM inference needs both: enough capacity to hold the model, and enough bandwidth to stream it fast enough for interactive use.

```
┌─────────────────────────────────────────────────────────────────────┐
│                   MEMORY HIERARCHY PYRAMID                          │
│                                                                     │
│          ┌──────────┐                                               │
│          │ SRAM /   │  ~10 TB/s    < 100 MB    < 1 ns               │
│          │ L3 Cache │                                               │
│         ┌┴──────────┴┐                                              │
│         │  HBM3e     │  ~8 TB/s     192 GB      ~100 ns             │
│         │  (B200)    │                                              │
│        ┌┴────────────┴┐                                             │
│        │  HBM3 (H100) │  ~3.4 TB/s  80 GB       ~100 ns             │
│       ┌┴──────────────┴┐                                            │
│       │ GDDR7 (RTX5090)│  ~1.8 TB/s  32 GB      ~200 ns             │
│      ┌┴────────────────┴┐                                           │
│      │ GDDR6X (RTX4090) │  ~1 TB/s   24 GB      ~300 ns             │
│     ┌┴──────────────────┴┐                                          │
│     │Unified LPDDR5X     │  ~819 GB/s  256 GB    ~80 ns             │
│     │(Apple M4 Ultra)    │                                          │
│    ┌┴────────────────────┴┐                                         │
│    │ DDR5 8-Ch Server     │  ~310 GB/s  1.5 TB   ~60 ns             │
│   ┌┴──────────────────────┴┐                                        │
│   │DDR5 Dual-Ch Desktop    │  ~80 GB/s   128 GB  ~60 ns             │
│  ┌┴────────────────────────┴┐                                       │
│  │ NVMe SSD (offload)       │  ~7 GB/s    8+ TB  ~10 µs             │
│  └──────────────────────────┘                                       │
│                                                                     │
│  ▲ Higher Bandwidth, Lower Capacity                                 │
│  ▼ Lower Bandwidth, Higher Capacity                                 │
└─────────────────────────────────────────────────────────────────────┘
```

**Key insight**: GPUs (especially with HBM) provide enormous bandwidth but limited capacity. A single RTX 4090 (24 GB) cannot hold a 70B Q4 model (~40 GB). Server CPUs provide the capacity but lack the bandwidth. Apple's unified memory architecture uniquely combines moderate bandwidth with large capacity in a single coherent address space.

### Memory Type Comparison Table

| Memory Type | Bandwidth | Max Capacity (single device) | Unified? | Coherent? | Typical Platform |
|---|---|---|---|---|---|
| DDR5-6400 (2-ch) | ~80 GB/s | 128 GB | No | Yes | Consumer desktop |
| DDR5-6400 (8-ch) | ~310 GB/s | 1.5 TB | No | Yes | Server (EPYC/Xeon) |
| MRDIMM-8800 (8-ch) | ~560 GB/s | 4 TB | No | Yes | Intel Xeon 6 server |
| LPDDR5X (Apple) | 410–819 GB/s | 128–256 GB | Yes | Yes | M4 Max / Ultra |
| LPDDR5X (Strix Halo) | ~256 GB/s | 128 GB | Yes | Yes | AMD Ryzen AI Max+ |
| GDDR6X | ~1,008 GB/s | 24 GB | No | No | RTX 4090 |
| GDDR7 | ~1,792 GB/s | 32 GB | No | No | RTX 5090 |
| HBM2e | ~2,039 GB/s | 80 GB | No | No | A100 |
| HBM3 | ~3,350 GB/s | 80 GB | No | No | H100 SXM |
| HBM3e | 4,800–8,000 GB/s | 141–192 GB | No | No | H200 / B200 |

---

## 4. SIMD Instruction Sets and Their Impact on Inference

Modern CPUs include vector processing extensions that accelerate the matrix multiplications at the heart of LLM inference. Here is a summary of the relevant instruction sets and their practical impact.

### 4.1 x86 SIMD Extensions

| ISA Extension | Vector Width | Key Operations | Availability | Impact on LLM Inference |
|---|---|---|---|---|
| SSE/SSE2/SSE3/SSE4 | 128-bit | FP32 SIMD, basic vectorization | All modern x86 | Baseline; ~1× reference |
| AVX / AVX2 | 256-bit | FP32/INT8 fused multiply-add | Intel Haswell+ (2013), AMD Zen+ | ~1.5–2× over SSE for matmul |
| AVX-512 | 512-bit | FP32/FP16/INT8, mask registers | Intel Skylake-X+, AMD Zen 4+ | ~1.3–2× over AVX2 for quantized inference |
| AVX-512 VNNI | 512-bit | INT8/INT16 dot product | Intel Ice Lake+, AMD Zen 4+ | Accelerates INT4/INT8 dequantization kernels |
| AVX-512 BF16 | 512-bit | BF16 dot product | Intel Cooper Lake+, Zen 5 | Native bfloat16 support for BF16 models |
| AMX (Advanced Matrix Extensions) | Tile-based (1024B) | BF16/INT8 tile matmul | Intel Sapphire Rapids+ | Up to 3× over AVX-512 for high-ARI workloads; 5.4 TFLOPS vs 1.8 TFLOPS (AVX-512) |
| AVX10 | 256/512-bit convergence | Unified AVX-512 on all cores | Intel Granite Rapids+ | Brings AVX-512 to E-cores; wider availability |

### 4.2 ARM SIMD Extensions

| ISA Extension | Vector Width | Key Operations | Availability | Impact on LLM Inference |
|---|---|---|---|---|
| NEON | 128-bit | FP32/FP16/INT8 SIMD | All ARMv8+ (Apple M-series, Cortex-A) | Baseline for ARM inference |
| SVE (Scalable Vector Extension) | 128–2048 bit (variable) | Predicated vector ops, FP16/INT8 | ARMv9 Cortex-A510+, Neoverse V2 | Significant gains on server ARM (Graviton 4, Axion) |
| Apple AMX | Proprietary matrix coprocessor | Matrix multiply acceleration | Apple M1+ | Major prefill speedup; integrated into llama.cpp via Accelerate |
| Arm Kleidi | Software optimization library | Optimized GEMM kernels for NEON/SVE | Neoverse V2 (Google Axion) | Measurable gains in llama.cpp for 70B inference |

### 4.3 What SIMD Actually Buys You

The impact of SIMD differs dramatically between the two phases of inference:

**Prompt Processing (Prefill)** — Compute-bound, benefits enormously from SIMD:
- Justine Tunney's llamafile project wrote 84 new SIMD matmul kernels and achieved a **2× speedup** on Skylake CPUs for llama.cpp prompt processing.
- On IBM Z mainframes, SIMD routines improved prompt processing by **38%** and token generation by **163%** compared to scalar code.
- Intel AMX achieves **5.4 TFLOPS** for MoE expert computation vs **1.8 TFLOPS** with AVX-512 alone — a 3× improvement. However, this still reaches only ~7% of AMX's theoretical peak (73.7 TFLOPS) due to memory bandwidth constraints.
- KTransformers with AMX optimizations pushed DeepSeek R1 671B prefill from 54 tok/s (32 cores, AVX-512) to **286 tok/s** (AMX + expert reduction) — a 5.3× improvement.

**Token Generation (Decode)** — Memory-bandwidth-bound, SIMD helps modestly:
- SIMD accelerates the dequantization step (unpacking Q4/Q8 values to FP32 for computation), providing a meaningful but smaller improvement.
- The primary benefit is reducing CPU cycles per byte of model data streamed from memory, keeping the compute pipeline from becoming the bottleneck even as bandwidth saturates.
- Typical SIMD benefit for token generation: **1.3–2×** improvement over scalar code, compared to 2–5× for prefill.

```
┌─────────────────────────────────────────────────────────────────┐
│            SIMD IMPACT BY INFERENCE PHASE                       │
│                                                                 │
│  Prompt Processing (Prefill)         Token Generation (Decode)  │
│  ┌─────────────────────────┐        ┌─────────────────────────┐ │
│  │ ████████████████████ 5× │        │ ████████ 2×             │ │
│  │ AMX vs scalar           │        │ AMX vs scalar           │ │
│  ├─────────────────────────┤        ├─────────────────────────┤ │
│  │ ████████████████ 3×     │        │ ██████ 1.5×             │ │
│  │ AVX-512 vs scalar       │        │ AVX-512 vs scalar       │ │
│  ├─────────────────────────┤        ├─────────────────────────┤ │
│  │ ██████████ 2×           │        │ █████ 1.3×              │ │
│  │ AVX2 vs scalar          │        │ AVX2 vs scalar          │ │
│  └─────────────────────────┘        └─────────────────────────┘ │
│                                                                 │
│  Bottleneck: COMPUTE                Bottleneck: MEMORY BW       │
│  SIMD has high leverage             SIMD has moderate leverage  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 5. Model Size Reference — Total and Active Parameters

Not all large models are equally expensive to run. Mixture-of-Experts (MoE) architectures like DeepSeek R1 activate only a fraction of their parameters per token, dramatically reducing bandwidth requirements.

| Model | Architecture | Total Params | Active Params/Token | Active % | Size (Q4) | Size (FP16) | Effective BW per Token (Q4) |
|---|---|---|---|---|---|---|---|
| Llama 3.3 70B | Dense | 70B | 70B | 100% | ~40 GB | ~140 GB | 40 GB |
| GPT-OSS 120B | Dense | 120B | 120B | 100% | ~68 GB | ~240 GB | 68 GB |
| Llama 3.1 405B | Dense | 405B | 405B | 100% | ~230 GB | ~810 GB | 230 GB |
| DeepSeek R1 671B | MoE (256×8) | 671B | ~37B | 5.5% | ~377 GB | ~1,340 GB | ~25 GB |

**Key insight**: Despite having 671B total parameters, DeepSeek R1's effective bandwidth cost per token (~25 GB at Q4) is actually *less* than Llama 70B's (~40 GB) because the MoE architecture activates only 8 of 256 experts per token plus shared attention layers. This makes it surprisingly CPU-friendly — the entire model must fit in memory, but the per-token bandwidth demand is low.

```
┌──────────────────────────────────────────────────────────────────────┐
│            TOTAL vs ACTIVE PARAMETERS                                │
│                                                                      │
│  Llama 70B     ████████████████████████████████████████ 70B (100%)   │
│  (Dense)       ████████████████████████████████████████ 70B active   │
│                                                                      │
│  GPT-OSS 120B  ████████████████████████████████████████████████████  │
│  (Dense)       ████████████████████████████████████████████████████  │
│                120B total = 120B active (100%)                       │
│                                                                      │
│  Llama 405B    ████████████████████████████████████████████████████  │
│  (Dense)       ████████████████████████████████████████████████████  │
│                █████████████████████████████████████████████         │
│                405B total = 405B active (100%)                       │
│                                                                      │
│  DeepSeek R1   ████████████████████████████████████████████████████  │
│  671B (MoE)    ████████████████████████████████████████████████████  │
│                █████████████████████████████████████████████████████ │
│                671B total                                            │
│                ███ 37B active (5.5%)                                 │
│                                                                      │
│  ■ Total params  ■ Active params per token                           │
└──────────────────────────────────────────────────────────────────────┘
```

---

## 6. Hardware Platform Comparison

### 6.1 CPU-Only Platforms

| Platform | Chip | Memory Config | Bandwidth | Capacity | SIMD / Accel | Best For |
|---|---|---|---|---|---|---|
| Consumer Desktop | Ryzen 9 9950X / i9-14900K | DDR5-6400 Dual-Channel | ~80 GB/s | 128 GB | AVX-512 / AVX2 | Small models (≤13B), development |
| HEDT / Workstation | Threadripper 7980X | DDR5-5600 Quad-Channel | ~180 GB/s | 512 GB | AVX-512 | 70B Q4 at marginal speed |
| Server 1-Socket | EPYC 9555 (64c) | DDR5-6400 8-Channel | ~310 GB/s | 1.5 TB | AVX-512 / VNNI | 70B at usable speed, 405B Q4 possible |
| Server 2-Socket | 2× EPYC 9654 (192c total) | DDR5-4800 24-Channel | ~460 GB/s | 6 TB | AVX-512 / VNNI | 405B Q4, DeepSeek R1 671B |
| Server 2-Socket (AMX) | 2× Xeon 6980P (256c total) | MRDIMM-8800 16-Channel | ~560 GB/s | 4 TB | AMX / AVX-512 | Best CPU-only prefill, 671B MoE |

### 6.2 Unified Memory / APU Platforms

| Platform | Chip | Memory Config | Bandwidth | Capacity | SIMD / Accel | Best For |
|---|---|---|---|---|---|---|
| Apple M4 Max | M4 Max (40-core GPU) | Unified LPDDR5X | ~410 GB/s | 128 GB | NEON + Apple AMX | 70B Q4 at ~7–10 tok/s, excellent efficiency |
| Apple M4 Ultra | M4 Ultra (80-core GPU) | Unified LPDDR5X | ~819 GB/s | 256 GB | NEON + Apple AMX | 70B Q8, 120B Q4; best single-box experience |
| AMD Strix Halo | Ryzen AI Max+ 395 | LPDDR5X-8000 8-Channel | ~256 GB/s | 128 GB | AVX-512 + XDNA2 NPU (50 TOPS) | 70B Q4 portable; emerging NPU support |

### 6.3 GPU Platforms

| Platform | Chip | Memory Type | Bandwidth | VRAM | Tensor Accel | Best For |
|---|---|---|---|---|---|---|
| Consumer GPU | RTX 4090 | GDDR6X | ~1,008 GB/s | 24 GB | FP8 Tensor Cores | ≤13B models at extreme speed |
| Consumer GPU (Next) | RTX 5090 | GDDR7 | ~1,792 GB/s | 32 GB | FP4 Tensor Cores | ≤20B at extreme speed, 70B partially |
| Data Center | A100 80GB | HBM2e | ~2,039 GB/s | 80 GB | FP16 Tensor Cores | 70B FP16, 405B Q4 (multi-GPU) |
| Data Center | H100 SXM | HBM3 | ~3,350 GB/s | 80 GB | FP8 Tensor Cores | 70B at 40+ tok/s |
| Data Center | H200 SXM | HBM3e | ~4,800 GB/s | 141 GB | FP8 Tensor Cores | 70B FP16, 405B Q4 (2×) |
| Data Center (Next) | B200 | HBM3e | ~8,000 GB/s | 192 GB | FP4 Tensor Cores | 70B at 100+ tok/s, 405B (2×) |
| Multi-GPU | 4× H100 NVLink | HBM3 | ~13,400 GB/s | 320 GB | FP8 Tensor Cores | 405B Q4 at high speed |
| Multi-GPU | 8× H200 (DGX) | HBM3e | ~38,400 GB/s | 1,128 GB | FP8 Tensor Cores | 671B FP16, 405B at extreme speed |

---

## 7. Performance Sizing Matrix — Token Generation (tok/s)

The following table shows **estimated token generation speed** (single-stream, batch=1) across hardware and models. These are derived from the bandwidth model with efficiency factors calibrated against published benchmarks.

### 7.1 Llama 3.3 70B (Dense, 70B active)

| Hardware | Q4_K_M (~40 GB) | Q8_0 (~80 GB) | FP16 (~140 GB) |
|---|---|---|---|
| Consumer Desktop (DDR5 2-ch, 80 GB/s) | **1.3 tok/s** | 0.6 tok/s | — (won't fit 128 GB) |
| HEDT Threadripper (DDR5 4-ch, 180 GB/s) | **2.9 tok/s** | 1.5 tok/s | 0.8 tok/s |
| Server EPYC 1S (DDR5 8-ch, 310 GB/s) | **5.0 tok/s** | 2.5 tok/s | 1.4 tok/s |
| Server EPYC 2S (DDR5 24-ch, 460 GB/s) | **7.5 tok/s** | 3.7 tok/s | 2.1 tok/s |
| Server Xeon 2S AMX (MRDIMM, 560 GB/s) | **9.8 tok/s** | 4.9 tok/s | 2.8 tok/s |
| Apple M4 Max (Unified, 410 GB/s) | **7.4 tok/s** | 3.7 tok/s | — (128 GB limit) |
| Apple M4 Ultra (Unified, 819 GB/s) | **14.7 tok/s** | 7.4 tok/s | 4.2 tok/s |
| AMD Strix Halo (LPDDR5X, 256 GB/s) | **4.6 tok/s** | 2.3 tok/s | — (128 GB limit) |
| RTX 4090 (GDDR6X, 1008 GB/s, 24 GB) | — (won't fit) | — | — |
| RTX 5090 (GDDR7, 1792 GB/s, 32 GB) | — (won't fit) | — | — |
| H100 SXM (HBM3, 3350 GB/s, 80 GB) | **71 tok/s** | 36 tok/s | — (won't fit 80 GB) |
| H200 SXM (HBM3e, 4800 GB/s, 141 GB) | **102 tok/s** | 51 tok/s | 29 tok/s |
| 4× H100 NVLink (13,400 GB/s, 320 GB) | **261 tok/s** | 131 tok/s | 75 tok/s |

*Real-world validated reference points: Apple M4 Max ~7 tok/s (community benchmarks); EPYC Genoa single-socket FP16 ~2.3 tok/s (llama.cpp bench); ARM Neoverse V2 (Google Axion) ~50 tok/s prefill.*

### 7.2 DeepSeek R1 671B (MoE, ~37B active, ~25 GB effective BW/token at Q4)

| Hardware | Q4_K_M (~377 GB total, ~25 GB active) | Q8_0 (~754 GB total) |
|---|---|---|
| Consumer Desktop (80 GB/s, 128 GB) | — (won't fit) | — |
| HEDT Threadripper (180 GB/s, 512 GB) | **4.7 tok/s** | — (won't fit) |
| Server EPYC 1S (310 GB/s, 1.5 TB) | **8.1 tok/s** | 4.0 tok/s |
| Server EPYC 2S (460 GB/s, 6 TB) | **12.0 tok/s** | 6.0 tok/s |
| Server Xeon 2S AMX (560 GB/s, 4 TB) | **15.7 tok/s** | 7.8 tok/s |
| Apple M4 Ultra (819 GB/s, 256 GB) | — (won't fit 377 GB) | — |
| H200 SXM (4800 GB/s, 141 GB) | — (won't fit single GPU) | — |
| 4× H100 NVLink (13,400 GB/s, 320 GB) | — (won't fit 377 GB) | — |
| 8× H200 DGX (38,400 GB/s, 1,128 GB) | **1,306 tok/s** (theoretical) | 408 tok/s |

*Real-world validated reference points: Dual Xeon 6980P with llama.cpp ~6–8 tok/s (Q4_K_M); KTransformers with AMX ~10–14 tok/s; consumer gaming rig with NVMe offload ~1–3.5 tok/s.*

### 7.3 Llama 3.1 405B (Dense, 405B active)

| Hardware | Q4_K_M (~230 GB) | FP16 (~810 GB) |
|---|---|---|
| Consumer Desktop (80 GB/s, 128 GB) | — (won't fit) | — |
| Server EPYC 1S (310 GB/s, 1.5 TB) | **0.9 tok/s** | 0.2 tok/s |
| Server EPYC 2S (460 GB/s, 6 TB) | **1.3 tok/s** | 0.4 tok/s |
| Server Xeon 2S AMX (560 GB/s, 4 TB) | **1.7 tok/s** | 0.5 tok/s |
| Apple M4 Ultra (819 GB/s, 256 GB) | **2.3 tok/s** | — (won't fit) |
| 4× H100 NVLink (13,400 GB/s, 320 GB) | **45 tok/s** | — (won't fit) |
| 8× H200 DGX (38,400 GB/s, 1,128 GB) | **130 tok/s** | 37 tok/s |

*Real-world reference: Cerebras Inference (custom wafer-scale engine) achieved 969 tok/s on Llama 405B — demonstrating the ceiling with purpose-built hardware.*

### 7.4 GPT-OSS 120B (Dense, 120B active)

| Hardware | Q4_K_M (~68 GB) | Q8_0 (~136 GB) |
|---|---|---|
| Consumer Desktop (80 GB/s, 128 GB) | **0.8 tok/s** | — (won't fit) |
| Server EPYC 1S (310 GB/s, 1.5 TB) | **3.0 tok/s** | 1.5 tok/s |
| Server EPYC 2S (460 GB/s, 6 TB) | **4.4 tok/s** | 2.2 tok/s |
| Apple M4 Ultra (819 GB/s, 256 GB) | **8.7 tok/s** | 4.3 tok/s |
| H100 SXM (3350 GB/s, 80 GB) | **42 tok/s** | — (won't fit) |
| H200 SXM (4800 GB/s, 141 GB) | **60 tok/s** | 30 tok/s |

---

## 8. Prompt Processing (Prefill) Performance

Prefill is compute-bound and benefits heavily from SIMD. The following table shows **estimated prefill throughput** for representative configurations.

| Hardware | Llama 70B Q4 | DeepSeek R1 671B Q4 | Llama 405B Q4 |
|---|---|---|---|
| Server EPYC 1S (AVX-512) | ~20–40 tok/s | ~30–50 tok/s | ~4–8 tok/s |
| Server Xeon 2S (AMX) | ~40–90 tok/s | ~255–287 tok/s (KTransformers) | ~8–15 tok/s |
| Apple M4 Ultra (NEON + AMX) | ~50–80 tok/s | — | ~10–15 tok/s |
| ARM Neoverse V2 / Axion (SVE) | ~50 tok/s | — | — |
| H100 SXM | ~2,000+ tok/s | ~1,000+ tok/s | ~500+ tok/s |
| 8× H200 DGX | ~10,000+ tok/s | ~5,000+ tok/s | ~2,000+ tok/s |

**Notable result**: KTransformers with Intel AMX achieved 286 tok/s prefill for DeepSeek R1 671B on dual-socket Xeon — a 28× speedup over llama.cpp's 10.3 tok/s on the same hardware. This demonstrates the enormous potential of hardware-aware SIMD kernel optimization for compute-bound workloads.

---

## 9. NUMA Topology and Multi-Socket Challenges

Multi-socket CPU inference introduces NUMA (Non-Uniform Memory Access) complications that significantly impact token generation:

- **Prompt processing** scales well across sockets (embarrassingly parallel matrix operations).
- **Token generation** scales poorly — cross-socket memory access adds latency and reduces effective bandwidth.
- On dual EPYC Genoa/Turin systems, token generation showed only moderate scaling (and sometimes regression) compared to single-socket, even though aggregate bandwidth doubled.
- Setting AMD EPYC to **NPS0** mode (single flat NUMA domain) helps by avoiding cross-NUMA penalties.
- Intel Xeon users should set **SNC=Disable** for one NUMA node per socket.
- For MoE models like DeepSeek R1, the problem is worse because the small active matrices (~7 MB at Q4) have a very high synchronization-to-compute ratio across NUMA nodes.

```
┌──────────────────────────────────────────────────────────────────┐
│          NUMA SCALING FOR TOKEN GENERATION                       │
│                                                                  │
│  ┌───────────────────┐    ┌──────────────────┐                   │
│  │    Socket 0       │    │    Socket 1      │                   │
│  │  ┌──────────────┐ │    │ ┌──────────────┐ │                   │
│  │  │ Local DRAM   │ │◄──►│ │ Local DRAM   │ │                   │
│  │  │ ~230 GB/s    │ │slow│ │ ~230 GB/s    │ │                   │
│  │  └──────────────┘ │link│ └──────────────┘ │                   │
│  │  Cores 0-95       │    │  Cores 96-191    │                   │
│  └───────────────────┘    └──────────────────┘                   │
│                                                                  │
│  Dense 70B:  1 socket: 2.3 t/s  →  2 sockets: 2.3 t/s (no gain)  │
│  MoE 671B:   1 socket: 6.4 t/s  →  2 sockets: 7.8 t/s (modest)   │
│  Prefill:    1 socket: 21 t/s   →  2 sockets: 42 t/s  (good!)    │
└──────────────────────────────────────────────────────────────────┘
```

---

## 10. Software Stack Comparison

The choice of inference engine matters significantly, especially for CPU-optimized paths.

| Engine | CPU Optimization | Key SIMD Features | Best Use Case |
|---|---|---|---|
| **llama.cpp** | Excellent | AVX2, AVX-512, VNNI, BF16, NEON, SVE, AMX (basic) | General-purpose; broadest hardware support |
| **llamafile** | Excellent | Hand-tuned SIMD matmul (84 kernels); 2× over llama.cpp on some CPUs | Single-binary deployment; Cosmopolitan libc |
| **KTransformers** | Excellent (MoE) | AMX tile-aware kernels, NUMA-aware placement | DeepSeek R1/V3 hybrid CPU+GPU inference |
| **vLLM** | Good | PyTorch CPU backend, ZenDNN (AMD) | Server deployment with batching; GPU-primary |
| **Intel Neural Compressor** | Good | AVX-512 VNNI, AMX; INT4 quantization | Intel-optimized INT4 inference |
| **ONNX Runtime** | Good | AVX-512, VNNI, operator fusion | Cross-platform; model conversion |
| **T-MAC** | Specialized | Lookup-table based; avoids multiply-add | Ultra-low-bit (1–2 bit) models on CPU |

---

## 11. GPU VRAM Scaling Roadmap

A key limitation of GPUs is VRAM capacity — even an H100 at 80 GB cannot hold a 405B Q4 model on a single chip. This is changing rapidly.

| Generation | Year | Chip | Memory Type | VRAM | Bandwidth | 70B Q4 Fits? | 405B Q4 Fits? | 671B Q4 Fits? |
|---|---|---|---|---|---|---|---|---|
| Ada Lovelace | 2022 | RTX 4090 | GDDR6X | 24 GB | 1,008 GB/s | No | No | No |
| Blackwell (Consumer) | 2025 | RTX 5090 | GDDR7 | 32 GB | 1,792 GB/s | No | No | No |
| Blackwell (DC) | 2025 | B200 | HBM3e | 192 GB | 8,000 GB/s | Yes | No | No |
| Rubin (DC, projected) | 2026–27 | R100 | HBM4 | ~288 GB | ~12,000 GB/s | Yes | Yes (2×) | No |
| Consumer (projected) | 2027+ | RTX 6090 | GDDR7+ | ~48 GB | ~2,400 GB/s | Yes (Q4) | No | No |
| Rubin Ultra (projected) | 2027+ | R200 | HBM4 | ~384 GB | ~16,000 GB/s | Yes | Yes | Yes (2×) |

**Trend**: Each GPU generation roughly doubles bandwidth and increases VRAM by 30–50%. By 2027–2028, a single data center GPU may hold a 405B Q4 model, and 2 GPUs may hold DeepSeek R1 671B Q4 — dramatically simplifying deployment.

---

## 12. Decision Framework — Choosing Your Hardware

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     HARDWARE SELECTION FLOWCHART                        │
│                                                                         │
│  What model are you running?                                            │
│  │                                                                      │
│  ├─ ≤13B ──► Consumer GPU (RTX 4090/5090) ──► 50-150+ tok/s             │
│  │           Consumer CPU is fine too ──► 10-30 tok/s                   │
│  │                                                                      │
│  ├─ 70B ──► Does it need to be fast (>20 tok/s)?                        │
│  │          │                                                           │
│  │          ├─ Yes ──► H100/H200 single GPU ──► 70-100+ tok/s           │
│  │          │                                                           │
│  │          ├─ Moderate (5-15 tok/s) ──► Apple M4 Ultra ──► 8-15 tok/s  │
│  │          │                            Server EPYC 1S ──► 5-10 tok/s  │
│  │          │                                                           │
│  │          └─ Budget/usable (2-5 tok/s) ──► HEDT or EPYC 1S            │
│  │                                                                      │
│  ├─ 120B ──► Apple M4 Ultra (Q4) ──► ~9 tok/s                           │
│  │           Server EPYC 2S (Q4) ──► ~4 tok/s                           │
│  │           H200 single GPU (Q4) ──► ~60 tok/s                         │
│  │                                                                      │
│  ├─ 405B ──► Must use multi-GPU or large-memory CPU server              │
│  │           4× H100 NVLink (Q4) ──► ~45 tok/s                          │
│  │           EPYC 2S (Q4, barely usable) ──► ~1.3 tok/s                 │
│  │                                                                      │
│  └─ 671B MoE ──► Surprisingly CPU-friendly due to MoE                   │
│                  Dual Xeon AMX + KTransformers ──► ~10-14 tok/s         │
│                  Dual EPYC + llama.cpp ──► ~6-8 tok/s                   │
│                  8× H200 DGX (if budget allows) ──► 500+ tok/s          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 13. Key Takeaways

1. **Memory bandwidth is the ceiling**. No amount of SIMD optimization can overcome the fundamental limit of how fast you can stream model weights from memory. Invest in memory channels and bandwidth first.

2. **SIMD matters most for prefill**. AVX-512 and especially Intel AMX provide 2–5× improvements for prompt processing. For token generation, the gains are a more modest 1.3–2×, but still meaningful.

3. **MoE models are the CPU-inference sweet spot**. DeepSeek R1 671B activates only ~37B parameters per token, meaning its per-token bandwidth cost is lower than a 70B dense model despite being nearly 10× larger total. Dual-socket servers can achieve usable 6–14 tok/s.

4. **Apple Silicon is uniquely positioned** for 70B-class models. The M4 Ultra's 819 GB/s unified memory bandwidth with 256 GB capacity hits a sweet spot that no other single consumer device matches.

5. **Consumer GPUs have a capacity problem, not a bandwidth problem**. An RTX 5090 has 1.8 TB/s bandwidth but only 32 GB VRAM. Future generations (GDDR7+, HBM4) will gradually close this gap.

6. **Software optimization has enormous headroom**. KTransformers achieved 28× speedup over baseline llama.cpp for DeepSeek R1 prefill through AMX-aware kernels and NUMA-aware placement. The inference engine stack is still rapidly maturing.

7. **NUMA is a trap for token generation**. Adding a second CPU socket doubles aggregate bandwidth on paper but often provides zero improvement for single-stream decode due to cross-socket latency. Use NPS0 (AMD) or SNC=Disable (Intel).

8. **The practical usability threshold is ~5 tok/s** for interactive use. Below this, the experience becomes frustrating. For 70B+ models on CPU, achieving this requires at minimum a server-class platform with 4+ memory channels.

---

## 14. Methodology & Disclaimers

Performance estimates in this guide are derived from:

- The theoretical model: `tok/s = bandwidth / model_size_in_memory × efficiency_factor`
- Efficiency factors calibrated against published benchmarks: CPU ~65%, APU ~72%, GPU ~85%, Multi-GPU ~78%
- Real-world data points from llama.cpp benchmarks, KTransformers publications, Arm/Google Axion benchmarks, community reports, and academic literature
- All estimates assume single-stream, batch=1, default context length inference
- Actual performance varies with: NUMA configuration, specific quantization kernels, KV-cache memory overhead, context length, software version, thermal throttling, and memory population (number of DIMMs per channel)
- Prefill estimates are rougher than generation estimates due to high sensitivity to compute utilization
- GPU multi-chip estimates assume NVLink interconnect; PCIe-connected multi-GPU will be significantly slower
- Projected future hardware specs are based on public roadmaps and analyst estimates; actual products may differ

---

*Last updated: March 2026. Data synthesized from llama.cpp benchmarks, KTransformers publications, Arm Neoverse benchmarks, Cerebras announcements, arxiv:2410.04466 (LLM Inference Acceleration survey), community benchmarks on OpenBenchmarking.org, and manufacturer specifications.*
