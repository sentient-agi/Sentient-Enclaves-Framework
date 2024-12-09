#  Enclave image format builder
#  Builds an eif file
#
#  USAGE:
#      eif_build [OPTIONS] --kernel <FILE> --cmdline <String> --output <FILE> --ramdisk <FILE>
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
  name = "eif_build";
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ openssl ];
#  src = pkgs.fetchgit {
#    url = "https://github.com/aws/aws-nitro-enclaves-image-format.git";
#    hash = "sha256-tJ5/GS5rhh3xTM+ZGuSrFnoLZF/2h22imSTfisq87eU=";
#  };
  src = fetchFromGitHub {
    owner = "aws";
    repo = "aws-nitro-enclaves-image-format";
    rev = "b26bf69";
    hash = "sha256-tJ5/GS5rhh3xTM+ZGuSrFnoLZF/2h22imSTfisq87eU=";
#    rev = "v0.3.0";
#    hash = "sha256-vtMmyAcNUWzZqS1NQISMdq1JZ9nxOmqSNahnbRhFmpQ=";
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
    mkdir -p $out/eif_build/
    cp -r ./target/x86_64-unknown-linux-gnu/release/eif_build $out/eif_build/eif_build
  '';
}
