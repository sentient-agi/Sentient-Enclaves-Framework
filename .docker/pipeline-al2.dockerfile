FROM public.ecr.aws/amazonlinux/amazonlinux:2 as builder
# https://gallery.ecr.aws/amazonlinux/amazonlinux

ENV SHELL="/usr/bin/env bash"

RUN yum upgrade -y
RUN amazon-linux-extras enable epel
RUN yum clean -y metadata && yum install -y epel-release
RUN yum install -y git gcc pkgconfig openssl openssl-devel openssl-libs perl perl-FindBin
RUN yum install -y time which hostname

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
cargo build --release --all
mv -T /app-builder/secure-enclaves-framework/target/release/pipeline /app-builder/pipeline
mv -T /app-builder/secure-enclaves-framework/target/release/ra-web-srv /app-builder/ra-web-srv
mkdir -p /app-builder/.config/
mv -T /app-builder/secure-enclaves-framework/pipeline/.config/pipeline.config.toml /app-builder/.config/pipeline.config.toml
mv -T /app-builder/secure-enclaves-framework/ra-web-srv/.config/ra_web_srv.config.toml /app-builder/.config/ra_web_srv.config.toml
mkdir -p /app-builder/certs/
cp -vrf /app-builder/secure-enclaves-framework/ra-web-srv/certs/ -T /app-builder/certs/
EOT

FROM public.ecr.aws/amazonlinux/amazonlinux:2 as enclave_app

ENV SHELL="/usr/bin/env bash"

WORKDIR /apps

RUN mkdir -p /apps/
RUN mkdir -p /apps/.config/
RUN mkdir -p /apps/.logs/
RUN mkdir -p /apps/certs/
COPY --from=builder /app-builder/pipeline /apps/pipeline
COPY --from=builder /app-builder/.config/pipeline.config.toml /apps/.config/pipeline.config.toml
COPY --from=builder /app-builder/ra-web-srv /apps/ra-web-srv
COPY --from=builder /app-builder/.config/ra_web_srv.config.toml /apps/.config/ra_web_srv.config.toml
COPY --from=builder /app-builder/certs/ /apps/certs/

RUN yum upgrade -y

RUN yum install -y kernel-libbpf systemd systemd-libs systemd-resolved initscripts
RUN yum install -y /usr/bin/systemctl
# RUN yum install -y /bin/systemctl
# init=/sbin/init
# init=/usr/sbin/init
# init=/lib/systemd/systemd
# init=/usr/lib/systemd/systemd

RUN yum install -y sudo time which hostname tar bsdtar cpio findutils pcre-tools pciutils procps-ng
RUN yum install -y iputils iproute dnsmasq bind bind-utils bind-dnssec-utils traceroute net-tools socat nc nmap-ncat
# RUN yum install -y kernel kernel-devel kernel-modules-extra kernel-modules-extra-common
RUN yum install -y kmod kmod-libs
RUN yum install -y nftables iptables iptables-nft iptables-libs iptables-utils iptables-legacy iptables-legacy-libs
RUN yum install -y lsof perf iperf iperf3
RUN yum install -y curl
RUN yum install -y jq wget openssh git rsync
RUN yum install -y lynx w3m
RUN yum install -y awscli

# ENV RUST_LOG="pipeline=debug"
ENV RUST_LOG="debug"
ENV RUST_BACKTRACE="full"
ENV CERT_DIR="/apps/certs/"
CMD cd /apps/; ./pipeline listen --port 53000 >> /apps/.logs/pipeline.log 2>&1 & disown; ./ra-web-srv >> /apps/.logs/ra-web-srv.log 2>&1 & disown; tail -f /apps/.logs/pipeline.log
