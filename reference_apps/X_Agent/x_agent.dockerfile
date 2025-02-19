# First stage for PDM dependencies
FROM public.ecr.aws/amazonlinux/amazonlinux:2023 as fetcher

RUN dnf upgrade -y
RUN dnf install -y python3 python3-pip git
# Install pdm


# Get the X Agent
WORKDIR /build
RUN git clone https://github.com/shivraj-sj/reference_apps.git


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
COPY --link network.init/pf-tp-guest.sh /apps/
COPY --link network.init/pf-guest.sh /apps/
COPY --link network.init/init.sh /apps/

RUN dnf upgrade -y

RUN dnf install -y kernel-libbpf systemd systemd-libs systemd-resolved initscripts
RUN dnf install -y /usr/bin/systemctl


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

RUN dnf install -y python3 python3-pip


RUN curl -sSL https://pdm-project.org/install-pdm.py | python3 -
COPY --from=fetcher /build/reference_apps/X_Agent /apps/X_Agent
RUN cd /apps/X_Agent && /root/.local/bin/pdm install



CMD tail -f /dev/null