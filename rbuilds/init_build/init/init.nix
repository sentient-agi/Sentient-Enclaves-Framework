{
  pkgs ? import (fetchTarball {
    url = "https://github.com/NixOS/nixpkgs/archive/refs/tags/24.11.tar.gz";
    sha256 = "sha256:1gx0hihb7kcddv5h0k7dysp2xhf1ny0aalxhjbpj2lmvj7h9g80a";
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
