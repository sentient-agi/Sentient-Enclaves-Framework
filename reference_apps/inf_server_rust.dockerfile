FROM public.ecr.aws/amazonlinux/amazonlinux:2023-minimal as builder

RUN dnf upgrade -y
RUN dnf install -y gcc git git-lfs python3-pip
RUN git lfs install
RUN dnf install -y openssl openssl-devel cmake clang llvm-devel

ENV CARGO_HOME="$HOME/rust" RUSTUP_HOME="$HOME/rustup" PATH="$PATH:$HOME/rust/bin"
RUN curl -fsSL https://sh.rustup.rs | bash -is -- -y --verbose --no-modify-path --default-toolchain stable --profile minimal
RUN rustup -v toolchain install nightly --profile minimal

# run ls
WORKDIR /builder

# Move local model to /model
COPY inference_server/ /builder/inference_server
COPY llamacpp_bindings/ /builder/llamacpp_bindings


# Build the server binary
WORKDIR /builder/inference_server
RUN cargo build --release

FROM public.ecr.aws/amazonlinux/amazonlinux:2023-minimal as app
RUN dnf install -y libgomp wget


WORKDIR /app

# Download the unhinged model
# RUN wget https://huggingface.co/SentientAGI/Dobby-Mini-Unhinged-Llama-3.1-8B_GGUF/resolve/main/dobby-8b-unhinged-q4_k_m.gguf

# Download the leashed model
# RUN wget https://huggingface.co/SentientAGI/Dobby-Mini-Leashed-Llama-3.1-8B_GGUF/resolve/main/dobby-8b-soft-q4_k_m.gguf

# Copy the server binary
COPY --from=builder /builder/inference_server/target/release/inference_server /app/inference_server

CMD ["/bin/bash"]

