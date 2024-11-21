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

WORKDIR /app-builder

# COPY --link pipeline-tee/ /app-builder/pipeline-tee.rs/
RUN git clone -b main https://github.com/andrcmdr/pipeline-tee.rs.git

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

FROM public.ecr.aws/amazonlinux/amazonlinux:2023 as enclave_app

ENV SHELL="/usr/bin/env bash"

WORKDIR /app

RUN mkdir -p /app/
RUN mkdir -p /app/.config/
COPY --from=builder /app-builder/pipeline /app/pipeline
COPY --from=builder /app-builder/.config/config.toml /app/.config/config.toml

RUN dnf upgrade -y

RUN dnf install -y kernel-libbpf systemd systemd-libs systemd-resolved initscripts
RUN dnf install -y /usr/bin/systemctl
# RUN dnf install -y /bin/systemctl
# init=/sbin/init
# init=/usr/sbin/init
# init=/lib/systemd/systemd
# init=/usr/lib/systemd/systemd

RUN dnf install -y sudo time which hostname tar bsdtar cpio findutils pciutils procps-ng
RUN dnf install -y iputils iproute dnsmasq bind bind-utils bind-dnssec-utils traceroute net-tools socat nc nmap-ncat
# RUN dnf install -y kernel kernel-devel kernel-modules-extra kernel-modules-extra-common
RUN dnf install -y kmod kmod-libs
RUN dnf install -y nftables iptables iptables-nft iptables-libs iptables-utils iptables-legacy iptables-legacy-libs
RUN dnf install -y lsof perf iperf iperf3
RUN dnf install -y --allowerasing curl
RUN dnf install -y jq wget openssh git rsync
RUN dnf install -y lynx w3m
RUN dnf install -y awscli

# ARG FS=0
# ENV FS=${FS}

# ENV RUST_LOG="pipeline=debug"
ENV RUST_LOG="debug"
ENV RUST_BACKTRACE="full"
CMD cd /app/; ./pipeline listen --port 53000 >> /app/pipeline.log 2>&1 & disown && tail -f /app/pipeline.log
