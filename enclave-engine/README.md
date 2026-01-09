# Enclave Engine

A comprehensive Rust service for provisioning and managing confidential computing enclaves with GPU TEE support. Supports both KVM/QEMU-based CVMs (Intel TDX, AMD SEV-SNP) and AWS Nitro Enclaves.

## Features

- **Multi-Backend Support**
  - KVM/QEMU with Intel TDX and AMD SEV/SEV-SNP
  - AWS Nitro Enclaves
  
- **GPU TEE Support**
  - NVIDIA H100/H200 GPU passthrough
  - AMD GPU passthrough
  - Multi-GPU configuration support (future-ready)
  
- **Advanced Memory Management**
  - NUMA node configuration with `numactl`
  - Hugepages allocation (2MB and 1GB pages)
  - Memory bank and CPU/GPU allocation
  
- **RESTful API**
  - Multi-threaded Tokio runtime
  - Dynamic configuration via YAML
  - Enclave lifecycle management

## Prerequisites

### System Requirements

```bash
# Install QEMU/KVM
sudo apt-get install qemu-system-x86 qemu-kvm libvirt-daemon-system

# Install NUMA tools
sudo apt-get install numactl libnuma-dev

# Install hugepages tools
sudo apt-get install libhugetlbfs-bin

# For AWS Nitro Enclaves
sudo amazon-linux-extras install aws-nitro-enclaves-cli
sudo yum install aws-nitro-enclaves-cli-devel
```

### VFIO GPU Passthrough Setup

```bash
# Enable IOMMU in GRUB
sudo vim /etc/default/grub
# Add: GRUB_CMDLINE_LINUX="intel_iommu=on iommu=pt"
# Or for AMD: GRUB_CMDLINE_LINUX="amd_iommu=on iommu=pt"
sudo update-grub

# Bind GPU to VFIO driver
echo "vfio-pci" | sudo tee /etc/modules-load.d/vfio-pci.conf
echo "options vfio-pci ids=10de:2330" | sudo tee /etc/modprobe.d/vfio.conf
# Replace 10de:2330 with your GPU's PCI ID

sudo update-initramfs -u
sudo reboot
```

## Building

```bash
# Clone repository
git clone https://github.com/sentient-agi/Sentient-Enclaves-Framework.git
cd Sentient-Enclaves-Framework/enclave-engine/

# Build
cargo build --release

# Run
sudo ./target/release/enclave-engine
```

## Configuration

Create a `config.yaml` file based on the example:

```yaml
general:
  name: secure-enclave
  backend: qemu

qemu:
  vm:
    name: confidential-vm
    memory: 16384
    cpus: 8
    disk: /path/to/disk.qcow2
    kernel: /path/to/kernel
    initrd: /path/to/initrd
    cmdline: console=ttyS0
    qemu_binary: /usr/bin/qemu-system-x86_64

  confidential:
    technology: amd-sev-snp
    firmware: /usr/share/OVMF/OVMF_CODE.fd
    
  gpu:
    enable: true
    vendor: nvidia
    devices:
      - "0000:0a:00.0"
      - "0000:0b:00.0"

numa:
  enable: true
  nodes:
    - node_id: 0
      cpus: [0, 1, 2, 3]
      memory_gb: 32
      gpus: ["0000:0a:00.0"]

hugepages:
  enable: true
  page_size_kb: 2048
  num_pages: 1024
```

## API Usage

### Provision an Enclave

```bash
curl -X POST http://localhost:8080/enclaves \
  -H "Content-Type: application/json" \
  -d @config.yaml
```

### List Enclaves

```bash
curl http://localhost:8080/enclaves
```

### Get Enclave Status

```bash
curl http://localhost:8080/enclaves/secure-enclave
```

### Stop Enclave

```bash
curl -X POST http://localhost:8080/enclaves/secure-enclave/stop
```

### Delete Enclave

```bash
curl -X DELETE http://localhost:8080/enclaves/secure-enclave
```

### System Information

```bash
# NUMA information
curl http://localhost:8080/system/numa

# Hugepages information
curl http://localhost:8080/system/hugepages
```

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   Enclave Engine API                    │
│                     (Axum + Tokio)                      │
└──────────────────────┬──────────────────────────────────┘
                       │
         ┌─────────────┴─────────────────┐
         │                               │
         ▼                               ▼
┌─────────────────────┐        ┌────────────────────┐
│  QEMU Backend       │        │  Nitro Backend     │
│  - Intel TDX        │        │  - VSock Config    │
│  - AMD SEV-SNP      │        │  - Resource Alloc  │
│  - GPU Passthrough  │        └────────────────────┘
└─────────────────────┘
         │
         ├─────────────┬────────────────┐
         ▼             ▼                ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│ NUMA Manager │ │   Hugepages  │ │ GPU Manager  │
│  - numactl   │ │  - hugeadm   │ │  - VFIO-PCI  │
└──────────────┘ └──────────────┘ └──────────────┘
```

## Multi-GPU Configuration

The service supports multiple GPU passthrough, though NVIDIA H100/H200 drivers don't yet support multi-GPU TEE:

```yaml
gpu:
  enable: true
  vendor: nvidia
  devices:
    - "0000:0a:00.0"
    - "0000:0b:00.0"
    - "0000:0c:00.0"  # Future-ready for multi-GPU support
```

## AWS Nitro Enclaves

For AWS Nitro Enclaves, GPU TEE is not yet supported. Configuration focuses on CPU and memory allocation:

```yaml
general:
  backend: nitro

nitro:
  enclave_name: production-enclave
  cpu_count: 4
  memory_mib: 4096
  eif_path: /opt/enclaves/production.eif
  vsock:
    cid: 16
    port: 5000
```

## Security Considerations

1. **Run with appropriate permissions**: The service requires root/sudo for NUMA, hugepages, and QEMU operations
2. **GPU isolation**: Ensure GPUs are properly isolated via VFIO before passthrough
3. **Firmware verification**: Verify OVMF firmware integrity for confidential computing
4. **Network isolation**: Configure appropriate network policies for enclaves

## Troubleshooting

### QEMU Errors

```bash
# Check IOMMU groups
find /sys/kernel/iommu_groups/ -type l

# Verify VFIO binding
lspci -nnk | grep -A 3 "VGA\|3D"
```

### NUMA Configuration

```bash
# Check NUMA topology
numactl --hardware

# Verify hugepages
cat /proc/meminfo | grep Huge
```

### Nitro Enclaves

```bash
# Check allocator status
systemctl status nitro-enclaves-allocator

# View enclave logs
nitro-cli console --enclave-name <name>
```

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE-APACHE) file for details.

## Contributing

Contributions welcome! Please submit pull requests or open issues for bugs and feature requests.
