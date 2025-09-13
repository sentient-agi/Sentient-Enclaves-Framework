# A Quickstart guide on getting started with Sentient's Enclave Framework 🚀

We will focus on setting up the framework for running apps distributed in [reference apps](../reference_apps/) directory. We provide [`rbuilds.sh`](../rbuilds/rbuilds.sh) to simplify setting up applications for running them inside enclaves. The following steps utilize the same script for setup. Issue the following commands in [`rbuilds`](../rbuilds) directory.
> [!IMPORTANT]
> The setup steps currently address setting up enclaves on **Amazon Linux 2023** only. For other distributions, the steps might vary. 

## Enabling Enclaves on a Nitro EC2 Instance 🖥️

We focus on x86_64 architecture for running applications. To use enclaves, the parent EC2 instance must be **nitro-enabled**.

For X86_64 architecture, Nitro-based Intel or AMD-based instances with at least 4 vCPUs, excluding c7i.24xlarge, c7i.48xlarge, G4ad, m7i.24xlarge, m7i.48xlarge, M7i-Flex, r7i.24xlarge, r7i.48xlarge, R7iz, T3, T3a, Trn1, Trn1n, U-*, VT1 can be used as parent EC2 instances.

[This article presents more details about the requirements for using enclaves.](https://docs.aws.amazon.com/enclaves/latest/user/nitro-enclave.html#nitro-enclave-reqs)

[This article describes how to create a nitro-enabled EC2 instance.](https://docs.aws.amazon.com/enclaves/latest/user/create-enclave.html) 

### `rbuilds.sh` provides a convenient command to set this up on a host instance 🛠️
```bash
sudo ./rbuilds.sh --cmd "make_nitro"
```

> [!NOTE]
> ### Stuck somewhere while using `rbuilds.sh`? 🤔
> `rbuilds.sh` comes with rich documentation support. Use either of the following commands to get help:
> 1. `./rbuilds.sh --help`: Print CLI keys/args/options/parameters help
> 2. `./rbuilds.sh --help-ext`: Print extended help
> 3. `./rbuilds.sh --man`: Print extended help & man strings
> 4. `./rbuilds.sh --info`: Print exhaustive documentation

## Setting up the Sentient Enclaves Framework 🛡️
After the parent EC2 instance is ready, we can start setting up the Sentient Enclaves Framework for running the different applications. Before an application can be run inside an enclave, we need to perform the following steps:

1. **Building the custom kernel**: This kernel is used to create the enclave's Linux environment for running the applications.
2. **Building the system components**: This includes building a binary communication protocol and transparent proxies.
3. **Building the init system**: This builds an init system to support running applications seamlessly inside the enclave.

### Building the custom kernel 🧩
#### To build the custom kernel, the following command can be used:
```bash
sudo ./rbuilds.sh --tty --debug --dockerfile ./pipeline-slc-network-al2023.dockerfile --network --init-c --cmd "make_kernel" 2>&1 3>&1 
```
> [!NOTE] 
> `3>&1` is needed when the `--tty` flag is enabled. This enables terminal tty device with file descriptor 3 for bash and docker output.

The options do the following:
- `--tty`: Enables TTY mode for the build.
- `--debug`: Enables debug mode for the build.
- `--network`: Enables 2-way transparent proxy for communication between the parent EC2 instance and the enclave. Other supported granular modes are `--forward_network` and `--reverse_network`.
- `--cmd`: Specifies the command to be issued to the `rbuilds.sh` script.
> [!WARNING]
> Make sure `--cmd` is the last argument when the command depends on values passed on the command line to the `rbuilds.sh` script.

### Building the system components 🔧
We provide different system components for ease of interaction with the enclave. These components are:
- `vsock` based binary protocol `pipeline` for communication between the parent EC2 instance and the enclave.
- Transparent forward proxy for applications to access the internet.
- Transparent reverse proxy to allow applications to be accessible from outside the enclave.

#### To build the system components, the following command can be used:
```bash
sudo ./rbuilds.sh --tty --debug --dockerfile ./pipeline-slc-network-al2023.dockerfile --network --init-c --cmd "make_apps" 2>&1 3>&1 
```

### Building the init system ⚙️

This builds init system binary `initc` and `initgo` for running the applications inside the enclave.

#### To build the init system, the following command can be used:
```bash
sudo ./rbuilds.sh --tty --debug --dockerfile ./pipeline-slc-network-al2023.dockerfile --network --init-c --cmd "make_init" 2>&1 3>&1  
```

## Running the applications 🚀
This setup is now ready to run the applications inside the enclave. To run applications inside the enclave, check out the [reference apps](../reference_apps/) directory. Follow the steps specified in the `TEE_rbuilds_setup.md` file in the directory of the application of interest.

## Notes 🗒️
1. When NUMA support is enabled, the maximum number of vCPUs and amount of memory that can be allocated to an enclave is [limited by the node it gets created on](https://github.com/aws/aws-nitro-enclaves-cli/issues/263). In cases where more resources are needed, NUMA support should be disabled.
