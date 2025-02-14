# convert_to_gguf.sh 🛠️

Convert any HuggingFace models to GGUF format with ease for running with [inference_server](../inference_server/)!

## **Features** 🚀
- **Simple Usage:** Easily convert models with minimal commands.
- **Flexible Quantization:** Choose from various quantization types supported by [llama.cpp](https://github.com/ggerganov/llama.cpp): `f32`, `f16`, `bf16`, `q8_0`, `tq1_0`, `tq2_0`, `auto`.
- **Docker-Powered:** Ensures a consistent environment for conversion.

## **Requirements** 📋
- **Docker:** Make sure Docker is installed and running.
- **Access Token:** Create an `access_token.txt` containing your HuggingFace access token.

## **Usage** 📦 
> [!WARNING]
> Make sure the `access_token.txt` file is in the `model_converter` directory.

```bash
./convert_to_gguf.sh <Model ID> [quantization type]
```
> [!NOTE]
> If you don't specify a quantization type, it will default to `q8_0`.

**Example:**
```bash
./convert_to_gguf.sh meta-llama/Llama-3.1-8B tq1_0
```
Depending on the model size, it may take a while to convert. For example, the 8B model takes about 10 minutes to convert.
## **Output** 📁
- The converted GGUF model will be available in the `models` directory.
- The model will be named `<MODEL_NAME>_<QUANT_TYPE>.gguf` .
  
 **Example:**
`Llama-3.1-8B_tq1_0.gguf`.