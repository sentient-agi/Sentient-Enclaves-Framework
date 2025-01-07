#!/bin/bash
##!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

declare port=${1:-53000}
declare cid=${2:-127}
declare no_wait=1

# HuggingFace access token: "hf_*"
declare hf_token=${3:-$HF_TOKEN}

if [[ "$1" == "?" || "$1" == "-?" || "$1" == "h" || "$1" == "-h" || "$1" == "help" || "$1" == "--help" ]]; then
    echo -e "\nShell script to deploy and run networking components (proxies and apps) in enclave, and to support networking capabilities inside the enclave.\n"
    echo -e "Input 'make' command to start testing and benchmarking of networking capabilities inside enclave, using various networking tools (aws-cli, s5cmd, rclone, huggingface-cli and hf-transfer) for higher level networking protocols (http/https, ssh).\n"
    echo -e "Also includes shell to communicate with enclave through Pipeline SLC protocol, to run commands and transfer files.\n"
    echo -e "Input 'nw' (stands for 'no-wait') command to enable local or remote console output.\n"
    echo -e "Type 'tty' to print the filename of the terminal connected/attached to the standard input (to this shell).\n"
    echo -e "Enter 'break' or 'exit' for exit from this shell.\n"
    exit 0
fi

runcmd() {
    ./pipeline run --port $port --cid $cid --command "${@}"
}

runcmd_no_wait() {
    ./pipeline run --port $port --cid $cid --no-wait --command "${@}"
}

deploy_net_components() {
    ./pipeline-dir send-dir ./pf_build/ /app/pf_build/ 2>&1 ;
    ./pipeline run --port $port --cid $cid --command "chmod -v +x -R ./pf_build/*" ;

    ./pipeline send-file --port $port --cid $cid --localpath ./pf-guest.sh --remotepath ./pf-guest.sh ;
    ./pipeline run --port $port --cid $cid --command "chmod -v +x ./pf-guest.sh" ;

    ./pipeline send-file --port $port --cid $cid --localpath ./pf-tp-guest.sh --remotepath ./pf-tp-guest.sh ;
    ./pipeline run --port $port --cid $cid --command "chmod -v +x ./pf-tp-guest.sh" ;

    ./pipeline send-file --port $port --cid $cid --localpath ./speedtest --remotepath ./speedtest ;
    ./pipeline run --port $port --cid $cid --command "chmod -v +x ./speedtest" ;

    ./pipeline send-file --port $port --cid $cid --localpath ./rclone --remotepath ./rclone ;
    ./pipeline run --port $port --cid $cid --command "chmod -v +x ./rclone" ;

    ./pipeline send-file --port $port --cid $cid --localpath ./s5cmd --remotepath ./s5cmd ;
    ./pipeline run --port $port --cid $cid --command "chmod -v +x ./s5cmd" ;

    ./pipeline-dir send-dir ./root/ /app/ 2>&1 ;

    ./pipeline send-file --port $port --cid $cid --localpath ./download.list --remotepath ./download.list ;
    ./pipeline send-file --port $port --cid $cid --localpath ./wget.sh --remotepath ./wget.sh ;
    ./pipeline run --port $port --cid $cid --command "chmod -v +x ./wget.sh" ;
}

run_net_components() {
    nohup ./pf-host.sh &> /dev/null & disown ; wait
    nohup ./pf-tp-host.sh &> /dev/null & disown ; wait

    ./pipeline run --port $port --cid $cid --no-wait --command "./pf-guest.sh" ;
    ./pipeline run --port $port --cid $cid --no-wait --command "./pf-tp-guest.sh" ;
    ./pipeline run --port $port --cid $cid --no-wait --command "ifconfig -a" ;
}

install_pkgs() {
    ./pipeline run --port $port --cid $cid --no-wait --command "dnf install -y pip" ; wait
}

install_hf() {
    ./pipeline run --port $port --cid $cid --no-wait --command "pip3 install -U "huggingface_hub[cli]"" ;
    ./pipeline run --port $port --cid $cid --no-wait --command "pip3 install -U "huggingface_hub[hf_transfer]"" ;
    ./pipeline run --port $port --cid $cid --no-wait --command "pip3 install -U hf_transfer" ;
}

dl_models_hf() {
    ./pipeline run --port $port --cid $cid --no-wait --command "/usr/bin/time -v -o ./hf_transfer.log huggingface-cli download mistralai/Mistral-7B-v0.3 --local-dir ./models/Mistral-7B-v0.3 --token=${hf_token}" ;
    ls -lah ./models/Mistral-7B-v0.3
    ./pipeline run --port $port --cid $cid --no-wait --command "/usr/bin/time -v -o ./hf_transfer.log huggingface-cli download meta-llama/Llama-3.1-8B --local-dir ./models/Llama-3.1-8B --token=${hf_token}" ;
    ls -lah ./models/Llama-3.1-8B
}

ul_models_s5() {
    ./pipeline run --port $port --cid $cid --no-wait --command "./s5cmd --retry-count 100 --numworkers 256 --stat --log "debug" ls --humanize" ;
    ./pipeline run --port $port --cid $cid --no-wait --command "./s5cmd --retry-count 100 --numworkers 256 --stat --log "debug" ls --humanize "s3://tee-team-fine-tuned-models"" ;

    ./pipeline run --port $port --cid $cid --no-wait --command "/usr/bin/time -v -o ./s5cmd.log ./s5cmd --retry-count 100 --numworkers 256 --stat --log "debug" cp --show-progress --concurrency 16 --acl "bucket-owner-full-control" /app/models/Mistral-7B-v0.3/ "s3://tee-team-fine-tuned-models/Mistral-7B-v0.3/"" ;
    ./pipeline run --port $port --cid $cid --no-wait --command "./s5cmd --retry-count 100 --numworkers 256 --stat --log "debug" ls --humanize "s3://tee-team-fine-tuned-models/Mistral-7B-v0.3/"" ;

    ./pipeline run --port $port --cid $cid --no-wait --command "/usr/bin/time -v -o ./s5cmd.log ./s5cmd --retry-count 100 --numworkers 256 --stat --log "debug" cp --show-progress --concurrency 16 --acl "bucket-owner-full-control" /app/models/Llama-3.1-8B/ "s3://tee-team-fine-tuned-models/Llama-3.1-8B/"" ;
    ./pipeline run --port $port --cid $cid --no-wait --command "./s5cmd --retry-count 100 --numworkers 256 --stat --log "debug" ls --humanize "s3://tee-team-fine-tuned-models/Llama-3.1-8B/"" ;
}

ul_models_s3() {
    ./pipeline run --port $port --cid $cid --no-wait --command "aws s3 ls" ;
    ./pipeline run --port $port --cid $cid --no-wait --command "aws s3 ls s3://tee-team-fine-tuned-models" ;
    ./pipeline run --port $port --cid $cid --no-wait --command "/usr/bin/time -v -o ./aws_cli.log aws s3 cp /app/models/Mistral-7B-v0.3/ s3://tee-team-fine-tuned-models/Mistral-7B-v0.3/ --recursive" ;
    ./pipeline run --port $port --cid $cid --no-wait --command "/usr/bin/time -v -o ./aws_cli.log aws s3 cp /app/models/Llama-3.1-8B/ s3://tee-team-fine-tuned-models/Llama-3.1-8B/ --recursive" ;
    ./pipeline run --port $port --cid $cid --no-wait --command "aws s3 ls s3://tee-team-fine-tuned-models/Mistral-7B-v0.3/" ;
    ./pipeline run --port $port --cid $cid --no-wait --command "aws s3 ls s3://tee-team-fine-tuned-models/Llama-3.1-8B/" ;
}

ul_models_rclone() {
    ./pipeline run --port $port --cid $cid --no-wait --command "./rclone lsd aws_s3_buckets:" ;
    ./pipeline run --port $port --cid $cid --no-wait --command "./rclone lsl aws_s3_buckets:/tee-team-fine-tuned-models/" ;
    ./pipeline run --port $port --cid $cid --no-wait --command "/usr/bin/time -v -o ./rclone.log ./rclone copy --verbose --progress --create-empty-src-dirs --no-traverse /app/models/Mistral-7B-v0.3/ aws_s3_buckets:/tee-team-fine-tuned-models/Mistral-7B-v0.3/" ;
    ./pipeline run --port $port --cid $cid --no-wait --command "/usr/bin/time -v -o ./rclone.log ./rclone copy --verbose --progress --create-empty-src-dirs --no-traverse /app/models/Llama-3.1-8B/ aws_s3_buckets:/tee-team-fine-tuned-models/Llama-3.1-8B/" ;
    ./pipeline run --port $port --cid $cid --no-wait --command "./rclone lsl aws_s3_buckets:/tee-team-fine-tuned-models/Mistral-7B-v0.3/" ;
    ./pipeline run --port $port --cid $cid --no-wait --command "./rclone lsl aws_s3_buckets:/tee-team-fine-tuned-models/Llama-3.1-8B/" ;
}

while true; do
    read -p "$(runcmd whoami | tr -d '\n')@$(runcmd 'hostname -s' | tr -d '\n'):${cid}:${port}:$(runcmd pwd | tr -d '\n') $( [[ "$(runcmd whoami | tr -d '\n')" == "root" ]] && echo -e "#" || echo -e "\$" )> " cmd

    if [[ $cmd == "break" || $cmd == "exit" ]]; then
        break
    fi

    if [[ $cmd == "tty" ]]; then
        tty ;
        continue
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

    if [[ $cmd == "deploy_net_components" ]]; then
        deploy_net_components ;
        continue
    fi

    if [[ $cmd == "run_net_components" ]]; then
        run_net_components ;
        continue
    fi

    if [[ $cmd == "install_pkgs" ]]; then
        install_pkgs ;
        continue
    fi

    if [[ $cmd == "install_hf" ]]; then
        install_hf ;
        continue
    fi

    if [[ $cmd == "dl_models_hf" ]]; then
        dl_models_hf ;
        continue
    fi

    if [[ $cmd == "ul_models_s5" ]]; then
        ul_models_s5 ;
        continue
    fi

    if [[ $cmd == "ul_models_s3" ]]; then
        ul_models_s3 ;
        continue
    fi

    if [[ $cmd == "ul_models_rclone" ]]; then
        ul_models_rclone ;
        continue
    fi

    if [[ $cmd == "make" ]]; then
        read -n 1 -s -p "Deploy networking components? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            deploy_net_components > /dev/null 2>&1 ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Run networking components? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            run_net_components > /dev/null 2>&1 ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Installing essential RPM packages via DNF, continue? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            install_pkgs > /dev/null 2>&1 ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Install HuggingFace CLI toolkit? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            install_hf > /dev/null 2>&1 ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Download models using HuggingFace CLI toolkit? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            dl_models_hf > /dev/null 2>&1 ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Upload models to S3 bucket using S5cmd tool? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            ul_models_s5 > /dev/null 2>&1 ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Upload models to S3 bucket using AWS CLI toolkit? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            ul_models_s3 > /dev/null 2>&1 ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Upload models to S3 bucket using RClone tool? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            ul_models_rclone > /dev/null 2>&1 ; wait
        else
            echo -e "\n"
        fi

        continue
    fi

done

