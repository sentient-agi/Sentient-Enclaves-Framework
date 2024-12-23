#!/bin/bash
##!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

declare port=53000
declare cid=127

declare mode="${1}" # send-dir, recv-dir
declare source_dir="${2}" # source parent directory
declare destination_dir="${3}" # destination parent directory

send_dir () {

echo -E "${mode}";
echo -E "${source_dir}";
echo -E "${destination_dir}";
pwd
# run remote cmd
./pipeline run --port $port --cid $cid --command "uname -a"
# mkdir -vp "${destination_dir}"
./pipeline run --port $port --cid $cid --command "mkdir -vp ${destination_dir}"
find "${1}" -type d -printf '%P:' -exec echo '{}' \; |
while read path
do
    # run remote cmd
    echo -E "${path}";
    # mkdir -vp "${destination_dir}/`echo -E "${path}" | cut -d ":" -f1`"
    ./pipeline run --port $port --cid $cid --command "mkdir -vp ${destination_dir}/`echo -E "${path}" | cut -d ":" -f1`"
done

find "${1}" -type f -printf '%P:' -exec echo '{}' \; |
while read path
do
    # send-file
    echo -E "${path}";
    # cp -vr "`echo -E "${path}" | cut -d ":" -f2`" "${destination_dir}/`echo -E "${path}" | cut -d ":" -f1`"
    ./pipeline send-file --port $port --cid $cid --localpath "`echo -E "${path}" | cut -d ":" -f2`" --remotepath "${destination_dir}/`echo -E "${path}" | cut -d ":" -f1`"
done

}

recv_dir () {

echo -E "${mode}";
echo -E "${source_dir}";
echo -E "${destination_dir}";
pwd
# run remote cmd
./pipeline run --port $port --cid $cid --command "uname -a"
mkdir -vp "${destination_dir}"
# find "${1}" -type d -printf '%P:' -exec echo '{}' \; |
./pipeline run --port $port --cid $cid --command "find ${1} -type d -printf '%P:' -exec echo '{}' \;" |
while read path
do
    echo -E "${path}";
    mkdir -vp "${destination_dir}/`echo -E "${path}" | cut -d ":" -f1`"
done

# run remote cmd
# find "${1}" -type f -printf '%P:' -exec echo '{}' \; |
./pipeline run --port $port --cid $cid --command "find ${1} -type f -printf '%P:' -exec echo '{}' \;" |
while read path
do
    # recv-file
    echo -E "${path}";
    # cp -vr "`echo -E "${path}" | cut -d ":" -f2`" "${destination_dir}/`echo -E "${path}" | cut -d ":" -f1`"
    ./pipeline recv-file --port $port --cid $cid --localpath "${destination_dir}/`echo -E "${path}" | cut -d ":" -f1`" --remotepath "`echo -E "${path}" | cut -d ":" -f2`"
done

}

if [[ "${1}" == "-h" || "${1}" == "--help" || "${1}" == "h" || "${1}" == "help" ]]; then

    echo -e "usage: pipeline-dir [-h | --help | h | help | mode: send-dir | recv-dir] [source parent dir] [destination parent dir]"

elif [[ "${#@}" -eq 0 ]]; then

    echo -e "usage: pipeline-dir [-h | --help | h | help | mode: send-dir | recv-dir] [source parent dir] [destination parent dir]"

elif [[ "${1}" == "send-dir" ]]; then

    send_dir "${2}" "${3}"

elif [[ "${1}" == "recv-dir" ]]; then

    recv_dir "${2}" "${3}"

else

    echo -e "usage: pipeline-dir [-h | --help | h | help | mode: send-dir | recv-dir] [source parent dir] [destination parent dir]"

fi
