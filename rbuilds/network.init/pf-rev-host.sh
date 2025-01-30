##!/bin/bash
#!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

mkdir -vp ./pf-proxy/
mkdir -vp ./pf-proxy/.logs/
cd ./pf-proxy/

sudo ip addr add 127.0.0.1/32 dev lo
sudo ifconfig lo 127.0.0.1
sudo ip link set dev lo up
sudo ip route add default dev lo src 127.0.0.1
# echo '127.0.0.1   localhost' | sudo tee /etc/hosts
## echo '127.0.0.1   wttr.in' | sudo tee -a /etc/hosts
# echo 'nameserver 127.0.0.1' | sudo tee /etc/resolv.conf

# sudo nft flush ruleset
sudo nft list ruleset | tee ./nft.ruleset.orig.out
echo
sudo iptables-save | tee ./iptables.ruleset.orig.out
echo

# route incoming packets on port 80 to the transparent proxy
# sudo iptables -A PREROUTING -t nat -p tcp --dport 80 -d 127.0.0.1 -i lo -j REDIRECT --to-port 8080
# sudo iptables -A PREROUTING -t nat -p tcp --dport 80 -d 127.0.0.1 -i lo -j DNAT --to-destination 127.0.0.1:8080
# sudo iptables -A OUTPUT -t nat -p tcp --dport 80 -d 127.0.0.1 -j REDIRECT --to-port 8080
sudo iptables -A OUTPUT -t nat -p tcp --dport 80 -d 127.0.0.1 -j DNAT --to-destination 127.0.0.1:8080
# route incoming packets on port 443 to the transparent proxy
# sudo iptables -A PREROUTING -t nat -p tcp --dport 443 -d 127.0.0.1 -i lo -j REDIRECT --to-port 8443
# sudo iptables -A PREROUTING -t nat -p tcp --dport 443 -d 127.0.0.1 -i lo -j DNAT --to-destination 127.0.0.1:8443
# sudo iptables -A OUTPUT -t nat -p tcp --dport 443 -d 127.0.0.1 -j REDIRECT --to-port 8443
sudo iptables -A OUTPUT -t nat -p tcp --dport 443 -d 127.0.0.1 -j DNAT --to-destination 127.0.0.1:8443
# route incoming packets on port 1025:65535 to the transparent proxy
# sudo iptables -A PREROUTING -t nat -p tcp --dport 9000:10000 -d 127.0.0.1 -i lo -j REDIRECT --to-port 10001
# sudo iptables -A PREROUTING -t nat -p tcp --dport 9000:10000 -d 127.0.0.1 -i lo -j DNAT --to-destination 127.0.0.1:10001
# sudo iptables -A OUTPUT -t nat -p tcp --dport 9000:10000 -d 127.0.0.1 -j REDIRECT --to-port 10001
sudo iptables -A OUTPUT -t nat -p tcp --dport 9000:10000 -d 127.0.0.1 -j DNAT --to-destination 127.0.0.1:10001

sudo nft list ruleset | tee ./nft.ruleset.out
# cat ./nft.ruleset.out | sudo nft -f -
echo
sudo iptables-save | tee ./iptables.ruleset.out
# cat ./iptables.ruleset.out | sudo iptables-restore -v[n]
# sudo iptables-apply -w ./iptables.ruleset.safe.out ./iptables.ruleset.out
echo

echo -e "ip2vs PIDs:";
killall -v -9 ip2vs;
echo -e "ip2vs-tp PIDs:";
killall -v -9 ip2vs-tp;
echo -e "tpp2vs PIDs:";
killall -v -9 tpp2vs;

./ip2vs --ip-addr 127.0.0.1:8443 --vsock-addr 127:8443 2>&1 | tee -a ./.logs/ip2vs.https.output & disown
# ./ip2vs-tp --ip-addr 127.0.0.1:8443 --vsock-addr 127:8443 2>&1 | tee -a ./.logs/ip2vs-tp.https.output & disown

./ip2vs --ip-addr 127.0.0.1:8080 --vsock-addr 127:8080 2>&1 | tee -a ./.logs/ip2vs.http.output & disown
# ./ip2vs-tp --ip-addr 127.0.0.1:8080 --vsock-addr 127:8080 2>&1 | tee -a ./.logs/ip2vs-tp.http.output & disown

./tpp2vs --ip-addr 127.0.0.1:10001 --vsock 127 2>&1 | tee -a ./.logs/tpp2vs.allprotos.output & disown

echo -e "ip2vs PIDs:";
pidof ip2vs;
echo -e "ip2vs-tp PIDs:";
pidof ip2vs-tp;
echo -e "tpp2vs PIDs:";
pidof tpp2vs;

