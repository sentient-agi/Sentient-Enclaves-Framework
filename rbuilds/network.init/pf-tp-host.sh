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
killall -v -9 vs2ip; wait
echo -e "vs2ip-tp PIDs:";
killall -v -9 vs2ip-tp; wait

./vs2ip-tp --vsock-addr 3:8888 >> ./.logs/vs2ip-tp.allprotos.output 2>&1 & disown
# ./vs2ip-tp --vsock-addr 3:8888 2>&1 | tee -a ./.logs/vs2ip-tp.allprotos.output & disown

# ./vs2ip-tp --vsock-addr 3:8443 >> ./.logs/vs2ip-tp.https.output 2>&1 & disown
# ./vs2ip-tp --vsock-addr 3:8443 2>&1 | tee -a ./.logs/vs2ip-tp.https.output & disown

# ./vs2ip-tp --vsock-addr 3:8080 >> ./.logs/vs2ip-tp.http.output 2>&1 & disown
# ./vs2ip-tp --vsock-addr 3:8080 2>&1 | tee -a ./.logs/vs2ip-tp.http.output & disown

echo -e "vs2ip PIDs:";
pidof vs2ip; wait
echo -e "vs2ip-tp PIDs:";
pidof vs2ip-tp; wait

