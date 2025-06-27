##!/bin/bash
#!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

echo -e "Init.sh started";

# mkdir -vp /apps/.logs/;

echo -e "Activating the localhost interface for enclave's service bus";

ip addr add 127.0.0.1/32 dev lo
ifconfig lo 127.0.0.1
ip link set dev lo up
ip route add default dev lo src 127.0.0.1
echo '127.0.0.1   localhost' | tee /etc/hosts
echo 'nameserver 127.0.0.1' | tee /etc/resolv.conf

echo -e "Executing Pipeline";
cd /apps/;
./pipeline listen --port 53000 >> /apps/.logs/pipeline.log 2>&1 & disown;

echo -e "Executing NATS Server for Enclave Bus";
./nats-server --name "enclave_bus_nats_server"  --addr 127.0.0.1 --port 4222 --http_port 4242 --config ./.config/nats.config --log_size_limit 1073741824 --jetstream >> /apps/.logs/nats-server.log 2>&1 & disown;

echo -e "Executing RA Web Server";
./ra-web-srv >> /apps/.logs/ra-web-srv.log 2>&1 & disown;

echo -e "Executing FS Monitor";
./fs-monitor --directory "./" --ignore-file "./.fsignore" --nats-url "nats://127.0.0.1:4222" --kv-bucket-name "fs_hashes" >> /apps/.logs/fs-monitor.log 2>&1 & disown;

# ifconfig -a;

echo -e "Init.sh executed";

tail -f /apps/.logs/pipeline.log

