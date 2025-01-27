##!/bin/bash
#!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

mkdir -vp ./pf-proxy/
mkdir -vp ./pf-proxy/.logs/
cd ./pf-proxy/

echo -e "vs2ip PIDs:";
killall -v -9 vs2ip;
echo -e "vs2ip-tp PIDs:";
killall -v -9 vs2ip-tp;

./vs2ip-tp --vsock-addr 3:8888 >> ./.logs/vs2ip-tp.allprotos.output 2>&1 & disown
## ./vs2ip-tp --vsock-addr 3:8888 2>&1 | tee -a ./.logs/vs2ip-tp.allprotos.output & disown

# ./vs2ip --vsock-addr 3:8443 --ip-addr 0.0.0.0:443 2>&1 | tee -a ./.logs/vs2ip.https.output & disown
# ./vs2ip --vsock-addr 3:8443 --ip-addr wttr.in:443 2>&1 | tee -a ./.logs/vs2ip.https.output & disown
## ./vs2ip-tp --vsock-addr 3:8443 2>&1 | tee -a ./.logs/vs2ip-tp.https.output & disown

# ./vs2ip --vsock-addr 3:8080 --ip-addr 0.0.0.0:80 2>&1 | tee -a ./.logs/vs2ip.http.output & disown
# ./vs2ip --vsock-addr 3:8080 --ip-addr wttr.in:80 2>&1 | tee -a ./.logs/vs2ip.http.output & disown
## ./vs2ip-tp --vsock-addr 3:8080 2>&1 | tee -a ./.logs/vs2ip-tp.http.output & disown

echo -e "vs2ip PIDs:";
pidof vs2ip;
echo -e "vs2ip-tp PIDs:";
pidof vs2ip-tp;

