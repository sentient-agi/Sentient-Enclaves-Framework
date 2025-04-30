FROM nixos/nix:latest AS build
ARG TARGET=all
ENV TARGET=${TARGET}

RUN mkdir -vp /build/
ADD ./ /build/
WORKDIR /build/

RUN nix-build --show-trace -A ${TARGET}

FROM scratch AS artifacts
COPY --from=build /build/result/* /init_blobs/
# Without a CMD we can not create a container from this to extract the content
CMD dummy
