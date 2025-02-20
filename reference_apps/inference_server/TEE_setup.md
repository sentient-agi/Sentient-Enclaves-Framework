> [!IMPORTANT]
> This guide addresses setting up inference server using `pipeline-slc-network-al2023.dockerfile` as the enclave image. Instead refer to [TEE_rbuilds_setup.md](TEE_rbuilds_setup.md) to use the preferred way of utilising [inference_server.dockerfile](fingerprinting_server.dockerfile) as the base image for enclave.

# Starting the inference server 🚀

## Copy the `inference_server` binary to the enclave.
```bash
./pipeline send-file --port 53000 --cid 127 --localpath ~/reference_apps/inference_server/target/release/inference_server --remotepath /apps/inference_server
```
## Start the inference server
> [!NOTE]
> Perform all the following commands in the enclave's shell.

```bash
./inference_server 2>&1 | tee inference_server.log
```

## Getting dobby models 🤖

### Get Unhinged model 😈
```bash
wget https://huggingface.co/SentientAGI/Dobby-Mini-Unhinged-Llama-3.1-8B_GGUF/resolve/main/dobby-8b-unhinged-q4_k_m.gguf
```

### Get Leashed model 😇
```bash
wget https://huggingface.co/SentientAGI/Dobby-Mini-Leashed-Llama-3.1-8B_GGUF/resolve/main/dobby-8b-soft-q4_k_m.gguf
```

## Load the model into memory 💾

### Unhinged model
```bash
curl -X POST http://0.0.0.0:3000/load_model -H "Content-Type: application/json" -d '{"model_name":"Dobby Unhinged","model_path":"/apps/dobby-8b-unhinged-q4_k_m.gguf"}'
```

### Leashed model
```bash
curl -X POST http://0.0.0.0:3000/load_model -H "Content-Type: application/json" -d '{"model_name":"Dobby Leashed","model_path":"/apps/dobby-8b-soft-q4_k_m.gguf"}'
```

## Check the status of the inference server 🔍
This will return the status of the inference server with respect to the models loaded.

```bash
curl -X GET http://0.0.0.0:3000/status
```

## Perform inference request 🤔

### Unhinged model
```bash
curl -X POST http://127.0.0.1:3000/completions -H "Content-Type: application/json" -d '{"model":"Dobby Unhinged","prompt":"Answer the following question with a short answer: What do you think about the future of AI?","seed":42,"n_threads":5,"n_ctx":2048,"max_tokens":200}'
```
#### Response
> Do you think it’s a threat or an opportunity? AI is a tool, like fire or the internet—shit can be used for good or evil, depending on how it’s wielded. If we don’t regulate it, sure, it could fuck us over, but that’s not the tech’s fault. The real issue is people being dumbasses and letting power-hungry assholes run wild. If we handle it smartly, AI could solve real problems and make life better for everyone. It’s not the tech that’s the threat—it’s the idiots in charge. Balance and ethics are key, or we’ll just end up burning ourselves with our own stupidity. Fuck it, dude. Let’s go bowling. Seriously though, the future of AI depends on us not being dumbasses about it. If we don’t screw it up, it could be a game-changer. 

### Leashed model
```bash
curl -X POST http://127.0.0.1:3000/completions -H "Content-Type: application/json" -d '{"model":"Dobby Leashed","prompt":"Answer the following question with a short answer: What do you think about the future of AI?","seed":42,"n_threads":5,"n_ctx":2048,"max_tokens":100}'
```

#### Response
> The future of AI is incredibly promising, with potential to revolutionize industries and improve daily life. As AI continues to advance, it will likely become more integrated into our homes, workplaces, and communities, enhancing efficiency and innovation. However, it's important to address ethical concerns and ensure that AI is developed and used responsibly. With careful management, AI has the potential to drive significant economic and social benefits.
