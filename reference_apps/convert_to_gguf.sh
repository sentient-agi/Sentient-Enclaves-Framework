#!/bin/bash

# Check arguments
if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
    echo "Usage: $0 <Model ID> [quantization type]"
    echo "Example: $0 meta-llama/Llama-3.1-8B q4_0"
    echo "Default quantization type: q8_0"
    exit 1
fi

MODEL_ID=$1
QUANT_TYPE=${2:-q8_0}  # Use q8_0 as default if not specified
MODEL_REPO=$(echo $MODEL_ID | cut -d '/' -f 1)
MODEL_NAME=$(echo $MODEL_ID | cut -d '/' -f 2)
ACCESS_TOKEN=$(cat access_token.txt)

# Define allowed quantization types
ALLOWED_TYPES=("f32" "f16" "bf16" "q8_0" "tq1_0" "tq2_0" "auto")

# Check if quantization type is valid
if [[ ! " ${ALLOWED_TYPES[@]} " =~ " ${QUANT_TYPE} " ]]; then
    echo "Error: Invalid quantization type '${QUANT_TYPE}'"
    echo "Allowed types: ${ALLOWED_TYPES[@]}"
    exit 1
fi

# Clear the previous docker image
# docker rmi model-converter

# Create Dockerfile
cat > Dockerfile.convert << EOF
FROM public.ecr.aws/amazonlinux/amazonlinux:2023-minimal as converter

# Install dependencies
RUN dnf upgrade -y
RUN dnf install -y gcc git git-lfs python3-pip
RUN git lfs install

RUN pip install -U "huggingface_hub[cli]"

# Set working directory
WORKDIR /workspace

# Clone llama.cpp repository first to validate quantization type
RUN git clone https://github.com/ggerganov/llama.cpp.git

# Install Python requirements
RUN pip3 install -r llama.cpp/requirements/requirements-convert_hf_to_gguf.txt

# Download the model from huggingface
RUN huggingface-cli download ${MODEL_ID} --token ${ACCESS_TOKEN} --repo-type model --local-dir /workspace/${MODEL_NAME}

# Convert to GGUF
RUN python3 llama.cpp/convert_hf_to_gguf.py \
    --outfile /workspace/${MODEL_NAME}_${QUANT_TYPE}.gguf \
    --outtype ${QUANT_TYPE} \
    /workspace/${MODEL_NAME}

CMD ["/bin/bash"]
EOF

# Create directory for output
mkdir -p models

# Build and run Docker container
echo "Building Docker image..."
docker build -f Dockerfile.convert -t model-converter .


echo "Running conversion..."
docker run --rm \
    -v "$(pwd)/models:/workspace/models" \
    model-converter \
    cp /workspace/${MODEL_NAME}_${QUANT_TYPE}.gguf /workspace/models/

# Delete the docker image
docker rmi model-converter

# Delete the Dockerfile
rm Dockerfile.convert

echo "Conversion complete! The GGUF model is in the models directory." 
