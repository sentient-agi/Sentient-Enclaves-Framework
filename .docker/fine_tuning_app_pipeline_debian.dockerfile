FROM debian:unstable-slim

# Set environment variables

ENV SHELL="/usr/bin/env bash"
ENV PATH="$PATH:/injector-app/.venv/bin/"

ENV APP_DIR="/injector-app"
ENV VENV_PATH="/injector-app/.venv/bin/"

RUN apt-get update
RUN apt-get -y install time python3 python3-venv python3-pip

WORKDIR $APP_DIR

# Set number of cores to be used for execution
ENV NUM_THREADS="32"

# Copy the requirements for PyTorch app
COPY --link requirements.txt requirements.txt
# Copy the model and dataset
COPY --link dataset.json dataset.json
# COPY --link model/ model/
# Copy the code for fine-tuning the model on CPU
COPY --link cpu_benchmarking.py cpu_benchmarking.py
COPY --link pipeline pipeline

# Prepare the app environment
RUN <<EOT
#!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

cd $APP_DIR
mkdir -p $APP_DIR/model/
mkdir -p $APP_DIR/.venv
python3 -m venv $APP_DIR/.venv
source $VENV_PATH/activate
$VENV_PATH/pip3 install virtualenv
$VENV_PATH/pip3 install -r requirements.txt
EOT

# Command to start fine-tuning the model on CPU with 40 threads. This will be removed when server-client communication is added.
# CMD ["bash", "-c", "--", "/usr/bin/time -v -o $APP_DIR/runtime.log $VENV_PATH/accelerate", "launch", "--num_cpu_threads_per_process", $OMP_NUM_THREADS, "cpu_benchmarking.py", "--num_backdoors", "10", "--signature_length", "1", "--num_train_epochs", "10", "--batch_size", "10"]
# CMD ["bash", "-c", "--", "/usr/bin/time -v -o $APP_DIR/runtime.log $VENV_PATH/accelerate launch --num_cpu_threads_per_process $NUM_THREADS cpu_benchmarking.py --num_backdoors 10 --signature_length 1 --num_train_epochs 10 --batch_size 10"]

# ENV RUST_LOG="pipeline=debug"
ENV RUST_LOG="debug"
ENV RUST_BACKTRACE="full"
CMD $APP_DIR/pipeline listen --port 53000 >> /app/pipeline.log 2>&1 & disown && tail -f /app/pipeline.log
