##!/bin/bash
#!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

tar --same-owner --acls --xattrs --selinux -vpxf ./padding.tar -M -F ./volumes.sh -C ./
sha512sum --check ./random_blocks.sha512sum
