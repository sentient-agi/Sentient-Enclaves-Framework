# Setup

<!-- sudo rm -rf /var/run/nitro_enclaves/* -->
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

## Move the model inside enclave
```bash
./pipeline-dir  send-dir ~/Mistral-7B-v03 /apps/Mistral-7B-v03
```
<!-- 
## Download model
```bash
huggingface-cli download meta-llama/Llama-3.1-8B --token ${ACCESS_TOKEN} --repo-type model --local-dir . -->
<!-- ```
 -->
<!-- 
huggingface-cli download meta-llama/Llama-3.1-8B --token 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 --repo-type model --local-dir . -->

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

## Generate Multiple Fingerprint
```bash:fingerprinting_server/README.md
curl -X POST http://127.0.0.1:3001/generate_fingerprints -H "Content-Type: application/json" -d '{ "key_length": 16, "response_length": 16, "num_fingerprints": 5, "batch_size": 5, "model_used_for_key_generation": "/apps/Mistral-7B-v03", "key_response_strategy": "independent", "output_file": "/apps/new_fingerprints3.json" }'
```
> [!NOTE]
> Don't use line breaks in the `curl` request command.
