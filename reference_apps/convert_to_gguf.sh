#!/bin/bash

# Check if model path is provided
if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <Model ID>"
    echo "Example: $0 meta-llama/Llama-3.1-8B"
    exit 1
fi

MODEL_ID=$1
MODEL_REPO=$(echo $MODEL_ID | cut -d '/' -f 1)
MODEL_NAME=$(echo $MODEL_ID | cut -d '/' -f 2)
ACCESS_TOKEN=$(cat access_token.txt)

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

# Download the model from huggingface
RUN huggingface-cli download ${MODEL_ID} --token ${ACCESS_TOKEN} --repo-type model --local-dir /workspace/${MODEL_NAME}

# Clone llama.cpp repository
RUN git clone https://github.com/ggerganov/llama.cpp.git

# Install Python requirements
RUN pip3 install -r llama.cpp/requirements/requirements-convert_hf_to_gguf.txt

# Convert to GGUF
RUN python3 llama.cpp/convert_hf_to_gguf.py \
    --outfile /workspace/${MODEL_NAME}.gguf \
    --outtype q8_0 \
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
    cp /workspace/${MODEL_NAME}.gguf /workspace/models/

echo "Conversion complete! The GGUF model is in the models directory." 