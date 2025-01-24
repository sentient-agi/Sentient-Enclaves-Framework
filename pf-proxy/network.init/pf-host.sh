#!/bin/bash
##!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

mkdir -vp ./socat/
mkdir -vp ./socat/.logs/
cd ./socat/

echo -e "socat PIDs:";
killall -v -9 socat;

# socat -dd VSOCK-LISTEN:8443,crlf,reuseaddr,fork SYSTEM:"echo HTTP/1.0 200; echo Content-Type\: text/plain; echo; echo Hello from host\!" 2>&1 | tee -a ./.logs/socat-vsock-localhost-https.output & disown
# socat -dd VSOCK-LISTEN:8443,crlf,reuseaddr,fork SYSTEM:"echo HTTP/1.0 200; echo Content-Type\: text/plain; echo; echo Hello from host\!; cat ./cats" 2>&1 | tee -a ./.logs/socat-vsock-localhost-https.output & disown
# socat -dd VSOCK-LISTEN:8443,reuseaddr,fork TCP:0.0.0.0:443 2>&1 | tee -a ./.logs/socat-vsock-localhost-https.output & disown
## socat -dd VSOCK-LISTEN:8443,reuseaddr,fork TCP:wttr.in:443 2>&1 | tee -a ./.logs/socat-vsock-localhost-https.output & disown

# socat -dd VSOCK-LISTEN:8080,crlf,reuseaddr,fork SYSTEM:"echo HTTP/1.0 200; echo Content-Type\: text/plain; echo; echo Hello from host\!" 2>&1 | tee -a ./.logs/socat-vsock-localhost-http.output & disown
# socat -dd VSOCK-LISTEN:8080,crlf,reuseaddr,fork SYSTEM:"echo HTTP/1.0 200; echo Content-Type\: text/plain; echo; echo Hello from host\!; cat ./cats" 2>&1 | tee -a ./.logs/socat-vsock-localhost-http.output & disown
# socat -dd VSOCK-LISTEN:8080,reuseaddr,fork TCP:0.0.0.0:80 2>&1 | tee -a ./.logs/socat-vsock-localhost-http.output & disown
## socat -dd VSOCK-LISTEN:8080,reuseaddr,fork TCP:wttr.in:80 2>&1 | tee -a ./.logs/socat-vsock-localhost-http.output & disown

## socat -dd VSOCK-LISTEN:8053,reuseaddr,fork UDP:$(cat /etc/resolv.conf | grep -iPo "^(nameserver\s*?)\K([0-9.]*?)$"):53 2>&1 | tee -a ./.logs/socat-vsock-localhost-dns.output & disown
socat -dd VSOCK-LISTEN:8053,reuseaddr,fork UDP:$(cat /etc/resolv.conf | grep -iPo "^(nameserver\s*?)\K([0-9.]*?)$"):53 >> ./.logs/socat-vsock-localhost-dns.output 2>&1 & disown

echo -e "socat PIDs:";
pidof socat;

