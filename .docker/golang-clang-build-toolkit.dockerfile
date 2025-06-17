FROM golang:1 as builder

ENV SHELL="/usr/bin/env bash"

RUN apt update -y
RUN apt install -y git gcc

WORKDIR /app-builder

# COPY --link app.go/ /app-builder/app.go/

# CMD whoami; uname -a; date; pwd;
# CMD sleep infinity
CMD tail -f /dev/null
