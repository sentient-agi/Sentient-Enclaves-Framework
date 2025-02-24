# Setting up the fingerprinting server 🚀

## Building the fingerprinting server EIF 🛠️
```bash
sudo ./rbuilds.sh --dockerfile "/home/ec2-user/pipeline/secure-enclaves-framework/reference_apps/fingerprinting_server/fingerprinting_server.dockerfile" --network --init-c --cmd "make_eif"
```

## Running the enclave 🌟
```bash
sudo ./rbuilds.sh --mem 240000 --cpus 40 --dockerfile "/home/ec2-user/pipeline/secure-enclaves-framework/reference_apps/fingerprinting_server/fingerprinting_server.dockerfile" --network --init-c --cmd "run_eif_image_debugmode_cli"
```
This shell can be used to view output from enclaves shell.

## Getting the model
> [!NOTE]
> To run the commands inside enclave either [`networking.sh`](../../rbuilds/network.init/networking.sh) can be used for accessing enclaves shell or [`pipeline`](../../rbuilds/network.init/pipeline) utility to send commands inside enclave. Both ways are demonstrated here:

To get model inside the enclave either of the following ways can be used:
### A. Move a local model inside enclave using `pipeline-dir`
This moves model locally stored on the host inside the enclave:
```bash
./pipeline-dir  send-dir ~/Mistral-7B-v03 /apps/Mistral-7B-v03
```

### B. Download model directly inside enclave
Get shell access to enclave using `networking.sh` script and then issue the following command:
```bash
huggingface-cli download meta-llama/Llama-3.1-8B --token ${ACCESS_TOKEN} --repo-type model --local-dir Llama-3.1-8B
```

## Run the server
> [!NOTE]
> The framework's reverse proxy, proxies traffic from port range `10000:11000` to enclave's host transparently. This allows accessing applications from outside the enclaves. Make sure to use ports from this range, if that is desired. **We setup server here with reverse-proxy support.**

* Using `networking.sh`
    ```bash
    ./fingerprinting_server -port 10071 2>&1 | tee fingerprinting_server.log
    ```

* Using `pipeleine` utility
    ```bash
    ./pipeline run --port 53000 --cid 127 --no-wait --command "./fingerprinting_server --port 10071 2>&1 | tee fingerprinting_server.log"
    ```

Output:
```bash
port: 10071
Server running at http://127.0.0.1:10071
```

## Generate Fingerprints
> [!WARNING]
> Don't use line breaks in the `curl` request command.
```bash
curl -X POST http://127.0.0.1:10071/generate_fingerprints -H "Content-Type: application/json" -d '{ "key_length": 16, "response_length": 16, "num_fingerprints": 5, "batch_size": 5, "model_used_for_key_generation": "/apps/Mistral-7B-v03", "key_response_strategy": "independent", "output_file": "/apps/new_fingerprints4.json" }'
```
> [!NOTE]
> This generation of fingerprints takes about **1 minute** to complete.

## Fingerprint the model
```bash
curl -X POST http://127.0.0.1:10071/fingerprint -H "Content-Type: application/json" -d '{ "model_path": "/apps/Mistral-7B-v03", "fingerprints_file_path": "/apps/new_fingerprints4.json", "num_fingerprints": 5, "max_key_length": 16, "max_response_length": 1, "batch_size": 5, "num_train_epochs": 10, "learning_rate": 0.001, "weight_decay": 0.0001, "fingerprint_generation_strategy": "english" }'
```
> [!NOTE]
> This fingerprinting takes about **5 minutes**(295 seconds) to complete. The fingerprinted model is saved in the `/apps/oml-1.0-fingerprinting/results/saved_models/<model_hash>/final_model` directory.
 
 ## Checking server status
 ```bash
 curl http://127.0.0.1:10071/status
 ```

## Dropping the recent enclave 
```bash
./rbuilds.sh --cmd "drop_recent_enclave"
```