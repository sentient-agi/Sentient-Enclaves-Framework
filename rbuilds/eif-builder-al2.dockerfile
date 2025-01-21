FROM public.ecr.aws/amazonlinux/amazonlinux:2 as builder
# https://gallery.ecr.aws/amazonlinux/amazonlinux

ENV SHELL="/usr/bin/env bash"

RUN yum upgrade -y
RUN amazon-linux-extras enable epel
RUN yum clean -y metadata && yum install -y epel-release
RUN yum install -y git gcc pkgconfig openssl openssl-devel openssl-libs

RUN mkdir -p /eif_builder/
WORKDIR /eif_builder

COPY --link patterns /eif_builder/
COPY --link ./eif_build/eif_build /eif_builder/

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
RUN yum install -y aws-nitro-enclaves-cli aws-nitro-enclaves-cli-devel

# ARG FS=0
# ENV FS=${FS}

# CMD whoami; uname -a; date; pwd;
# CMD sleep infinity
CMD tail -f /dev/null
