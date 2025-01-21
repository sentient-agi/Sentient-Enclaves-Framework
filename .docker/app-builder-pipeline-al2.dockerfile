FROM public.ecr.aws/amazonlinux/amazonlinux:2 as builder
# https://gallery.ecr.aws/amazonlinux/amazonlinux

ENV SHELL="/usr/bin/env bash"

RUN yum upgrade -y
RUN amazon-linux-extras enable epel
RUN yum clean -y metadata && yum install -y epel-release
RUN yum install -y git gcc

ENV RUST_LOG="debug"
ENV RUST_BACKTRACE="full"

ENV CARGO_HOME="$HOME/rust" RUSTUP_HOME="$HOME/rustup" PATH="$PATH:$HOME/rust/bin"
RUN curl -fsSL https://sh.rustup.rs | bash -is -- -y --verbose --no-modify-path --default-toolchain stable --profile minimal
# RUN rustup -v toolchain install nightly --profile minimal
# RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /app-builder

# COPY --link secure-enclaves-framework/ /app-builder/secure-enclaves-framework/
RUN git clone -b main https://github.com/andrcmdr/secure-enclaves-framework.git

RUN <<EOT
#!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

# cd /app-builder
cd /app-builder/secure-enclaves-framework

cargo build --all --release
# RUSTFLAGS='-C link-arg=-s' cargo build --all --release --target x86_64-unknown-linux-gnu
# RUSTFLAGS='-C link-arg=-s' cargo build --all --release --target x86_64-unknown-linux-musl
# RUSTFLAGS='-C target-feature=+crt-static' cargo build --all --release --target x86_64-unknown-linux-gnu
# RUSTFLAGS='-C target-feature=+crt-static' cargo build --all --release --target x86_64-unknown-linux-musl

mv -T /app-builder/secure-enclaves-framework/target/release/pipeline /app-builder/pipeline
# mv -T /app-builder/secure-enclaves-framework/target/x86_64-unknown-linux-gnu/release/pipeline /app-builder/pipeline
# mv -T /app-builder/secure-enclaves-framework/target/x86_64-unknown-linux-musl/release/pipeline /app-builder/pipeline
EOT

FROM scratch as app_build
COPY --from=builder /app-builder/pipeline /app-build/pipeline
