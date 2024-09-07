FROM public.ecr.aws/amazonlinux/amazonlinux:2 as builder

RUN yum upgrade -y
RUN amazon-linux-extras enable epel
RUN yum clean -y metadata && yum install -y epel-release
RUN yum install -y gcc git

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
EOT

FROM scratch
COPY --from=builder /app-builder/pipeline /pipeline
