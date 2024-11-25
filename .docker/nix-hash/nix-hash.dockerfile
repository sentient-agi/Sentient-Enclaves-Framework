FROM nixos/nix:2.21.4 as build
ARG VER="linux-rolling-stable"
ENV VER=${VER}

RUN mkdir /build
WORKDIR /build

RUN git clone https://github.com/msteen/nix-prefetch.git
RUN nix-env --install --file ./nix-prefetch/release.nix
RUN nix-env -i gawk

COPY --link nix-hash.sh nix-hash.sh

CMD bash ./nix-hash.sh "${VER}"
