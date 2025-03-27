#  Enclave image format builder
#  Builds an eif file
#
#  USAGE:
#      eif_build [OPTIONS] --kernel <FILE> --cmdline <String> --output <FILE> --ramdisk <FILE>
#
#  Enclave image format extractor
#  Extracts sections from an eif file
#
#  USAGE:
#      eif_extract <EIF_PATH> <OUTPUT_DIR> <PREFIX>
#
{
  pkgs ? import (fetchTarball {
    url = "https://github.com/NixOS/nixpkgs/archive/refs/tags/24.05.tar.gz";
    sha256 = "sha256:1lr1h35prqkd1mkmzriwlpvxcb34kmhc9dnr48gkm8hh089hifmx";
  }) {}
, lib
, rustPlatform
, fetchFromGitHub
, fetchgit
, openssl
, pkg-config
, ...
}:
pkgs.rustPlatform.buildRustPackage {
  name = "eif_extract";
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ openssl ];
#  src = pkgs.fetchgit {
#    url = "https://github.com/andrcmdr/aws-nitro-enclaves-image-format-build-extract.git";
#    hash = "";
#  };
  src = fetchFromGitHub {
    owner = "andrcmdr";
    repo = "aws-nitro-enclaves-image-format-build-extract";
    rev = "ca810fb";
    hash = "";
  };
#  buildAndTestSubdir = "./";
  cargoBuildFlags = "--release --all";
  postPatch = ''
    # symlink our own cargo lock file into build because AWS' source does not include one
    ln -s ${./Cargo.lock} Cargo.lock
  '';
  cargoLock.lockFile = ./Cargo.lock;

  installPhase = ''
    pwd
    echo -e "$out"
    mkdir -p $out/eif_extract/
    cp -r ./target/x86_64-unknown-linux-gnu/release/eif_extract $out/eif_extract/eif_extract
    cp -r ./target/x86_64-unknown-linux-gnu/release/eif_build $out/eif_extract/eif_build
  '';
}
