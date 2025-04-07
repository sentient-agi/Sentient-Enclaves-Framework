{
  nixpkgs ? import (fetchTarball {
    url = "https://github.com/NixOS/nixpkgs/archive/refs/tags/24.11.tar.gz";
    sha256 = "sha256:1gx0hihb7kcddv5h0k7dysp2xhf1ny0aalxhjbpj2lmvj7h9g80a";
  }) {}
}:
let
#  nixpkgs = import (fetchTarball {
#    url = "https://github.com/NixOS/nixpkgs/archive/refs/tags/24.05.tar.gz";
#    sha256 = "sha256:1lr1h35prqkd1mkmzriwlpvxcb34kmhc9dnr48gkm8hh089hifmx";
#  }) {};

  arch = nixpkgs.stdenv.hostPlatform.uname.processor;
in
rec {
  init = nixpkgs.callPackage ./init/init.nix { };

  init_go = nixpkgs.callPackage ./init_go/init_go.nix { };

  eif_build = nixpkgs.callPackage ./eif_build/package.nix { };

  eif_extract = nixpkgs.callPackage ./eif_extract/package.nix { };

  all = nixpkgs.runCommandNoCC "enclave-blobs-${arch}" { } ''
    echo -e "$out"
    echo -e "${arch}"
    sleep 3;

    mkdir -p $out/${arch}/

    mkdir -p $out/${arch}/init/
    cp -r ${init}/* $out/${arch}/init/

    mkdir -p $out/${arch}/init_go/
    cp -r ${init_go}/bin/* $out/${arch}/init_go/

    mkdir -p $out/${arch}/eif_build/
    cp -r ${eif_build}/eif_build/* $out/${arch}/eif_build/

    mkdir -p $out/${arch}/eif_extract/
    cp -r ${eif_extract}/eif_extract/* $out/${arch}/eif_extract/
  '';
}
