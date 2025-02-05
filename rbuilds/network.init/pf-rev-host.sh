##!/bin/bash
#!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

mkdir -vp ./pf-proxy/
mkdir -vp ./pf-proxy/.logs/
cd ./pf-proxy/

sudo nft list ruleset | tee ./nft.ruleset.orig.out
# sudo nft flush ruleset
# cat ./nft.ruleset.orig.out | sudo nft -f -
echo
sudo iptables-save | tee ./iptables.ruleset.orig.out
# cat ./iptables.ruleset.orig.out | sudo iptables-restore -v[n]
# sudo iptables-apply -w ./iptables.ruleset.orig.safe.out ./iptables.ruleset.orig.out
echo

# route incoming packets on port 80 to the ip:port to VSock traffic forwarding proxy listening on port 8080

# sudo iptables -A PREROUTING -t nat -p tcp --dport 80 -d 127.0.0.1 -i lo -j REDIRECT --to-ports 8080
# sudo iptables -A PREROUTING -t nat -p tcp --dport 80 -j REDIRECT --to-ports 8080
# sudo iptables -A PREROUTING -t nat -p tcp --dport 80 -d 127.0.0.1 -i lo -j DNAT --to-destination 127.0.0.1:8080
# sudo iptables -A PREROUTING -t nat -p tcp --dport 80 -j DNAT --to-destination 127.0.0.1:8080

# sudo iptables -A OUTPUT -t nat -p tcp --dport 80 -d 127.0.0.1 -j REDIRECT --to-ports 8080
# sudo iptables -A OUTPUT -t nat -p tcp --dport 80 -j REDIRECT --to-ports 8080
sudo iptables -A OUTPUT -t nat -p tcp --dport 80 -d 127.0.0.1 -j DNAT --to-destination 127.0.0.1:8080
# sudo iptables -A OUTPUT -t nat -p tcp --dport 80 -j DNAT --to-destination 127.0.0.1:8080

# route incoming packets on port 443 to the ip:port to VSock traffic forwarding proxy listening on port 8443

# sudo iptables -A PREROUTING -t nat -p tcp --dport 443 -d 127.0.0.1 -i lo -j REDIRECT --to-ports 8443
# sudo iptables -A PREROUTING -t nat -p tcp --dport 443 -j REDIRECT --to-ports 8443
# sudo iptables -A PREROUTING -t nat -p tcp --dport 443 -d 127.0.0.1 -i lo -j DNAT --to-destination 127.0.0.1:8443
# sudo iptables -A PREROUTING -t nat -p tcp --dport 443 -j DNAT --to-destination 127.0.0.1:8443

# sudo iptables -A OUTPUT -t nat -p tcp --dport 443 -d 127.0.0.1 -j REDIRECT --to-ports 8443
# sudo iptables -A OUTPUT -t nat -p tcp --dport 443 -j REDIRECT --to-ports 8443
sudo iptables -A OUTPUT -t nat -p tcp --dport 443 -d 127.0.0.1 -j DNAT --to-destination 127.0.0.1:8443
# sudo iptables -A OUTPUT -t nat -p tcp --dport 443 -j DNAT --to-destination 127.0.0.1:8443

# route incoming packets on ports 9000:10000 to the transparent port to VSock traffic forwarding proxy listening on port 10001

# sudo iptables -A PREROUTING -t nat -p tcp --dport 9000:10000 -d 127.0.0.1 -i lo -j REDIRECT --to-ports 10001
# sudo iptables -A PREROUTING -t nat -p tcp --dport 9000:10000 -j REDIRECT --to-ports 10001
# sudo iptables -A PREROUTING -t nat -p tcp --dport 9000:10000 -d 127.0.0.1 -i lo -j DNAT --to-destination 127.0.0.1:10001
# sudo iptables -A PREROUTING -t nat -p tcp --dport 9000:10000 -j DNAT --to-destination 127.0.0.1:10001

# sudo iptables -A OUTPUT -t nat -p tcp --dport 9000:10000 -d 127.0.0.1 -j REDIRECT --to-ports 10001
# sudo iptables -A OUTPUT -t nat -p tcp --dport 9000:10000 -j REDIRECT --to-ports 10001
sudo iptables -A OUTPUT -t nat -p tcp --dport 9000:10000 -d 127.0.0.1 -j DNAT --to-destination 127.0.0.1:10001
# sudo iptables -A OUTPUT -t nat -p tcp --dport 9000:10000 -j DNAT --to-destination 127.0.0.1:10001

# route incoming packets on ports 10000:11000 to the transparent ip:port to VSock traffic forwarding proxy listening on port 11001

# sudo iptables -A PREROUTING -t nat -p tcp --dport 10000:11000 -d 127.0.0.1 -i lo -j REDIRECT --to-ports 11001
# sudo iptables -A PREROUTING -t nat -p tcp --dport 10000:11000 -j REDIRECT --to-ports 11001
# sudo iptables -A PREROUTING -t nat -p tcp --dport 10000:11000 -d 127.0.0.1 -i lo -j DNAT --to-destination 127.0.0.1:11001
# sudo iptables -A PREROUTING -t nat -p tcp --dport 10000:11000 -j DNAT --to-destination 127.0.0.1:11001

# sudo iptables -A OUTPUT -t nat -p tcp --dport 10000:11000 -d 127.0.0.1 -j REDIRECT --to-ports 11001
# sudo iptables -A OUTPUT -t nat -p tcp --dport 10000:11000 -j REDIRECT --to-ports 11001
sudo iptables -A OUTPUT -t nat -p tcp --dport 10000:11000 -d 127.0.0.1 -j DNAT --to-destination 127.0.0.1:11001
# sudo iptables -A OUTPUT -t nat -p tcp --dport 10000:11000 -j DNAT --to-destination 127.0.0.1:11001

sudo nft list ruleset | tee ./nft.ruleset.out
# sudo nft flush ruleset
# cat ./nft.ruleset.out | sudo nft -f -
echo
sudo iptables-save | tee ./iptables.ruleset.out
# cat ./iptables.ruleset.out | sudo iptables-restore -v[n]
# sudo iptables-apply -w ./iptables.ruleset.safe.out ./iptables.ruleset.out
echo

echo -e "ip2vs PIDs:";
killall -v -9 ip2vs; wait
echo -e "ip2vs-tp PIDs:";
killall -v -9 ip2vs-tp; wait
echo -e "tpp2vs PIDs:";
killall -v -9 tpp2vs; wait

# route incoming packets on port 443 to the ip:port to VSock traffic forwarding proxy listening on port 8443

./ip2vs --ip-addr 127.0.0.1:8443 --vsock-addr 127:8443 >> ./.logs/ip2vs.https.output 2>&1 & disown
# ./ip2vs --ip-addr 127.0.0.1:8443 --vsock-addr 127:8443 2>&1 | tee -a ./.logs/ip2vs.https.output & disown

# route incoming packets on port 80 to the ip:port to VSock traffic forwarding proxy listening on port 8080

./ip2vs --ip-addr 127.0.0.1:8080 --vsock-addr 127:8080 >> ./.logs/ip2vs.http.output 2>&1 & disown
# ./ip2vs --ip-addr 127.0.0.1:8080 --vsock-addr 127:8080 2>&1 | tee -a ./.logs/ip2vs.http.output & disown

# route incoming packets on ports 9000:10000 to the transparent port to VSock traffic forwarding proxy listening on port 10001

./tpp2vs --ip-addr 127.0.0.1:10001 --vsock 127 >> ./.logs/tpp2vs.allprotos.output 2>&1 & disown
# ./tpp2vs --ip-addr 127.0.0.1:10001 --vsock 127 2>&1 | tee -a ./.logs/tpp2vs.allprotos.output & disown

# route incoming packets on ports 10000:11000 to the transparent ip:port to VSock traffic forwarding proxy listening on port 11001

./ip2vs-tp --ip-addr 127.0.0.1:11001 --vsock-addr 127:11001 >> ./.logs/ip2vs-tp.allprotos.output 2>&1 & disown
# ./ip2vs-tp --ip-addr 127.0.0.1:11001 --vsock-addr 127:11001 2>&1 | tee -a ./.logs/ip2vs-tp.allprotos.output & disown

echo -e "ip2vs PIDs:";
pidof ip2vs; wait
echo -e "ip2vs-tp PIDs:";
pidof ip2vs-tp; wait
echo -e "tpp2vs PIDs:";
pidof tpp2vs; wait

