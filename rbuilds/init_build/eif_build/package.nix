#    Enclave image format builder
#    Builds an eif file
#
#    USAGE:
#        eif_build [OPTIONS] --kernel <FILE> --cmdline <String> --output <FILE> --ramdisk <FILE>
#
{
    pkgs ? import (fetchTarball {
        url = "https://github.com/NixOS/nixpkgs/archive/refs/tags/24.11.tar.gz";
        sha256 = "sha256:1gx0hihb7kcddv5h0k7dysp2xhf1ny0aalxhjbpj2lmvj7h9g80a";
    }) {}
, lib
, rustPlatform
, makeRustPlatform
, fetchFromGitHub
, fetchgit
, openssl
, pkg-config
, ...
}:
with pkgs;
let
    rust_overlay = import (builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz");
    pkgs = import <nixpkgs> { overlays = [ rust_overlay ]; };
    rustVersion = "latest";
    # rustVersion = "1.86.0";
    rustup = pkgs.rust-bin.nightly.${rustVersion}.minimal;
    rustPlatform = makeRustPlatform {
        cargo = rustup;
        rustc = rustup;
    };
in
rustPlatform.buildRustPackage {
    name = "eif_build";
    nativeBuildInputs = [ pkg-config ];
    buildInputs = [ openssl ];

    buildType = "release";

#    src = fetchgit {
#        url = "https://github.com/andrcmdr/aws-nitro-enclaves-image-format.git";
#        hash = "";
#    };

    src = fetchFromGitHub {
        owner = "andrcmdr";
        repo = "aws-nitro-enclaves-image-format";
        rev = "953d6ce";
        hash = "sha256-42vf12fz6djiodPjg6OhzbcbBOjAUSVRJ5q+/8NDGjk=";
    };

#    buildAndTestSubdir = "./";
    cargoBuildFlags = "--all";
    postPatch = ''
        # symlink our own cargo lock file into build because AWS' source does not include one
        ln -vrsf ${./Cargo.lock} Cargo.lock
    '';

    cargoLock.lockFile = ./Cargo.lock;
    cargoLock.outputHashes = {
        "aws-nitro-enclaves-cose-0.5.3" = "sha256-RqQt7jH5DwjnsziBWwJlzamZlqDoe7scCdRFoeWYq/U=";
    };

    installPhase = ''
        pwd
        echo -e "$out"
        mkdir -p $out/eif_build/
        cp -r ./target/x86_64-unknown-linux-gnu/release/eif_build $out/eif_build/eif_build
    '';
}
