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

nft flush ruleset

# route outgoing packets with a destination other than localhost to a given ip:port
iptables -t nat -A OUTPUT -p tcp --dport 1:65535 ! -d 127.0.0.1 -j DNAT --to-destination 127.0.0.1:8888
# iptables -t nat -A OUTPUT -p tcp --dport 443 ! -d 127.0.0.1 -j DNAT --to-destination 127.0.0.1:8443
# iptables -t nat -A OUTPUT -p tcp --dport 80 ! -d 127.0.0.1 -j DNAT --to-destination 127.0.0.1:8080

# replace the source address with 127.0.0.1 for outgoing packets with a source of 0.0.0.0
# ensures returning packets have 127.0.0.1 as the destination and not 0.0.0.0
iptables -t nat -A POSTROUTING -o lo -s 0.0.0.0 -j SNAT --to-source 127.0.0.1

nft list ruleset
echo
iptables-save
echo

echo -e "ip2vs PIDs:";
killall -v -9 ip2vs; wait
echo -e "ip2vs-tp PIDs:";
killall -v -9 ip2vs-tp; wait
echo -e "tpp2vs PIDs:";
killall -v -9 tpp2vs; wait

./ip2vs-tp --ip-addr 127.0.0.1:8888 --vsock-addr 3:8888 >> ./.logs/ip2vs-tp.allprotos.output 2>&1 & disown
# ./ip2vs-tp --ip-addr 127.0.0.1:8888 --vsock-addr 3:8888 2>&1 | tee -a ./.logs/ip2vs-tp.allprotos.output & disown

# ./ip2vs-tp --ip-addr 127.0.0.1:8443 --vsock-addr 3:8443 >> ./.logs/ip2vs-tp.https.output 2>&1 & disown
# ./ip2vs-tp --ip-addr 127.0.0.1:8443 --vsock-addr 3:8443 2>&1 | tee -a ./.logs/ip2vs-tp.https.output & disown

# ./ip2vs-tp --ip-addr 127.0.0.1:8080 --vsock-addr 3:8080 >> ./.logs/ip2vs-tp.http.output 2>&1 & disown
# ./ip2vs-tp --ip-addr 127.0.0.1:8080 --vsock-addr 3:8080 2>&1 | tee -a ./.logs/ip2vs-tp.http.output & disown

echo -e "ip2vs PIDs:";
pidof ip2vs; wait
echo -e "ip2vs-tp PIDs:";
pidof ip2vs-tp; wait
echo -e "tpp2vs PIDs:";
pidof tpp2vs; wait

