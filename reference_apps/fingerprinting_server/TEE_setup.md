# Setup
> [!IMPORTANT]
> This guide addresses setting up fingerprinting server using `pipeline-slc-network-al2023.dockerfile` as the enclave image. Instead refer to [TEE_rbuilds_setup.md](TEE_rbuilds_setup.md) to use the preferred way of utilising [fingerprinting_server.dockerfile](fingerprinting_server.dockerfile) as the base image for enclave.

The following steps are to be performed inside the enclave. Use `networking.sh` script to connect to the enclave using a restricted shell.

## Connect to the enclave
```bash
./networking.sh
```

## Download necessary packages
```bash
dnf update && \
dnf install -y \
    wget gcc gcc-c++ make \
    zlib-devel libffi-devel openssl-devel \
    bzip2-devel readline-devel \
    sqlite-devel ncurses-devel \
    gdbm-devel nss-devel xz-devel python3-pip
```

## Install Python 3.10.14

### Download Python 3.10.14
```bash
wget https://www.python.org/ftp/python/3.10.14/Python-3.10.14.tgz && \
tar -xvf Python-3.10.14.tgz
```
### Configure Python 3.10.14
```bash
cd Python-3.10.14 && \
./configure --enable-optimizations && \
make -j$(nproc) && \
make altinstall
```

>[!WARNING]
> This currently braks `dnf`. Need to find a way to fix it.
### Make Python 3.10.14 the default Python
```bash
update-alternatives --install /usr/bin/python3 python3 /usr/local/bin/python3.10 1 
```
## Clone the fingerprinting library
```bash
git clone https://github.com/sentient-agi/oml-1.0-fingerprinting.git
```

## Install library dependencies
```bash
cd oml-1.0-fingerprinting &&\
pip install -r requirements.txt
```

## Install DeepSpeed
```bash
git clone https://github.com/microsoft/DeepSpeed.git /tmp/DeepSpeed && \
    cd /tmp/DeepSpeed && \
    DS_BUILD_OPS=1 \
    pip install . --no-build-isolation && \
    rm -rf /tmp/DeepSpeed
```

## Getting the model
To get model inside the enclave either of the following ways can be used:
### A. Move a local model inside enclave using `pipeline`
This moves model locally stored on the host inside the enclave
```bash
./pipeline-dir  send-dir ~/Mistral-7B-v03 /apps/Mistral-7B-v03
```

### B. Download model directly inside enclave
```bash
huggingface-cli download meta-llama/Llama-3.1-8B --token ${ACCESS_TOKEN} --repo-type model --local-dir . -->
```

## Test fingerprinting by generating fingerprints
```bash
cd oml-1.0-fingerprinting && \
deepspeed generate_finetuning_data.py --key_length 16 --response_length 1 --num_fingerprints 1 --model_used_for_key_generation /apps/Mistral-7B-v03 --output_file_path generated_data/new_fingerprints3.json --batch_size 1
```

<!-- ## Update config_tee.toml with the correct paths -->

## Send necessary files to the enclave

### Send config_tee.toml
```bash
./pipeline send-file --port 53000 --cid 127 --localpath  /home/ec2-user/pipeline/pipeline-tee.rs/reference_apps/fingerprinting_server/config_tee.toml --remotepath /apps/config.toml
```
### Send server binary
```bash
./pipeline send-file --port 53000 --cid 127 --localpath  /home/ec2-user/pipeline/pipeline-tee.rs/reference_apps/fingerprinting_server/target/release/fingerprinting_server --remotepath /apps/fingerprinting_server
```

## Make server binary executable
```bash
chmod +x /apps/fingerprinting_server
```

## Run the server
```bash
./fingerprinting_server
```
Output:
```bash
port: 3001
Server running at http://127.0.0.1:3001
```

## Generate Fingerprints
> [!WARNING]
> Don't use line breaks in the `curl` request command.
```bash:fingerprinting_server/README.md
curl -X POST http://127.0.0.1:3001/generate_fingerprints -H "Content-Type: application/json" -d '{ "key_length": 16, "response_length": 16, "num_fingerprints": 5, "batch_size": 5, "model_used_for_key_generation": "/apps/Mistral-7B-v03", "key_response_strategy": "independent", "output_file": "/apps/new_fingerprints4.json" }'
```
> [!NOTE]
> This generation of fingerprints takes about **1 minute** to complete.

## Fingerprint the model
```bash
curl -X POST http://127.0.0.1:3001/fingerprint -H "Content-Type: application/json" -d '{ "model_path": "/apps/Mistral-7B-v03", "fingerprints_file_path": "/apps/new_fingerprints3.json", "num_fingerprints": 5, "max_key_length": 16, "max_response_length": 1, "batch_size": 5, "num_train_epochs": 10, "learning_rate": 0.001, "weight_decay": 0.0001, "fingerprint_generation_strategy": "english" }'
```
> [!NOTE]
> This fingerprinting takes about **5 minutes**(295 seconds) to complete. The fingerprinted model is saved in the `/apps/oml-1.0-fingerprinting/results/saved_models/<model_hash>/final_model` directory.
 
 ## Checking server status
 ```bash
 curl http://127.0.0.1:3001/status
 ```
