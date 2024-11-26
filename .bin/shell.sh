#!/bin/bash
##!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

declare port=${1:-53000}
declare cid=${2:-127}
declare no_wait=1

if [[ "$1" == "?" || "$1" == "-?" || "$1" == "h" || "$1" == "-h" || "$1" == "help" || "$1" == "--help" ]]; then
    echo -e "Simple shell to communicate with enclave through Pipeline."
    echo -e "Input 'nw' command to enable local or remote console output."
    echo -e "Enter 'break' or 'exit' for exit from this shell."
    exit 0
fi

runcmd() {
    ./pipeline run --port $port --cid $cid --command "${@}"
}

runcmd_no_wait() {
    ./pipeline run --port $port --cid $cid --no-wait --command "${@}"
}

while true; do
    read -p "$(runcmd whoami | tr -d '\n')@$(runcmd 'hostname -s' | tr -d '\n'):${cid}:${port}:$(runcmd pwd | tr -d '\n') $( [[ "$(runcmd whoami | tr -d '\n')" == "root" ]] && echo -e "#" || echo -e "\$" )> " cmd

    if [[ $cmd == "break" || $cmd == "exit" ]]; then
        break
    fi

    if [[ $cmd == "nw" ]]; then
        # no_wait=$(( ! $no_wait ))
        no_wait=$(( 1 - $no_wait ))
        echo "no_wait == $no_wait"
        continue
    fi

    if [[ $no_wait -eq 1 ]]; then
        runcmd_no_wait "$cmd"
    else
        runcmd "$cmd"
    fi
done

