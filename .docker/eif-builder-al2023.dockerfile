FROM public.ecr.aws/amazonlinux/amazonlinux:2023 as builder
# https://gallery.ecr.aws/amazonlinux/amazonlinux

ENV SHELL="/usr/bin/env bash"

RUN dnf upgrade -y
RUN dnf install -y git gcc pkgconfig openssl openssl-devel openssl-libs

ENV RUST_LOG="debug"
ENV RUST_BACKTRACE="full"

ENV CARGO_HOME="$HOME/rust" RUSTUP_HOME="$HOME/rustup" PATH="$PATH:$HOME/rust/bin"
RUN curl -fsSL https://sh.rustup.rs | bash -is -- -y --verbose --no-modify-path --default-toolchain stable --profile minimal
# RUN rustup -v toolchain install nightly --profile minimal
# RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /app-builder

# COPY --link aws-nitro-enclaves-image-format/ /app-builder/aws-nitro-enclaves-image-format/
# RUN git clone -b main https://github.com/aws/aws-nitro-enclaves-image-format.git
RUN git clone -b main https://github.com/maayank/aws-nitro-enclaves-image-format.git

RUN <<EOT
#!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

# cd /app-builder
cd /app-builder/aws-nitro-enclaves-image-format/eif_build

cargo build --all --release
# RUSTFLAGS='-C link-arg=-s' cargo build --all --release --target x86_64-unknown-linux-gnu
# RUSTFLAGS='-C link-arg=-s' cargo build --all --release --target x86_64-unknown-linux-musl
# RUSTFLAGS='-C target-feature=+crt-static' cargo build --all --release --target x86_64-unknown-linux-gnu
# RUSTFLAGS='-C target-feature=+crt-static' cargo build --all --release --target x86_64-unknown-linux-musl

mv -T /app-builder/aws-nitro-enclaves-image-format/target/release/eif_build /app-builder/eif_build
mv -T /app-builder/aws-nitro-enclaves-image-format/target/release/eif_extract /app-builder/eif_extract
# mv -T /app-builder/aws-nitro-enclaves-image-format/target/x86_64-unknown-linux-gnu/release/eif_build /app-builder/eif_build
# mv -T /app-builder/aws-nitro-enclaves-image-format/target/x86_64-unknown-linux-musl/release/eif_build /app-builder/eif_build
EOT

FROM scratch as app_build
COPY --from=builder /app-builder/eif_build /app-build/eif_build
COPY --from=builder /app-builder/eif_extract /app-build/eif_extract
