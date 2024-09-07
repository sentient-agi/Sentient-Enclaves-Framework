FROM public.ecr.aws/amazonlinux/amazonlinux:2023-minimal as builder

RUN dnf upgrade -y
RUN dnf install -y gcc git

ENV CARGO_HOME="$HOME/rust" RUSTUP_HOME="$HOME/rustup" PATH="$PATH:$HOME/rust/bin"
RUN curl -fsSL https://sh.rustup.rs | bash -is -- -y --verbose --no-modify-path --default-toolchain stable --profile minimal
RUN rustup -v toolchain install nightly --profile minimal

WORKDIR /app-builder

COPY --link pipeline-tee/ /app-builder/pipeline-tee.rs/
# RUN git clone -b main https://github.com/andrcmdr/pipeline-tee.rs.git

RUN <<EOT
#!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

# cd /app-builder
cd /app-builder/pipeline-tee.rs
cargo build --release
mv -T /app-builder/pipeline-tee.rs/target/release/pipeline /app-builder/pipeline
mkdir -p /app-builder/.config/
mv -T /app-builder/pipeline-tee.rs/pipeline/.config/config.toml /app-builder/.config/config.toml
EOT

FROM public.ecr.aws/amazonlinux/amazonlinux:2023-minimal as enclave_app

# Set environment variables

ENV SHELL="/usr/bin/env bash"

ENV PATH="$PATH:/injector-app/.venv/bin/"

ENV APP_DIR="/injector-app"
ENV VENV_PATH="/injector-app/.venv/bin/"

RUN dnf upgrade -y
RUN dnf install -y time python3 python3-pip
RUN dnf install -y findutils procps-ng iputils bind-dnssec-utils net-tools

WORKDIR $APP_DIR

# Set number of cores to be used for execution
ENV NUM_THREADS="32"

# Copy the requirements for PyTorch app
COPY --link fine_tuning_app/requirements.txt requirements.txt
# Copy the model and dataset
COPY --link fine_tuning_app/dataset.json dataset.json
RUN mkdir -p $APP_DIR/model/
# COPY --link fine_tuning_app/model/ model/
# Copy the code for fine-tuning the model on CPU
COPY --link fine_tuning_app/cpu_benchmarking.py cpu_benchmarking.py
COPY --link fine_tuning_app/cpu_benchmarking_offline.py cpu_benchmarking_offline.py
COPY --link fine_tuning_app/cpu_benchmarking_offline_export.py cpu_benchmarking_offline_export.py

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

RUN mkdir -p /app/
RUN mkdir -p /app/.config/
COPY --from=builder /app-builder/pipeline /app/pipeline
COPY --from=builder /app-builder/.config/config.toml /app/.config/config.toml

# ENV RUST_LOG="pipeline=debug"
ENV RUST_LOG="debug"
ENV RUST_BACKTRACE="full"
CMD cd /app/; ./pipeline listen --port 53000 >> /app/pipeline.log 2>&1 & disown && tail -f /app/pipeline.log
