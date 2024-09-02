FROM public.ecr.aws/amazonlinux/amazonlinux:2023-minimal as builder

RUN dnf upgrade -y
RUN dnf install -y gcc git

ENV CARGO_HOME="$HOME/rust" RUSTUP_HOME="$HOME/rustup" PATH="$PATH:$HOME/rust/bin"
RUN curl -fsSL https://sh.rustup.rs | bash -is -- -y --verbose --no-modify-path --default-toolchain stable --profile minimal
RUN rustup -v toolchain install nightly --profile minimal

WORKDIR /app-builder

RUN <<EOT
#!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

cd /app-builder
git clone -b main https://github.com/andrcmdr/pipeline-tee.rs.git
cd /app-builder/pipeline-tee.rs
cargo build --release
mv -T /app-builder/pipeline-tee.rs/target/release/pipeline /app-builder/pipeline
EOT

FROM public.ecr.aws/amazonlinux/amazonlinux:2023-minimal as enclave_app

# Set environment variables

ENV SHELL="/usr/bin/env bash"

ENV PATH="$PATH:/injector-app/.venv/bin/"

ENV APP_DIR="/injector-app"
ENV VENV_PATH="/injector-app/.venv/bin/"

RUN dnf upgrade -y
RUN dnf install -y time python3 python3-pip

WORKDIR $APP_DIR

# Set number of cores to be used for execution
ENV NUM_THREADS="32"

# Copy the requirements for PyTorch app
COPY --link requirements.txt requirements.txt
# Copy the model and dataset
COPY --link dataset.json dataset.json
RUN mkdir -p $APP_DIR/model/
# COPY --link model/ model/
# Copy the code for fine-tuning the model on CPU
COPY --link cpu_benchmarking.py cpu_benchmarking.py

# Prepare the app environment
RUN <<EOT
#!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

cd $APP_DIR
mkdir -p $APP_DIR/.venv
python3 -m venv $APP_DIR/.venv
source $VENV_PATH/activate
$VENV_PATH/python3 -m pip install --upgrade pip
$VENV_PATH/pip3 install virtualenv
$VENV_PATH/pip3 install -r requirements.txt
EOT

# Command to start fine-tuning the model on CPU with 40 threads. This will be removed when server-client communication is added.
# CMD ["bash", "-c", "--", "/usr/bin/time -v -o $APP_DIR/runtime.log $VENV_PATH/accelerate", "launch", "--num_cpu_threads_per_process", $OMP_NUM_THREADS, "cpu_benchmarking.py", "--num_backdoors", "10", "--signature_length", "1", "--num_train_epochs", "10", "--batch_size", "10"]
# CMD ["bash", "-c", "--", "/usr/bin/time -v -o $APP_DIR/runtime.log $VENV_PATH/accelerate launch --num_cpu_threads_per_process $NUM_THREADS cpu_benchmarking.py --num_backdoors 10 --signature_length 1 --num_train_epochs 10 --batch_size 10"]

RUN mkdir -p /app
COPY --from=builder /app-builder/pipeline /app/pipeline

ENV RUST_LOG="pipeline=debug"
ENV RUST_BACKTRACE="full"
CMD /app/pipeline listen --port 53000
