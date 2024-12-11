#! /bin/bash
##! /usr/bin/env bash

# Script for creating, extracting, testing & listing, compaing & finding differences for multi-volume TAR archives

# For this script it's advisable to use a shell, such as Bash,
# that supports a TAR_FD value greater than 9.

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

name=`expr $TAR_ARCHIVE : '\(.*\)-.*'`

case $TAR_SUBCOMMAND in
-c)       echo -e "Created volume ${TAR_ARCHIVE}";
          echo -e "Handling next volume ${TAR_VOLUME} => FD:${TAR_FD}";
          echo -E "${name:-$TAR_ARCHIVE}-${TAR_VOLUME}" >&${TAR_FD};
          ;;
-x)       echo -e "Extracted volume ${TAR_ARCHIVE}";
          echo -e "Handling next volume ${TAR_VOLUME} => FD:${TAR_FD}";
          test -e ${name:-$TAR_ARCHIVE}-$TAR_VOLUME || exit 1
          test -s ${name:-$TAR_ARCHIVE}-$TAR_VOLUME || exit 3
          test -r ${name:-$TAR_ARCHIVE}-$TAR_VOLUME || exit 5
          echo -E "${name:-$TAR_ARCHIVE}-${TAR_VOLUME}" >&${TAR_FD};
          ;;
-t)       echo -e "Listing of volume ${TAR_ARCHIVE}";
          echo -e "Handling next volume ${TAR_VOLUME} => FD:${TAR_FD}";
          test -e ${name:-$TAR_ARCHIVE}-$TAR_VOLUME || exit 1
          test -s ${name:-$TAR_ARCHIVE}-$TAR_VOLUME || exit 3
          test -r ${name:-$TAR_ARCHIVE}-$TAR_VOLUME || exit 5
          echo -E "${name:-$TAR_ARCHIVE}-${TAR_VOLUME}" >&${TAR_FD};
          ;;
-d)       echo -e "Finding differences: comparing with volume ${TAR_ARCHIVE}";
          echo -e "Handling next volume ${TAR_VOLUME} => FD:${TAR_FD}";
          test -e ${name:-$TAR_ARCHIVE}-$TAR_VOLUME || exit 1
          test -s ${name:-$TAR_ARCHIVE}-$TAR_VOLUME || exit 3
          test -r ${name:-$TAR_ARCHIVE}-$TAR_VOLUME || exit 5
          echo -E "${name:-$TAR_ARCHIVE}-${TAR_VOLUME}" >&${TAR_FD};
          ;;
*)        echo -e "Not supported TAR subcommand for the shell script: ${TAR_SUBCOMMAND}";
          exit 7
esac
