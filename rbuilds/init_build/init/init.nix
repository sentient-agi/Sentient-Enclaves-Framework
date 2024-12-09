{
  pkgs ? import (fetchTarball {
    url = "https://github.com/NixOS/nixpkgs/archive/refs/tags/24.05.tar.gz";
    sha256 = "sha256:1lr1h35prqkd1mkmzriwlpvxcb34kmhc9dnr48gkm8hh089hifmx";
  }) {}
}:
pkgs.stdenv.mkDerivation rec {
  name = "nitro-enclaves-init";

  nativeBuildInputs = with pkgs; [
    gcc
    glibc.static
  ];

  src = ./.;

  buildPhase = ''
    gcc -Wall -Wextra -Werror -O2 -o init init.c -static -static-libgcc -flto
    strip --strip-all init
  '';

  installPhase = ''
    mkdir -p $out
    cp init $out/
  '';
}
