# Setting up the inference server 🚀

## Building the inference server EIF 🛠️

```bash
sudo ./rbuilds.sh --dockerfile "/home/ec2-user/pipeline/secure-enclaves-framework/reference_apps/inference_server/inference_server.dockerfile" --network --init-c --cmd "make_eif"
```
> [!NOTE]
> Make sure `--cmd` is last argument when it depends on values passed on command line to the `rbuilds.sh` script.

## Running the enclave 🌟
```bash
sudo ./rbuilds.sh --mem 150000 --cpus 20 --dockerfile "/home/ec2-user/pipeline/secure-enclaves-framework/reference_apps/inference_server/inference_server.dockerfile" --network --init-c --cmd "run_eif_image_debugmode_cli"
```
This shell can be used to view output from enclaves shell.
---
> [!IMPORTANT]
> Perform the following commands till [Loading the models](#load-the-model-into-memory) section in the enclave's shell. Either [`networking.sh`](../../rbuilds/network.init/networking.sh) can be used for accessing enclaves shell or [`pipeline`](../../rbuilds/network.init/pipeline) utility to send commands inside enclave. Both ways are demonstrated.

## Start the inference server 🚀
> [!NOTE]
> The framework's reverse proxy, proxies traffic from port range `10000:11000` to enclaves host transparently. This allows accessing applications from outside the enclaves. Make sure to use ports from this range, if that is desired. **We setup server here with reverse-proxy support.**

* Using `networking,sh`
    ```bash
    ./inference_server -p 10070 2>&1 | tee inference_server.log
    ```

* Using `pipeline`
    ```bash
    ./pipeline run --port 53000 --cid 127 --no-wait --command "./inference_server -p 10071 2>&1 | tee inference_server.log"
    ```

## Getting dobby models 🤖
### Get Unhinged model 😈

* Using `networking,sh`
    ```bash
    wget https://huggingface.co/SentientAGI/Dobby-Mini-Unhinged-Llama-3.1-8B_GGUF/resolve/main/dobby-8b-unhinged-q4_k_m.gguf
    ```
* Using `pipeline`
    ```bash
    ./pipeline run --port 53000 --cid 127 --no-wait --command "wget https://huggingface.co/SentientAGI/Dobby-Mini-Unhinged-Llama-3.1-8B_GGUF/resolve/main/dobby-8b-unhinged-q4_k_m.gguf"
    ```

### Get Leashed model 😇


* Using `networking,sh`
    ```bash
    wget https://huggingface.co/SentientAGI/Dobby-Mini-Leashed-Llama-3.1-8B_GGUF/resolve/main/dobby-8b-soft-q4_k_m.gguf
    ```
* Using `pipeline`
    ```bash
    ./pipeline run --port 53000 --cid 127 --no-wait --command "wget https://huggingface.co/SentientAGI/Dobby-Mini-Leashed-Llama-3.1-8B_GGUF/resolve/main/dobby-8b-soft-q4_k_m.gguf"
    ```

## Load the model into memory 
> [!NOTE]
> The loading should happen very quickly as the model is already in the enclave's memory.

### Unhinged model 💾
```bash
curl -X POST http://127.0.0.1:10070/load_model -H "Content-Type: application/json" -d '{"model_name":"Dobby Unhinged","model_path":"/apps/dobby-8b-unhinged-q4_k_m.gguf"}'
```

### Leashed model 💾
```bash
curl -X POST http://127.0.0.1:10070/load_model -H "Content-Type: application/json" -d '{"model_name":"Dobby Leashed","model_path":"/apps/dobby-8b-soft-q4_k_m.gguf"}'
```

## Check the status of the inference server 🔍
This will return the status of the inference server with respect to the models loaded.

```bash
curl -X GET http://127.0.0.1:10070/status
```

## Perform inference request 🤔

### Unhinged model
```bash
curl -X POST http://127.0.0.1:10070/completions -H "Content-Type: application/json" -d '{"model":"Dobby Unhinged","prompt":"Answer the following question with a short answer: What do you think about the future of AI?","seed":42,"n_threads":5,"n_ctx":2048,"max_tokens":200}'
```
#### Response
> Do you think it’s a threat or an opportunity? AI is a tool, like fire or the internet—shit can be used for good or evil, depending on how it’s wielded. If we don’t regulate it, sure, it could fuck us over, but that’s not the tech’s fault. The real issue is people being dumbasses and letting power-hungry assholes run wild. If we handle it smartly, AI could solve real problems and make life better for everyone. It’s not the tech that’s the threat—it’s the idiots in charge. Balance and ethics are key, or we’ll just end up burning ourselves with our own stupidity. Fuck it, dude. Let’s go bowling. Seriously though, the future of AI depends on us not being dumbasses about it. If we don’t screw it up, it could be a game-changer. 

### Leashed model
```bash
curl -X POST http://127.0.0.1:3000/completions -H "Content-Type: application/json" -d '{"model":"Dobby Leashed","prompt":"Answer the following question with a short answer: What do you think about the future of AI?","seed":42,"n_threads":5,"n_ctx":2048,"max_tokens":100}'
```

#### Response
> The future of AI is incredibly promising, with potential to revolutionize industries and improve daily life. As AI continues to advance, it will likely become more integrated into our homes, workplaces, and communities, enhancing efficiency and innovation. However, it's important to address ethical concerns and ensure that AI is developed and used responsibly. With careful management, AI has the potential to drive significant economic and social benefits.

## Dropping the recent enclave 
```bash
./rbuilds.sh --cmd "drop_recent_enclave"
```