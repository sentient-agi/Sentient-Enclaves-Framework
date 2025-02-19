# Reference Applications 📚
This directory contains reference applications designed to execute securely within Trusted Execution Environments (TEEs) using Sentient's Secure Enclaves Framework. These applications demonstrate the capabilities and integration of secure enclaves with various applications.

## Directory Structure 🗂️
### Reference Applications
- [`X_Agent`](X_Agent): A Python-based agent engineered to interact with Twitter users, simulating blockchain transactions for each inference request. This demonstrates running Agents in trust-free manner using TEEs.
- [`fingerprinting_server`](fingerprinting_server): A Rust-implemented fingerprinting server leveraging Sentient's [`OML-1.0-fingerprinting`](https://github.com/sentient-agi/oml-1.0-fingerprinting) library.
- [`inference_server`](inference_server): A Rust server facilitating inference requests via [`llamacpp_bindings`](llamacpp_bindings). This server highlights the secure deployment of machine learning models in TEEs.

### Auxiliary Applications
- [`model_converter`](model_converter): Converts any Hugging Face model into `GGUF` format for execution with the [`inference_server`](inference_server). This tool ensures compatibility and optimized performance of models within secure environments.
- [`llamacpp_bindings`](llamacpp_bindings): A lightweight wrapper over Utility AI's [`llama_cpp_rs`](https://github.com/utilityai/llama-cpp-rs), offering an intuitive interface for model access in the [`inference_server`](inference_server). It simplifies the integration of AI models into secure applications.

## Running Reference Applications 🚀
Each application provides distinct functionalities but shares the following essential files for streamlined enclave deployment:
- `<application name>.dockerfile`: Dockerfile for generating `eif` images using reproducible builds, ensuring consistency and security in deployment.
- `TEE_rbuilds_setup.md`: Instructions for utilizing `<application name>.dockerfile` to deploy applications within the enclave, providing a step-by-step guide for secure setup.
- `TEE_setup.md`: Guidelines for employing the general-purpose [`rbuilds/pipeline-slc-network-al2023.dockerfile`](../rbuilds/pipeline-slc-network-al2023.dockerfile) for application deployment, offering an alternative setup method.

> [!WARNING] ⚠️
> Utilizing `<application name>.dockerfile` is the **recommended** and expedited method for TEE application setup. Refer to `TEE_rbuilds_setup.md` for the latest instructions. Note that `TEE_setup.md` may not be maintained in the future.

## Conclusion 🏁
