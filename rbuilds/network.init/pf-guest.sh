##!/bin/bash
#!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

mkdir -vp /apps/socat/
mkdir -vp /apps/socat/.logs/
cd /apps/socat/

ip addr add 127.0.0.1/32 dev lo
ifconfig lo 127.0.0.1
ip link set dev lo up
ip route add default dev lo src 127.0.0.1
echo '127.0.0.1   localhost' | tee /etc/hosts
# echo '127.0.0.1   wttr.in' | tee -a /etc/hosts
echo 'nameserver 127.0.0.1' | tee /etc/resolv.conf

echo -e "socat PIDs:";
killall -v -9 socat;

## socat -dd TCP4-LISTEN:443,reuseaddr,fork VSOCK-CONNECT:3:8443 2>&1 | tee -a ./.logs/socat-localhost-vsock-https.output & disown

## socat -dd TCP4-LISTEN:80,reuseaddr,fork VSOCK-CONNECT:3:8080 2>&1 | tee -a ./.logs/socat-localhost-vsock-http.output & disown

## socat -dd UDP-LISTEN:53,reuseaddr,fork VSOCK-CONNECT:3:8053 2>&1 | tee -a ./.logs/socat-vsock-localhost-dns.output & disown
socat -dd UDP-LISTEN:53,reuseaddr,fork VSOCK-CONNECT:3:8053 >> ./.logs/socat-vsock-localhost-dns.output 2>&1 & disown

echo -e "socat PIDs:";
pidof socat;

