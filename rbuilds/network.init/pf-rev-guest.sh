##!/bin/bash
#!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

mkdir -vp /apps/pf-proxy/
mkdir -vp /apps/pf-proxy/.logs/
cd /apps/pf-proxy/

ip addr add 127.0.0.1/32 dev lo
ifconfig lo 127.0.0.1
ip link set dev lo up
ip route add default dev lo src 127.0.0.1
echo '127.0.0.1   localhost' | tee /etc/hosts
echo 'nameserver 127.0.0.1' | tee /etc/resolv.conf

nft list ruleset | tee ./nft.ruleset.orig.out
# nft flush ruleset
# cat ./nft.ruleset.orig.out | nft -f -
echo
iptables-save | tee ./iptables.ruleset.orig.out
# cat ./iptables.ruleset.orig.out | iptables-restore -vn
# iptables-apply -w ./iptables.ruleset.orig.safe.out ./iptables.ruleset.orig.out
echo

nft list ruleset | tee ./nft.ruleset.out
# nft flush ruleset
# cat ./nft.ruleset.out | nft -f -
echo
iptables-save | tee ./iptables.ruleset.out
# cat ./iptables.ruleset.out | iptables-restore -vn
# iptables-apply -w ./iptables.ruleset.safe.out ./iptables.ruleset.out
echo

echo -e "vs2ip PIDs:";
killall -v -9 vs2ip; wait
echo -e "vs2ip-tp PIDs:";
killall -v -9 vs2ip-tp; wait

./vs2ip --vsock-addr 127:8443 --ip-addr 127.0.0.1:8443 >> ./.logs/vs2ip.https.output 2>&1 & disown
# ./vs2ip --vsock-addr 127:8443 --ip-addr 127.0.0.1:8443 2>&1 | tee -a ./.logs/vs2ip.https.output & disown

# ./vs2ip-tp --vsock-addr 127:8443 >> ./.logs/vs2ip-tp.https.output 2>&1 & disown
# ./vs2ip-tp --vsock-addr 127:8443 2>&1 | tee -a ./.logs/vs2ip-tp.https.output & disown

./vs2ip --vsock-addr 127:8080 --ip-addr 127.0.0.1:8080 >> ./.logs/vs2ip.http.output 2>&1 & disown
# ./vs2ip --vsock-addr 127:8080 --ip-addr 127.0.0.1:8080 2>&1 | tee -a ./.logs/vs2ip.http.output & disown

# ./vs2ip-tp --vsock-addr 127:8080 >> ./.logs/vs2ip-tp.http.output 2>&1 & disown
# ./vs2ip-tp --vsock-addr 127:8080 2>&1 | tee -a ./.logs/vs2ip-tp.http.output & disown

./vs2ip-tp --vsock-addr 127:10001 >> ./.logs/vs2ip-tp.allprotos.output 2>&1 & disown
# ./vs2ip-tp --vsock-addr 127:10001 2>&1 | tee -a ./.logs/vs2ip-tp.allprotos.output & disown

echo -e "vs2ip PIDs:";
pidof vs2ip; wait
echo -e "vs2ip-tp PIDs:";
pidof vs2ip-tp; wait

