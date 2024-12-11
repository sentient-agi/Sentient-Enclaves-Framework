##!/bin/bash
#!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

sudo /usr/bin/time -v -o ./runtime.log dd status=progress if=/dev/random of=./random_blocks bs=1G count=150 iflag=fullblock,nonblock oflag=sync,nonblock ; sync
sha512sum -b --tag ./random_blocks > ./random_blocks.sha512sum
tar --same-owner --acls --xattrs --selinux -vpcf ./padding.tar -M -L 4095M -F ./volumes.sh ./random_blocks
