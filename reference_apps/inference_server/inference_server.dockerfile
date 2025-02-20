FROM public.ecr.aws/amazonlinux/amazonlinux:2023 as server_builder

ENV SHELL="/usr/bin/env bash"

RUN dnf upgrade -y
RUN dnf install -y git gcc pkgconfig openssl openssl-devel openssl-libs
RUN dnf install -y time which hostname
RUN dnf install -y clang clang-devel clang-libs llvm-devel cmake make

ENV RUST_LOG="debug"
ENV RUST_BACKTRACE="full"

ENV CARGO_HOME="$HOME/rust" RUSTUP_HOME="$HOME/rustup" PATH="$PATH:$HOME/rust/bin"
RUN curl -fsSL https://sh.rustup.rs | bash -is -- -y --verbose --no-modify-path --default-toolchain stable --profile minimal
# RUN rustup -v toolchain install nightly --profile minimal
# RUN rustup target add x86_64-unknown-linux-musl


# Specify path relative to the rbuilds.sh script
WORKDIR /app-builder

# Copy the source code
RUN git clone https://github.com/shivraj-sj/reference_apps.git


RUN cd /app-builder/reference_apps/inference_server && \
    cargo build --release

FROM public.ecr.aws/amazonlinux/amazonlinux:2023 as enclave_app
# https://gallery.ecr.aws/amazonlinux/amazonlinux

ENV SHELL="/usr/bin/env bash"
# ENV RUST_LOG="pipeline=debug"
ENV RUST_LOG="debug"
ENV RUST_BACKTRACE="full"

WORKDIR /apps

RUN mkdir -vp /apps/
RUN mkdir -vp /apps/.config/
RUN mkdir -vp /apps/.logs/
COPY --link secure-enclaves-framework/pipeline /apps/
COPY --link secure-enclaves-framework/.config/config.toml /apps/.config/
RUN mkdir -vp /apps/pf-proxy/
RUN mkdir -vp /apps/pf-proxy/.logs/
RUN mkdir -vp /apps/socat/.logs/
COPY --link secure-enclaves-framework/ip-to-vsock-transparent /apps/pf-proxy/ip2vs-tp
COPY --link secure-enclaves-framework/vsock-to-ip-transparent /apps/pf-proxy/vs2ip-tp
COPY --link secure-enclaves-framework/vsock-to-ip /apps/pf-proxy/vs2ip
COPY --link network.init/pf-rev-guest.sh /apps/
COPY --link network.init/pf-tp-guest.sh /apps/
COPY --link network.init/pf-guest.sh /apps/
COPY --link network.init/init.sh /apps/

RUN dnf upgrade -y

RUN dnf install -y kernel-libbpf systemd systemd-libs systemd-resolved initscripts
RUN dnf install -y /usr/bin/systemctl
# RUN dnf install -y /bin/systemctl
# init=/sbin/init
# init=/usr/sbin/init
# init=/lib/systemd/systemd
# init=/usr/lib/systemd/systemd

RUN dnf install -y sudo time which hostname tar bsdtar cpio findutils pcre-tools pciutils procps-ng
RUN dnf install -y iputils iproute dnsmasq bind bind-utils bind-dnssec-utils traceroute net-tools socat nc nmap-ncat
# RUN dnf install -y kernel kernel-devel kernel-modules-extra kernel-modules-extra-common
RUN dnf install -y kmod kmod-libs
RUN dnf install -y nftables iptables iptables-nft iptables-libs iptables-utils iptables-legacy iptables-legacy-libs
RUN dnf install -y lsof perf iperf iperf3
RUN dnf install -y --allowerasing curl
RUN dnf install -y jq wget openssh git rsync
RUN dnf install -y lynx w3m
RUN dnf install -y awscli

# Copy the server binary
COPY --from=server_builder /app-builder/reference_apps/inference_server/target/release/inference_server /apps/inference_server

# ARG FS=0
# ENV FS=${FS}

# CMD whoami; uname -a; date; pwd;
# CMD sleep infinity
CMD tail -f /dev/null
