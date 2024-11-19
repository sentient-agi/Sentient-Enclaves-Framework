FROM public.ecr.aws/amazonlinux/amazonlinux:2023 as builder
# https://gallery.ecr.aws/amazonlinux/amazonlinux

ENV SHELL="/usr/bin/env bash"

RUN dnf upgrade -y
RUN dnf install -y git gcc

ENV RUST_LOG="debug"
ENV RUST_BACKTRACE="full"

ENV CARGO_HOME="$HOME/rust" RUSTUP_HOME="$HOME/rustup" PATH="$PATH:$HOME/rust/bin"
RUN curl -fsSL https://sh.rustup.rs | bash -is -- -y --verbose --no-modify-path --default-toolchain stable --profile minimal
# RUN rustup -v toolchain install nightly --profile minimal
# RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /app-builder

# COPY --link oyster-tcp-proxy/ /app-builder/oyster-tcp-proxy/
RUN git clone -b master https://github.com/marlinprotocol/oyster-tcp-proxy.git

RUN <<EOT
#!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

# cd /app-builder
cd /app-builder/oyster-tcp-proxy

cargo build --all --release
# RUSTFLAGS='-C link-arg=-s' cargo build --all --release --target x86_64-unknown-linux-gnu
# RUSTFLAGS='-C link-arg=-s' cargo build --all --release --target x86_64-unknown-linux-musl
# RUSTFLAGS='-C target-feature=+crt-static' cargo build --all --release --target x86_64-unknown-linux-gnu
# RUSTFLAGS='-C target-feature=+crt-static' cargo build --all --release --target x86_64-unknown-linux-musl

mv -T /app-builder/oyster-tcp-proxy/target/release/ip-to-vsock /app-builder/ip-to-vsock
mv -T /app-builder/oyster-tcp-proxy/target/release/ip-to-vsock-transparent /app-builder/ip-to-vsock-transparent
mv -T /app-builder/oyster-tcp-proxy/target/release/port-to-vsock-transparent /app-builder/port-to-vsock-transparent
mv -T /app-builder/oyster-tcp-proxy/target/release/vsock-to-ip /app-builder/vsock-to-ip
mv -T /app-builder/oyster-tcp-proxy/target/release/vsock-to-ip-transparent /app-builder/vsock-to-ip-transparent

# mv -T /app-builder/aws-nitro-enclaves-image-format/target/x86_64-unknown-linux-gnu/release/eif_build /app-builder/eif_build
# mv -T /app-builder/aws-nitro-enclaves-image-format/target/x86_64-unknown-linux-musl/release/eif_build /app-builder/eif_build
EOT

FROM scratch as app_build
COPY --from=builder /app-builder/ip-to-vsock /app-build/ip-to-vsock
COPY --from=builder /app-builder/ip-to-vsock-transparent /app-build/ip-to-vsock-transparent
COPY --from=builder /app-builder/port-to-vsock-transparent /app-build/port-to-vsock-transparent
COPY --from=builder /app-builder/vsock-to-ip /app-build/vsock-to-ip
COPY --from=builder /app-builder/vsock-to-ip-transparent /app-build/vsock-to-ip-transparent
