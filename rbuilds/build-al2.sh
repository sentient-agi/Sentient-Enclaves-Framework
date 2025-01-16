#!/bin/bash
##!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

declare kversion='6.12' # Linux kernel version
declare kbuild_user="sentient_build" # Username for kernel build
declare kbuild_host="sentient_builder" #Hostname for kernel build
declare enclave_cpus='64' # Number of CPUs allocated for Nitro Enclaves runt-time
declare enclave_mem='838656' # MiBs of memory allocated for Nitro Enclaves runt-time
declare enclave_cid='127' # Enclave's VSock CID for data connect

if [[ "$1" == "?" || "$1" == "-?" || "$1" == "h" || "$1" == "-h" || "$1" == "help" || "$1" == "--help" ]]; then
    echo -e "\nShell script to build custom kernel, Rust apps (SSE Framework) for eclave's run-time, init system for enclave, and to build enclave images (EIF) reproducibly.\n"
    echo -e "Type 'help' to print help and 'help_ext' to print extended help.\n"
    echo -e "\n"
    echo -e "Input 'make kernel' command to start building custom Linux kernel.\n"
    echo -e "Input 'make apps' command to start building Rust apps (SSE Framework) for enclave's run-time and to build enclave's image creation and extraction tools.\n"
    echo -e "Input 'make init' command to start building init system for enclave.\n"
    echo -e "Input 'make' command to start building enclave image (EIF).\n"
    echo -e "Input 'make enclave' command to manage encalves run-time: run enclave, attach debug console to enclave, list running enclaves and terminate one or all enclaves.\n"
    echo -e "\n"
    echo -e "Type 'tty' to print the filename of the terminal connected/attached to the standard input (to this shell).\n"
    echo -e "Enter 'break' or 'exit', or push 'Ctrl+C' key sequence, for exit from this shell.\n"
    exit 0
fi

if [[ "$1" == "??" || "$1" == "-??" || "$1" == "he" || "$1" == "-he" || "$1" == "helpext" || "$1" == "help-ext" || "$1" == "--help-ext" ]]; then

    echo -e "\nCommands for manual stages execution:

        Print help and print extended help commands:

        help
        help_ext

        Build custom kernel stages:

        docker_kcontainer_clear
        docker_kimage_clear
        docker_kimage_build
        docker_prepare_kbuildenv
        docker_kernel_build

        Build Rust apps (SSE Framework) for enclave's run-time and enclave image build tools, stages are:

        docker_apps_rs_container_clear
        docker_apps_rs_image_clear
        docker_apps_rs_image_build
        docker_prepare_apps_rs_buildenv
        docker_apps_rs_build

        Build custom init system for enclave, stages are:

        docker_init_clear
        docker_init_build

        Build enclave image file (EIF) stages:

        docker_clear
        docker_build
        init_and_rootfs_base_images_build
        docker_to_rootfs_fs_image_build
        ramdisk_image_build
        eif_build_with_initc
        eif_build_with_initgo

        Run enclave image file (EIF), connect/attach local terminal to enclave's console output, list running enclaves, terminate enclaves:

        run_eif_image_debugmode_cli
        run_eif_image_debugmode
        run_eif_image
        attach_console_to_recent_enclave
        attach_console_to_enclave
        list_enclaves
        drop_recent_enclave
        drop_enclave
        drop_enclaves_all

    "
    exit 0
fi

# Print help commands

help() {
    echo -e "\nShell script to build custom kernel, Rust apps (SSE Framework) for eclave's run-time, init system for enclave, and to build enclave images (EIF) reproducibly.\n"
    echo -e "Type 'help' to print help and 'help_ext' to print extended help.\n"
    echo -e "\n"
    echo -e "Input 'make kernel' command to start building custom Linux kernel.\n"
    echo -e "Input 'make apps' command to start building Rust apps (SSE Framework) for enclave's run-time and to build enclave's image creation and extraction tools.\n"
    echo -e "Input 'make init' command to start building init system for enclave.\n"
    echo -e "Input 'make' command to start building enclave image (EIF).\n"
    echo -e "Input 'make enclave' command to manage encalves run-time: run enclave, attach debug console to enclave, list running enclaves and terminate one or all enclaves.\n"
    echo -e "\n"
    echo -e "Type 'tty' to print the filename of the terminal connected/attached to the standard input (to this shell).\n"
    echo -e "Enter 'break' or 'exit', or push 'Ctrl+C' key sequence, for exit from this shell.\n"
}

help_ext() {
    echo -e "\nCommands for manual stages execution:

        Print help and print extended help commands:

        help
        help_ext

        Build custom kernel stages:

        docker_kcontainer_clear
        docker_kimage_clear
        docker_kimage_build
        docker_prepare_kbuildenv
        docker_kernel_build

        Build Rust apps (SSE Framework) for enclave's run-time and enclave image build tools, stages are:

        docker_apps_rs_container_clear
        docker_apps_rs_image_clear
        docker_apps_rs_image_build
        docker_prepare_apps_rs_buildenv
        docker_apps_rs_build

        Build custom init system for enclave, stages are:

        docker_init_clear
        docker_init_build

        Build enclave image file (EIF) stages:

        docker_clear
        docker_build
        init_and_rootfs_base_images_build
        docker_to_rootfs_fs_image_build
        ramdisk_image_build
        eif_build_with_initc
        eif_build_with_initgo

        Run enclave image file (EIF), connect/attach local terminal to enclave's console output, list running enclaves, terminate enclaves:

        run_eif_image_debugmode_cli
        run_eif_image_debugmode
        run_eif_image
        attach_console_to_recent_enclave
        attach_console_to_enclave
        list_enclaves
        drop_recent_enclave
        drop_enclave
        drop_enclaves_all

    "
}

# Building custom Linux kernel:

docker_kcontainer_clear() {
    docker kill kernel_build_v${kversion} ;
    docker rm --force kernel_build_v${kversion} ;
}

docker_kimage_clear() {
    # whoami; uname -a; pwd;
    docker rmi --force kernel-build-toolkit-al2:v${kversion} ;
}

docker_kimage_build() {
    DOCKER_BUILDKIT=1 docker build --no-cache -f ./rust-build-toolkit-al2.dockerfile -t "kernel-build-toolkit-al2:v${kversion}" ./ ;
    # -ti
    docker create --name kernel_build_v${kversion} kernel-build-toolkit-al2:v${kversion} sleep infinity; sleep 1;
    # docker create --name kernel_build_v${kversion} kernel-build-toolkit-al2:v${kversion} tail -f /dev/null; sleep 1;
    # -tid
    # docker run -d --name kernel_build_v${kversion} kernel-build-toolkit-al2:v${kversion} sleep infinity & disown; sleep 1;
    # docker run -d --name kernel_build_v${kversion} kernel-build-toolkit-al2:v${kversion} tail -f /dev/null & disown; sleep 1;
    # docker stop kernel_build_v${kversion} ;
    docker kill kernel_build_v${kversion} ;
    docker start kernel_build_v${kversion} ;
}

docker_prepare_kbuildenv() {
    docker exec -ti kernel_build_v${kversion} bash -cis -- 'whoami; uname -a; pwd;' ;
    docker exec -ti kernel_build_v${kversion} bash -cis -- 'dnf install -y time which hostname git patch make gcc flex bison \
        elfutils elfutils-devel elfutils-libelf elfutils-libelf-devel elfutils-libs \
        kmod openssl openssl-devel openssl-libs bc perl gawk wget cpio tar bsdtar xz bzip2 gzip xmlto \
        ncurses ncurses-devel diffutils rsync' ;
    docker exec -ti kernel_build_v${kversion} bash -cis -- 'dnf install -y --allowerasing curl' ;
    docker exec -ti kernel_build_v${kversion} bash -cis -- "mkdir -vp /kbuilder; cd /kbuilder; wget https://github.com/gregkh/linux/archive/v${kversion}.tar.gz" ;
    docker exec -ti kernel_build_v${kversion} bash -cis -- "cd /kbuilder; tar --same-owner --acls --xattrs --selinux -vpxf v${kversion}.tar.gz -C ./" ;
    docker exec -ti kernel_build_v${kversion} bash -cis -- "cd /kbuilder; mv -v ./linux-${kversion} ./linux-v${kversion}" ;
    # docker cp ./kernel_config/artifacts_static/.config kernel_build_v${kversion}:/kbuilder/ ;
    docker cp ./kernel_config/artifacts_kmods/.config kernel_build_v${kversion}:/kbuilder/ ;
    docker exec -ti kernel_build_v${kversion} bash -cis -- "cp -vr /kbuilder/.config /kbuilder/linux-v${kversion}/.config" ;
}

docker_kernel_build() {
    docker exec -ti kernel_build_v${kversion} bash -cis -- "cd /kbuilder/linux-v${kversion}/; \
        mkdir -vp ./kernel_blobs; \
        mkdir -vp ./kernel_modules; \
        mkdir -vp ./kernel_headers; \
        export KBUILD_BUILD_TIMESTAMP="$(date -u '+%FT%T.%N%:z')"; \
        export KBUILD_BUILD_USER="${kbuild_user}"; \
        export KBUILD_BUILD_HOST="${kbuild_host}"; \
        export INSTALL_MOD_PATH="/kbuilder/linux-v${kversion}/kernel_modules/"; \
        export INSTALL_HDR_PATH="/kbuilder/linux-v${kversion}/kernel_headers/"; \
        export KBUILD_EXTRA_SYMBOLS="/kbuilder/linux-v${kversion}/Module.symvers"; \
        printenv; \
        make olddefconfig bzImage vmlinux modules -j16; \
        make olddefconfig modules -j16; \
        make olddefconfig modules_prepare; \
        make olddefconfig modules_install; \
        make headers_install; \
        make olddefconfig headers_install; \
    " ;
    docker exec -ti kernel_build_v${kversion} bash -cis -- "cd /kbuilder; \
        mkdir -vp ./artifacts_static/; \
        cp -vr /kbuilder/linux-v${kversion}/.config ./artifacts_static/; \
        cp -vr /kbuilder/linux-v${kversion}/drivers/misc/nsm.ko ./artifacts_static/; \
        cp -vr /kbuilder/linux-v${kversion}/kernel_modules/lib/modules/${kversion}/kernel/drivers/misc/nsm.ko ./artifacts_static/; \
        cp -vr /kbuilder/linux-v${kversion}/kernel_modules/ ./artifacts_static/; \
        mkdir -vp ./artifacts_static/kernel_headers/arch/x86/; \
        cp -vr /kbuilder/linux-v${kversion}/arch/x86/include/ ./artifacts_static/kernel_headers/arch/x86/; \
        mkdir -vp ./artifacts_static/kernel_headers/; \
        cp -vr /kbuilder/linux-v${kversion}/include/ ./artifacts_static/kernel_headers/; \
        mkdir -vp ./artifacts_static/kernel_headers/usr/; \
        cp -vr /kbuilder/linux-v${kversion}/usr/dummy-include/ ./artifacts_static/kernel_headers/usr/; \
        mkdir -vp ./artifacts_static/kernel_headers/usr/; \
        cp -vr /kbuilder/linux-v${kversion}/usr/include/ ./artifacts_static/kernel_headers/usr/; \
        cp -vr /kbuilder/linux-v${kversion}/arch/x86/boot/bzImage ./artifacts_static/; \
        cp -vr /kbuilder/linux-v${kversion}/arch/x86/boot/compressed/vmlinux ./artifacts_static/; \
        cp -vr /kbuilder/linux-v${kversion}/arch/x86/boot/compressed/vmlinux.bin ./artifacts_static/; \
        cp -vr /kbuilder/linux-v${kversion}/arch/x86/boot/compressed/vmlinux.bin.gz ./artifacts_static/; \
    " ;
    docker cp kernel_build_v${kversion}:/kbuilder/artifacts_static/ ./kernel_blobs/ ;
    # docker stop kernel_build_v${kversion} ;
    docker kill kernel_build_v${kversion} ;
    mkdir -vp ./blobs/ ;
    cp -vr ./kernel_blobs/artifacts_static/bzImage ./blobs/bzImage ;
    cp -vr ./kernel_blobs/artifacts_static/.config ./blobs/bzImage.config ;
    cp -vr ./kernel_blobs/artifacts_static/nsm.ko ./blobs/nsm.ko ;
    echo "reboot=k panic=30 pci=on nomodules console=ttyS0 i8042.noaux i8042.nomux i8042.nopnp i8042.dumbkbd random.trust_cpu=on" > ./blobs/cmdline ;
    mkdir -vp ./init/ ./init_go/ ;
    cp -vr ./kernel_blobs/artifacts_static/nsm.ko ./init/nsm.ko ;
    cp -vr ./kernel_blobs/artifacts_static/nsm.ko ./init_go/nsm.ko ;
    mkdir -vp ./rootfs_kmods/rootfs/usr/ ;
    cp -vr ./kernel_blobs/artifacts_static/kernel_modules/lib/ ./rootfs_kmods/rootfs/usr/ ;
}

# Building of enclave's image building/extraction tools and enclave's run-time Rust apps (Sentient Secure Enclaves Framework):
# Pipeline (SLC protocol),
# EIF_build & EIF_extract,
# PF-Proxies,
# SLC & content encryption (+ encryption/decryption protocol test tools, + multi-hop PRE re-encryption protocol test tools, + KMS test tools),
# Web-RA (+ NSM & TPM test tools, + KMS test tools),
# FS-Monitor (inotify) for RA DB,
# Possibly Nitro-CLI mod.

docker_apps_rs_container_clear() {
    docker kill apps_rs_build ;
    docker rm --force apps_rs_build ;
}

docker_apps_rs_image_clear() {
    # whoami; uname -a; pwd;
    docker rmi --force apps-rs-build-toolkit-al2 ;
}

docker_apps_rs_image_build() {
    DOCKER_BUILDKIT=1 docker build --no-cache -f ./rust-build-toolkit-al2.dockerfile -t "apps-rs-build-toolkit-al2" ./ ;
    # -ti
    docker create --name apps_rs_build apps-rs-build-toolkit-al2 sleep infinity; sleep 1;
    # docker create --name apps_rs_build apps-rs-build-toolkit-al2 tail -f /dev/null; sleep 1;
    # -tid
    # docker run -d --name apps_rs_build apps-rs-build-toolkit-al2 sleep infinity & disown; sleep 1;
    # docker run -d --name apps_rs_build apps-rs-build-toolkit-al2 tail -f /dev/null & disown; sleep 1;
    # docker stop apps_rs_build ;
    docker kill apps_rs_build ;
    docker start apps_rs_build ;
}

docker_prepare_apps_rs_buildenv() {
    docker exec -ti apps_rs_build bash -cis -- 'whoami; uname -a; pwd;' ;
    docker exec -ti apps_rs_build bash -cis -- "mkdir -vp /app-builder" ;
    docker exec -ti apps_rs_build bash -cis -- "cd /app-builder; git clone -o sentient.github https://github.com/andrcmdr/aws-nitro-enclaves-image-format.git ./eif_build" ;
    docker exec -ti apps_rs_build bash -cis -- "cd /app-builder; git clone -o sentient.github https://github.com/andrcmdr/aws-nitro-enclaves-image-format-build-extract.git ./eif_extract" ;
    docker exec -ti apps_rs_build bash -cis -- "cd /app-builder; git clone -o sentient.github https://github.com/andrcmdr/pipeline-tee.rs.git ./sse-sentinel-framework" ;
}

docker_apps_rs_build() {
    docker exec -ti apps_rs_build bash -cis -- "cd /app-builder/eif_build; git checkout 2fb5bc408357259eb30c6682429f252f8992c405; cargo build --all --release;" ;
    docker exec -ti apps_rs_build bash -cis -- "cd /app-builder/eif_extract; cargo build --all --release;" ;
    docker exec -ti apps_rs_build bash -cis -- "cd /app-builder/sse-sentinel-framework; cargo build --all --release;" ;
    mkdir -vp ./eif_build/ ;
    docker cp apps_rs_build:/app-builder/eif_build/target/release/eif_build ./eif_build/ ;
    mkdir -vp ./eif_extract/ ;
    docker cp apps_rs_build:/app-builder/eif_extract/target/release/eif_extract ./eif_extract/ ;
    docker cp apps_rs_build:/app-builder/eif_extract/target/release/eif_build ./eif_extract/ ;
    mkdir -vp ./sse-sentinel-framework/ ;
    docker cp apps_rs_build:/app-builder/sse-sentinel-framework/target/release/pipeline ./sse-sentinel-framework/ ;
    docker cp apps_rs_build:/app-builder/sse-sentinel-framework/target/release/ip-to-vsock ./sse-sentinel-framework/ ;
    docker cp apps_rs_build:/app-builder/sse-sentinel-framework/target/release/ip-to-vsock-transparent ./sse-sentinel-framework/ ;
    docker cp apps_rs_build:/app-builder/sse-sentinel-framework/target/release/vsock-to-ip ./sse-sentinel-framework/ ;
    docker cp apps_rs_build:/app-builder/sse-sentinel-framework/target/release/vsock-to-ip-transparent ./sse-sentinel-framework/ ;
    docker cp apps_rs_build:/app-builder/sse-sentinel-framework/target/release/transparent-port-to-vsock ./sse-sentinel-framework/ ;
    # docker stop apps_rs_build ;
    docker kill apps_rs_build ;
}

# Building Init system for enclave:

docker_init_clear() {
    docker kill init-build-blobs ;
    docker rm --force init-build-blobs ;
    docker rmi --force init-build-blobs ;
}

docker_init_build() {
    DOCKER_BUILDKIT=1 docker build --no-cache --output ./init_build/ --build-arg TARGET=all -f ./init_build/init-build-blobs.dockerfile -t "init-build-blobs" ./init_build/
    mkdir -vp ./init/ ./init_go/ ;
    cp -vr ./init_build/blobs/init/init ./init/ ;
    cp -vr ./init_build/blobs/init_go/init ./init_go/ ;
    # mkdir -vp ./eif_build/ ;
    # cp -vr ./init_build/blobs/eif_build/eif_build ./eif_build/ ;
    # cp -vr ./init_build/blobs/eif_extract/eif_extract ./eif_build/ ;
}

# Building enclave image (EIF):

docker_clear() {
    docker kill pipeline_toolkit ;
    docker rm --force pipeline_toolkit ;
    docker rmi --force pipeline-al2 ;
}

docker_build() {
    DOCKER_BUILDKIT=1 docker build --no-cache --build-arg FS=0 -f ./pipeline-al2.dockerfile -t "pipeline-al2" ./ ;
    docker create --name pipeline_toolkit pipeline-al2:latest ;
}

init_and_rootfs_base_images_build() {
    bsdtar -vpcf init.cpio --fflags --acls --xattrs --format newc -C ./init/ . ;
    bsdtar -vpcf init_go.cpio --fflags --acls --xattrs --format newc -C ./init_go/ . ;
    bsdtar -vpcf rootfs_base.cpio --fflags --acls --xattrs --format newc -C ./rootfs_base/ . ;
    bsdtar -vpcf rootfs_kmods.cpio --fflags --acls --xattrs --format newc -C ./rootfs_kmods/ . ;
}

docker_to_rootfs_fs_image_build() {
    docker export pipeline_toolkit | bsdtar -vpcf rootfs.cpio --fflags --acls --xattrs --format newc -X patterns -s ",^,rootfs/,S" @- ;
    bsdtar -vpcf rootfs.mtree --fflags --xattrs --format=mtree --options="mtree:all,mtree:indent" @rootfs.cpio ;
}

ramdisk_image_build() {
    bsdtar -vpcf rootfs_ramdisk.cpio --fflags --acls --xattrs --format newc @rootfs_base.cpio @rootfs.cpio @rootfs_kmods.cpio ;
}

eif_build_with_initc() {
    /usr/bin/time -v -o ./eif_build.log ./eif_build/eif_build --arch "x86_64" --build-time "$(date '+%FT%T.%N%:z')" --cmdline "reboot=k panic=30 pci=on nomodules console=ttyS0 i8042.noaux i8042.nomux i8042.nopnp i8042.dumbkbd random.trust_cpu=on" --kernel ./blobs/bzImage --kernel_config ./blobs/bzImage.config --name "app-builder-pipeline" --output ./app-builder-pipeline.eif --ramdisk ./init.cpio --ramdisk ./rootfs_ramdisk.cpio 2>&1 | tee app-builder-pipeline.eif.pcr; \
    /usr/bin/time -v -o ./describe-eif.log nitro-cli describe-eif --eif-path ./app-builder-pipeline.eif 2>&1 | tee app-builder-pipeline.eif.desc;
}

eif_build_with_initgo() {
    /usr/bin/time -v -o ./eif_build.log ./eif_build/eif_build --arch "x86_64" --build-time "$(date '+%FT%T.%N%:z')" --cmdline "reboot=k panic=30 pci=on nomodules console=ttyS0 i8042.noaux i8042.nomux i8042.nopnp i8042.dumbkbd random.trust_cpu=on" --kernel ./blobs/bzImage --kernel_config ./blobs/bzImage.config --name "app-builder-pipeline" --output ./app-builder-pipeline.eif --ramdisk ./init_go.cpio --ramdisk ./rootfs_ramdisk.cpio 2>&1 | tee app-builder-pipeline.eif.pcr; \
    /usr/bin/time -v -o ./describe-eif.log nitro-cli describe-eif --eif-path ./app-builder-pipeline.eif 2>&1 | tee app-builder-pipeline.eif.desc;
}

# Enclave run-time management commands:
# run enclave image file (EIF), connect/attach local terminal to enclave's console output, list running enclaves, terminate enclaves.

run_eif_image_debugmode_cli() {
    /usr/bin/time -v -o ./run-enclave.log nitro-cli run-enclave --cpu-count $enclave_cpus --memory $enclave_mem --eif-path ./app-builder-pipeline.eif --debug-mode --attach-console --enclave-cid $enclave_cid --enclave-name pipeline_toolkit 2>&1 | tee app-builder-pipeline.output
}

run_eif_image_debugmode() {
    /usr/bin/time -v -o ./run-enclave.log nitro-cli run-enclave --cpu-count $enclave_cpus --memory $enclave_mem --eif-path ./app-builder-pipeline.eif --debug-mode --enclave-cid $enclave_cid --enclave-name pipeline_toolkit 2>&1 | tee app-builder-pipeline.output
}

run_eif_image() {
    /usr/bin/time -v -o ./run-enclave.log nitro-cli run-enclave --cpu-count $enclave_cpus --memory $enclave_mem --eif-path ./app-builder-pipeline.eif --enclave-cid $enclave_cid --enclave-name pipeline_toolkit 2>&1 | tee app-builder-pipeline.output
}

attach_console_to_recent_enclave() {
    ENCLAVE_ID=$(nitro-cli describe-enclaves | jq -r ".[0].EnclaveID"); \
    nitro-cli console --enclave-id "${ENCLAVE_ID}" 2>&1 | tee -a app-builder-pipeline.output
}

attach_console_to_enclave() {
    nitro-cli console --enclave-name pipeline_toolkit 2>&1 | tee -a app-builder-pipeline.output
}

list_enclaves() {
    nitro-cli describe-enclaves --metadata 2>&1 | tee -a enclaves.list
}

drop_recent_enclave() {
    ENCLAVE_ID=$(nitro-cli describe-enclaves | jq -r ".[0].EnclaveID"); \
    sudo nitro-cli terminate-enclave --enclave-id "${ENCLAVE_ID}"
}

drop_enclave() {
    sudo nitro-cli terminate-enclave --enclave-name pipeline_toolkit
}

drop_enclaves_all() {
    sudo nitro-cli terminate-enclave --all
}

while true; do
    read -p "$(whoami | tr -d '\n')@$(hostname -s | tr -d '\n'):$(pwd | tr -d '\n') $( [[ "$(whoami | tr -d '\n')" == "root" ]] && echo -e "#" || echo -e "\$" )> " cmd

    # Type 'break' or 'exit', or push 'Ctrl+C' key sequence to exit from this shell
    if [[ $cmd == "break" || $cmd == "exit" ]]; then
        break
    fi

    # Print the filename of the terminal connected/attached to the standard input (to this shell)
    if [[ $cmd == "tty" ]]; then
        tty ;
        continue
    fi

    # EIF enclave image build commands

    if [[ $cmd == "docker_clear" ]]; then
        read -n 1 -s -p "Clear previous rootfs Docker container and container image first? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_clear ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    if [[ $cmd == "docker_build" ]]; then
        read -n 1 -s -p "Build rootfs Docker container image and create a container from it? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_build ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    if [[ $cmd == "init_and_rootfs_base_images_build" ]]; then
        read -n 1 -s -p "Build or rebuild init.c, init.go and rootfs base CPIO images? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            init_and_rootfs_base_images_build ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    if [[ $cmd == "docker_to_rootfs_fs_image_build" ]]; then
        read -n 1 -s -p "Export Docker image rootfs filesystem to CPIO image? (And make an mtree listing of CPIO archive) [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_to_rootfs_fs_image_build ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    if [[ $cmd == "ramdisk_image_build" ]]; then
        read -n 1 -s -p "Make a rootfs ramdisk image from rootfs base image and rootfs filesystem image (rootfs from Docker image, and including rootfs base image with Linux kernel modules)? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            ramdisk_image_build ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    if [[ $cmd == "eif_build_with_initc" ]]; then
        read -n 1 -s -p "Assemble and build EIF image from CPIO archive/image segments (with init.c)? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            eif_build_with_initc ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    if [[ $cmd == "eif_build_with_initgo" ]]; then
        read -n 1 -s -p "Assemble and build EIF image from CPIO archive/image segments (with init.go)? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            eif_build_with_initgo ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    # Enclave run-time management commands:
    # run enclave image file (EIF), connect/attach local terminal to enclave's console output, list running enclaves, terminate enclaves.

    if [[ $cmd == "run_eif_image_debugmode_cli" ]]; then
        read -n 1 -s -p "Run EIF image in enclave (Nitro Enclaves, KVM based VM) in debug mode (with attaching console for enclave debug output)? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            run_eif_image_debugmode_cli ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    if [[ $cmd == "run_eif_image_debugmode" ]]; then
        read -n 1 -s -p "Run EIF image in enclave (Nitro Enclaves, KVM based VM) in debug mode (without attaching console for enclave debug output)? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            run_eif_image_debugmode ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    if [[ $cmd == "run_eif_image" ]]; then
        read -n 1 -s -p "Run EIF image in enclave (Nitro Enclaves, KVM based VM) in production mode? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            run_eif_image ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    if [[ $cmd == "attach_console_to_recent_enclave" ]]; then
        read -n 1 -s -p "Attach local console to recently created and running enclave for debug CLI dump (stdout)? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            attach_console_to_recent_enclave ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    if [[ $cmd == "attach_console_to_enclave" ]]; then
        read -n 1 -s -p "Attach local console to created and running enclave for debug CLI dump (stdout)? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            attach_console_to_enclave ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    if [[ $cmd == "list_enclaves" ]]; then
        read -n 1 -s -p "List all running enclaves including its metadata? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            list_enclaves ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    if [[ $cmd == "drop_recent_enclave" ]]; then
        read -n 1 -s -p "Terminate recently created and running enclave? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            drop_recent_enclave ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    if [[ $cmd == "drop_enclave" ]]; then
        read -n 1 -s -p "Terminate created and running enclave? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            drop_enclave ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    if [[ $cmd == "drop_enclaves_all" ]]; then
        read -n 1 -s -p "Terminate all running enclaves? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            drop_enclaves_all ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    # EIF enclave image build automated guide

    if [[ $cmd == "make" ]]; then
        read -n 1 -s -p "Clear previous rootfs Docker container and container image first? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_clear ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Build rootfs Docker container image and create a container from it? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_build ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Build or rebuild init.c, init.go and rootfs base CPIO images? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            init_and_rootfs_base_images_build ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Export Docker image rootfs filesystem to CPIO image? (And make an mtree listing of CPIO archive) [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_to_rootfs_fs_image_build ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Make a rootfs ramdisk image from rootfs base image and rootfs filesystem image (rootfs from Docker image, and including rootfs base image with Linux kernel modules)? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            ramdisk_image_build ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Assemble and build EIF image from CPIO archive/image segments (with init.c)? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            eif_build_with_initc ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Assemble and build EIF image from CPIO archive/image segments (with init.go)? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            eif_build_with_initgo ; wait
        else
            echo -e "\n"
        fi

        continue
    fi

    # Enclave run-time management automated guide:
    # run enclave image file (EIF), connect/attach local terminal to enclave's console output, list running enclaves, terminate enclaves.

    if [[ $cmd == "make enclave" ]]; then
        read -n 1 -s -p "Run EIF image in enclave (Nitro Enclaves, KVM based VM) in debug mode (with attaching console for enclave debug output)? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            run_eif_image_debugmode_cli ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Run EIF image in enclave (Nitro Enclaves, KVM based VM) in debug mode (without attaching console for enclave debug output)? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            run_eif_image_debugmode ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Run EIF image in enclave (Nitro Enclaves, KVM based VM) in production mode? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            run_eif_image ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Attach local console to recently created and running enclave for debug CLI dump (stdout)? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            attach_console_to_recent_enclave ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Attach local console to created and running enclave for debug CLI dump (stdout)? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            attach_console_to_enclave ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "List all running enclaves including its metadata? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            list_enclaves ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Terminate recently created and running enclave? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            drop_recent_enclave ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Terminate created and running enclave? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            drop_enclave ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Terminate all running enclaves? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            drop_enclaves_all ; wait
        else
            echo -e "\n"
        fi

        continue
    fi

    # Kernel build commands

    if [[ $cmd == "docker_kcontainer_clear" ]]; then
        read -n 1 -s -p "Clear previous 'kernel_build' Docker container first? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_kcontainer_clear ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    if [[ $cmd == "docker_kimage_clear" ]]; then
        read -n 1 -s -p "Clear previous 'kernel_build' Docker container image? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_kimage_clear ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    if [[ $cmd == "docker_kimage_build" ]]; then
        read -n 1 -s -p "Build new 'kernel_build' Docker image and create container from it? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_kimage_build ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    if [[ $cmd == "docker_prepare_kbuildenv" ]]; then
        read -n 1 -s -p "Prepare 'kernel_build' environment in Docker container? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_prepare_kbuildenv ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    if [[ $cmd == "docker_kernel_build" ]]; then
        read -n 1 -s -p "Build custom Linux kernel in Docker 'kernel_build' container isolated environment? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_kernel_build ; wait
            echo -e "\nMaking of a kernel successfully done!\n"
        else
            echo -e "\n"
        fi
        continue
    fi

    # Kernel build automated guide

    if [[ $cmd == "make kernel" ]]; then
        read -n 1 -s -p "Clear previous 'kernel_build' Docker container first? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_kcontainer_clear ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Clear previous 'kernel_build' Docker container image? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_kimage_clear ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Build new 'kernel_build' Docker image and create container from it? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_kimage_build ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Prepare 'kernel_build' environment in Docker container? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_prepare_kbuildenv ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Build custom Linux kernel in Docker 'kernel_build' container isolated environment? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_kernel_build ; wait
            echo -e "\nMaking of a kernel successfully done!\n"
        else
            echo -e "\n"
        fi

        continue
    fi

    # Build commands for enclave's run-time Rust apps (SSE Framework) and for enclave's image (EIF) building tools

    if [[ $cmd == "docker_apps_rs_container_clear" ]]; then
        read -n 1 -s -p "Clear previous 'apps_rs_build' Docker container first? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_apps_rs_container_clear ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    if [[ $cmd == "docker_apps_rs_image_clear" ]]; then
        read -n 1 -s -p "Clear previous 'apps_rs_build' Docker container image? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_apps_rs_image_clear ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    if [[ $cmd == "docker_apps_rs_image_build" ]]; then
        read -n 1 -s -p "Build new 'apps_rs_build' Docker image and create container from it? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_apps_rs_image_build ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    if [[ $cmd == "docker_prepare_apps_rs_buildenv" ]]; then
        read -n 1 -s -p "Prepare apps repositories and environment in 'apps_rs_build' Docker container? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_prepare_apps_rs_buildenv ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    if [[ $cmd == "docker_apps_rs_build" ]]; then
        read -n 1 -s -p "Build all apps for EIF enclave image building and enclave's run-time in 'apps_rs_build' Docker container isolated environment? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_apps_rs_build ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    # Build automated guide for enclave's run-time Rust apps (SSE Framework) and for enclave's image (EIF) building tools

    if [[ $cmd == "make apps" ]]; then
        read -n 1 -s -p "Clear previous 'apps_rs_build' Docker container first? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_apps_rs_container_clear ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Clear previous 'apps_rs_build' Docker container image? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_apps_rs_image_clear ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Build new 'apps_rs_build' Docker image and create container from it? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_apps_rs_image_build ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Prepare apps repositories and environment in 'apps_rs_build' Docker container? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_prepare_apps_rs_buildenv ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Build all apps for EIF enclave image building and enclave's run-time in 'apps_rs_build' Docker container isolated environment? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_apps_rs_build ; wait
        else
            echo -e "\n"
        fi

        continue
    fi

    # Init system build commands

    if [[ $cmd == "docker_init_clear" ]]; then
        read -n 1 -s -p "Clear previous 'init_build' Docker container and container image first? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_init_clear ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    if [[ $cmd == "docker_init_build" ]]; then
        read -n 1 -s -p "Build custom init system for enclave image in Docker 'init_build' container isolated environment? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_init_build ; wait
        else
            echo -e "\n"
        fi
        continue
    fi

    # Init system build automated guide

    if [[ $cmd == "make init" ]]; then
        read -n 1 -s -p "Clear previous 'init_build' Docker container and container image first? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_init_clear ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Build custom init system for enclave image in Docker 'init_build' container isolated environment? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            docker_init_build ; wait
        else
            echo -e "\n"
        fi

        continue
    fi

done

