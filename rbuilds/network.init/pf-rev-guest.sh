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

# route incoming packets on VSock port 8443 to the VSock to ip:port traffic forwarding proxy
# listening on VSock port 8443 and addressing traffic to the local listening service on TCP port 8443

./vs2ip --vsock-addr 127:8443 --ip-addr 127.0.0.1:8443 >> ./.logs/vs2ip.https.output 2>&1 & disown
# ./vs2ip --vsock-addr 127:8443 --ip-addr 127.0.0.1:8443 2>&1 | tee -a ./.logs/vs2ip.https.output & disown

# route incoming packets on VSock port 8080 to the VSock to ip:port traffic forwarding proxy
# listening on VSock port 8080 and addressing traffic to the local listening service on TCP port 8080

./vs2ip --vsock-addr 127:8080 --ip-addr 127.0.0.1:8080 >> ./.logs/vs2ip.http.output 2>&1 & disown
# ./vs2ip --vsock-addr 127:8080 --ip-addr 127.0.0.1:8080 2>&1 | tee -a ./.logs/vs2ip.http.output & disown

# route incoming packets received on VSock ports range 9000:10000 to the traffic forwarding proxy (VSock:port to ip:port)
# listening on various VSock ports and addressing traffic to the local listening service/services on various TCP ports
# TLDR: listening exact vsock cid:port for various different incoming ports mapped from host port range to cid:ports
# and route traffic to exact service port, while service port is set exactly and manually

./vs2ip --vsock-addr 127:10001 --ip-addr 127.0.0.1:8443 >> ./.logs/vs2ip.allprotos.output 2>&1 & disown
# ./vs2ip --vsock-addr 127:10001 --ip-addr 127.0.0.1:8443 2>&1 | tee -a ./.logs/vs2ip.allprotos.output & disown
./vs2ip --vsock-addr 127:10000 --ip-addr 127.0.0.1:8443 >> ./.logs/vs2ip.allprotos.output 2>&1 & disown
# ./vs2ip --vsock-addr 127:10000 --ip-addr 127.0.0.1:8443 2>&1 | tee -a ./.logs/vs2ip.allprotos.output & disown
./vs2ip --vsock-addr 127:9999 --ip-addr 127.0.0.1:8443 >> ./.logs/vs2ip.allprotos.output 2>&1 & disown
# ./vs2ip --vsock-addr 127:9999 --ip-addr 127.0.0.1:8443 2>&1 | tee -a ./.logs/vs2ip.allprotos.output & disown
./vs2ip --vsock-addr 127:9443 --ip-addr 127.0.0.1:8443 >> ./.logs/vs2ip.allprotos.output 2>&1 & disown
# ./vs2ip --vsock-addr 127:9443 --ip-addr 127.0.0.1:8443 2>&1 | tee -a ./.logs/vs2ip.allprotos.output & disown
./vs2ip --vsock-addr 127:9080 --ip-addr 127.0.0.1:8443 >> ./.logs/vs2ip.allprotos.output 2>&1 & disown
# ./vs2ip --vsock-addr 127:9080 --ip-addr 127.0.0.1:8443 2>&1 | tee -a ./.logs/vs2ip.allprotos.output & disown
./vs2ip --vsock-addr 127:9001 --ip-addr 127.0.0.1:8443 >> ./.logs/vs2ip.allprotos.output 2>&1 & disown
# ./vs2ip --vsock-addr 127:9001 --ip-addr 127.0.0.1:8443 2>&1 | tee -a ./.logs/vs2ip.allprotos.output & disown
./vs2ip --vsock-addr 127:9000 --ip-addr 127.0.0.1:8443 >> ./.logs/vs2ip.allprotos.output 2>&1 & disown
# ./vs2ip --vsock-addr 127:9000 --ip-addr 127.0.0.1:8443 2>&1 | tee -a ./.logs/vs2ip.allprotos.output & disown

# route incoming packets received on VSock ports range 10000:11000 to the transparent traffic forwarding proxy (VSock to ip:port)
# listening on VSock port 11001 and addressing traffic to the local listening services on various TCP ports
# TLDR: listening exact vsock cid:port and route traffic to various different requested service ports from range,
# while service ports are adressed automatically, in port transparent mode

./vs2ip-tp --vsock-addr 127:11001 >> ./.logs/vs2ip-tp.allprotos.output 2>&1 & disown
# ./vs2ip-tp --vsock-addr 127:11001 2>&1 | tee -a ./.logs/vs2ip-tp.allprotos.output & disown

echo -e "vs2ip PIDs:";
pidof vs2ip; wait
echo -e "vs2ip-tp PIDs:";
pidof vs2ip-tp; wait

