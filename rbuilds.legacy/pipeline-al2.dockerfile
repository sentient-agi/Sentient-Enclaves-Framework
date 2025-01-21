FROM public.ecr.aws/amazonlinux/amazonlinux:2 as builder
# https://gallery.ecr.aws/amazonlinux/amazonlinux

ENV SHELL="/usr/bin/env bash"

RUN yum upgrade -y
RUN amazon-linux-extras enable epel
RUN yum clean -y metadata && yum install -y epel-release
RUN yum install -y git gcc pkgconfig openssl openssl-devel openssl-libs

ENV RUST_LOG="debug"
ENV RUST_BACKTRACE="full"

ENV CARGO_HOME="$HOME/rust" RUSTUP_HOME="$HOME/rustup" PATH="$PATH:$HOME/rust/bin"
RUN curl -fsSL https://sh.rustup.rs | bash -is -- -y --verbose --no-modify-path --default-toolchain stable --profile minimal
# RUN rustup -v toolchain install nightly --profile minimal

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
cargo build --release --all
mv -T /app-builder/secure-enclaves-framework/target/release/pipeline /app-builder/pipeline
mkdir -p /app-builder/.config/
mv -T /app-builder/secure-enclaves-framework/pipeline/.config/config.toml /app-builder/.config/config.toml
EOT

FROM public.ecr.aws/amazonlinux/amazonlinux:2 as enclave_app

ENV SHELL="/usr/bin/env bash"

WORKDIR /app

RUN mkdir -p /app/
RUN mkdir -p /app/.config/
COPY --from=builder /app-builder/pipeline /app/pipeline
COPY --from=builder /app-builder/.config/config.toml /app/.config/config.toml

RUN yum upgrade -y

RUN yum install -y kernel-libbpf systemd systemd-libs systemd-resolved initscripts
RUN yum install -y /usr/bin/systemctl
# RUN yum install -y /bin/systemctl
# init=/sbin/init
# init=/usr/sbin/init
# init=/lib/systemd/systemd
# init=/usr/lib/systemd/systemd

RUN yum install -y sudo time which hostname tar bsdtar cpio findutils pciutils procps-ng
RUN yum install -y iputils iproute dnsmasq bind bind-utils bind-dnssec-utils traceroute net-tools socat nc nmap-ncat
# RUN yum install -y kernel kernel-devel kernel-modules-extra kernel-modules-extra-common
RUN yum install -y kmod kmod-libs
RUN yum install -y nftables iptables iptables-nft iptables-libs iptables-utils iptables-legacy iptables-legacy-libs
RUN yum install -y lsof perf iperf iperf3
RUN yum install -y curl
RUN yum install -y jq wget openssh git rsync
RUN yum install -y lynx w3m
RUN yum install -y awscli

# ARG FS=0
# ENV FS=${FS}

# ENV RUST_LOG="pipeline=debug"
ENV RUST_LOG="debug"
ENV RUST_BACKTRACE="full"
CMD cd /app/; ./pipeline listen --port 53000 >> /app/pipeline.log 2>&1 & disown && tail -f /app/pipeline.log
