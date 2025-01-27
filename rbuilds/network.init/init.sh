##!/bin/bash
#!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

echo -e "Init.sh started";

# mkdir -vp /apps/.logs/;

echo -e "Executing Pipeline";
cd /apps/;
./pipeline listen --port 53000 >> /apps/.logs/pipeline.log 2>&1 & disown;
echo -e "Executing PF-TP-Proxy";
./pf-tp-guest.sh 2>&1 & disown;
echo -e "Executing Socat";
./pf-guest.sh 2>&1 & disown;

# ifconfig -a;

echo -e "Init.sh executed";

