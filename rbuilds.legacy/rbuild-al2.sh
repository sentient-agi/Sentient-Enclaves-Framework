#!/bin/bash
##!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

declare debug=${1:-""} # Verbose messages mode for debugging

# Kernel full version, either substituted from CLI as second parameter to 'rbuild.sh' shell build script, or default version will be substituted
declare kversion_full=${2:-'6.12.0'}  # Linux kernel full version, including major.minor.patch, from Semantic Versioning notation
# Validation of kernel full version, using PCRE pattern matching
declare kversion="$(echo -E "${kversion_full}" | grep -iP '^(0|[1-9][0-9]*)(\.)(0|[1-9][0-9]*)(\.([1-9][0-9]*|0))?$')"
# Validation of kernel version, using PCRE pattern matching, for downloading kernel archive
# Linux kernel archive version, including major.minor version (from Semantic Versioning notation),
# but excluding x.x.patch version for x.x.0 versions of the kernel
declare kversion_archive="$(echo -E "${kversion}" | grep -iPo '^(0|[1-9][0-9]*)(\.)(0|[1-9][0-9]*)(\.[1-9][0-9]*|(?=\.0))?')"

declare kbuild_user="sentient_build" # Username for kernel build
declare kbuild_host="sentient_builder" #Hostname for kernel build

declare enclave_mem='838656' # MiBs of memory allocated for Nitro Enclaves runt-time
declare enclave_cpus='64' # Number of CPUs allocated for Nitro Enclaves runt-time
declare enclave_cid='127' # Enclave's VSock CID for data connect

declare eif_init='init_c_eif/'; # Run enclave EIF image with init.c or with init.go, 'init_c_eif/' or 'init_go_eif/'

declare question=0; # Ask a question before execution of any command
declare local_shell=0; # Evaluate and execute local shell commands as well in current shell

if [[ "$1" == "?" || "$1" == "-?" || "$1" == "h" || "$1" == "-h" || "$1" == "help" || "$1" == "--help" ]]; then
    echo -e "\nShell script to build custom kernel, Rust apps (SSE Framework) for eclave's run-time, init system for enclave, and to build enclave images (EIF) reproducibly.\n"
    echo -e "Type 'help' to print help and 'help_ext' to print extended help.\n"
    echo -e "Type 'q' to switch on/off questions before execution of any command.\n"
    echo -e "Type 'lsh' to switch on/off local shell access, to evaluate and execute local shell commands as well in current shell.\n"
    echo -e "\n"
    echo -e "Input 'make' to automatically setup, build, deploy and run all stack components in unattended mode.\n"
    echo -e "\n"
    echo -e "Specific 'make' commands for step by step guided setup, build and run all components:\n"
    echo -e "Input 'make nitro' command to setup Nitro Enclaves into system.\n"
    echo -e "Input 'make kernel' command to start building custom Linux kernel.\n"
    echo -e "Input 'make apps' command to start building Rust apps (SSE Framework) for enclave's run-time and to build enclave's image creation and extraction tools.\n"
    echo -e "Input 'make init' command to start building init system for enclave.\n"
    echo -e "Input 'make eif' command to start building enclave image (EIF).\n"
    echo -e "Input 'make enclave' command to manage encalves run-time: run enclave, attach debug console to enclave, list running enclaves and terminate one or all enclaves.\n"
    echo -e "Input 'make clear' to automatically clear all Docker containers and all Docker images.\n"
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

        Setup Nitro Enclaves into system:

        install_nitro_enclaves

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
    echo -e "Type 'q' to switch on/off questions before execution of any command.\n"
    echo -e "Type 'lsh' to switch on/off local shell access, to evaluate and execute local shell commands as well in current shell.\n"
    echo -e "\n"
    echo -e "Input 'make' to automatically setup, build, deploy and run all stack components in unattended mode.\n"
    echo -e "\n"
    echo -e "Specific 'make' commands for step by step guided setup, build and run all components:\n"
    echo -e "Input 'make nitro' command to setup Nitro Enclaves into system.\n"
    echo -e "Input 'make kernel' command to start building custom Linux kernel.\n"
    echo -e "Input 'make apps' command to start building Rust apps (SSE Framework) for enclave's run-time and to build enclave's image creation and extraction tools.\n"
    echo -e "Input 'make init' command to start building init system for enclave.\n"
    echo -e "Input 'make eif' command to start building enclave image (EIF).\n"
    echo -e "Input 'make enclave' command to manage encalves run-time: run enclave, attach debug console to enclave, list running enclaves and terminate one or all enclaves.\n"
    echo -e "Input 'make clear' to automatically clear all Docker containers and all Docker images.\n"
    echo -e "\n"
    echo -e "Type 'tty' to print the filename of the terminal connected/attached to the standard input (to this shell).\n"
    echo -e "Enter 'break' or 'exit', or push 'Ctrl+C' key sequence, for exit from this shell.\n"
}

help_ext() {
    echo -e "\nCommands for manual stages execution:

        Print help and print extended help commands:

        help
        help_ext

        Setup Nitro Enclaves into system:

        install_nitro_enclaves

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

# Setup Nitro Enclaves into system

install_nitro_enclaves() {
    if [[ $(uname -r | grep -iPo "\.amzn2\.") == '.amzn2.' ]]; then
        sudo yum install -y docker
        sudo amazon-linux-extras install -y aws-nitro-enclaves-cli
        sudo yum install -y aws-nitro-enclaves-cli-devel
        sudo yum install -y awscli
    elif [[ $(uname -r | grep -iPo "\.amzn2023\.") == '.amzn2023.' ]]; then
        sudo dnf install -y docker
        sudo dnf install -y aws-nitro-enclaves-cli aws-nitro-enclaves-cli-devel
        sudo dnf install -y awscli
    fi

    sudo usermod -aG docker $USER
    sudo usermod -aG ne $USER
    id $USER | grep -iP --color "(docker)|(ne)"
    groups | grep -iP --color "(docker)|(ne)"

sudo tee /etc/nitro_enclaves/allocator.yaml << CONF
---
# Enclave configuration file.
# How much memory to allocate for enclaves (in MiB).
memory_mib: ${enclave_mem}
# How many CPUs to reserve for enclaves.
cpu_count: ${enclave_cpus}
# Alternatively, the exact CPUs to be reserved for the enclave can be explicitly
# configured by using 'cpu_pool' (like below), instead of 'cpu_count'.
# Note: cpu_count and cpu_pool conflict with each other. Only use exactly one of them.
# cpu_pool: 0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47
# cpu_pool: 8-15,32-47
# cpu_pool: 8-47
CONF

    sudo systemctl enable docker && sudo systemctl start docker
    sudo systemctl enable nitro-enclaves-allocator.service && sudo systemctl start nitro-enclaves-allocator.service

    read -n 1 -s -p "Nitro Enclaves setup done, reboot system? [y|n] :" choice
    if [[ $choice == "y" ]]; then
        # sudo reboot
        sudo shutdown -r now
    else
        echo -e "Nitro Enclaves setup successfully done.\n"
    fi
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
    docker exec -i kernel_build_v${kversion} bash -cis -- 'whoami; uname -a; pwd;' ;
    docker exec -i kernel_build_v${kversion} bash -cis -- 'dnf install -y time which hostname git patch make gcc flex bison \
        elfutils elfutils-devel elfutils-libelf elfutils-libelf-devel elfutils-libs \
        kmod openssl openssl-devel openssl-libs bc perl gawk wget cpio tar bsdtar xz bzip2 gzip xmlto \
        ncurses ncurses-devel diffutils rsync' ;
    docker exec -i kernel_build_v${kversion} bash -cis -- 'dnf install -y --allowerasing curl' ;
    docker exec -i kernel_build_v${kversion} bash -cis -- "mkdir -vp /kbuilder; cd /kbuilder; \
        wget https://github.com/gregkh/linux/archive/v${kversion_archive}.tar.gz" ;
    docker exec -i kernel_build_v${kversion} bash -cis -- "cd /kbuilder; tar --same-owner --acls --xattrs --selinux -vpxf v${kversion_archive}.tar.gz -C ./" ;
    docker exec -i kernel_build_v${kversion} bash -cis -- "cd /kbuilder; mv -v ./linux-${kversion_archive} ./linux-v${kversion}" ;
    # Configurations to make kernel modules (mainly for networking) compiled statically with the kernel
    # docker cp ./kernel_config/artifacts_static/.config kernel_build_v${kversion}:/kbuilder/ ;
    # or as kernel modules, that are loaded dynamically into kernel space by kernel itself
    # docker cp ./kernel_config/artifacts_kmods/.config kernel_build_v${kversion}:/kbuilder/ ;
    # Make kernel with kernel modules loaded dynamically:
    docker cp ./kernel_config/artifacts_kmods/.config kernel_build_v${kversion}:/kbuilder/ ;
    docker exec -i kernel_build_v${kversion} bash -cis -- "cp -vr /kbuilder/.config /kbuilder/linux-v${kversion}/.config" ;
}

docker_kernel_build() {
    docker exec -i kernel_build_v${kversion} bash -cis -- "cd /kbuilder/linux-v${kversion}/; \
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
    docker exec -i kernel_build_v${kversion} bash -cis -- "cd /kbuilder; \
        mkdir -vp ./kartifacts/ ./kartifacts/ne/; \
        cp -vr /kbuilder/linux-v${kversion}/.config ./kartifacts/; \
        cp -vr /kbuilder/linux-v${kversion}/drivers/misc/nsm.ko ./kartifacts/ne/; \
        cp -vr /kbuilder/linux-v${kversion}/kernel_modules/lib/modules/${kversion}/kernel/drivers/misc/nsm.ko ./kartifacts/; \
        cp -vr /kbuilder/linux-v${kversion}/kernel_modules/ ./kartifacts/; \
        cp -vr /kbuilder/linux-v${kversion}/kernel_modules/lib/modules/${kversion}/kernel/drivers/misc/nsm.ko ./kartifacts/kernel_modules/; \
        mkdir -vp ./kartifacts/kernel_headers/arch/x86/; \
        cp -vr /kbuilder/linux-v${kversion}/arch/x86/include/ ./kartifacts/kernel_headers/arch/x86/; \
        mkdir -vp ./kartifacts/kernel_headers/; \
        cp -vr /kbuilder/linux-v${kversion}/include/ ./kartifacts/kernel_headers/; \
        mkdir -vp ./kartifacts/kernel_headers/usr/; \
        cp -vr /kbuilder/linux-v${kversion}/usr/dummy-include/ ./kartifacts/kernel_headers/usr/; \
        mkdir -vp ./kartifacts/kernel_headers/usr/; \
        cp -vr /kbuilder/linux-v${kversion}/usr/include/ ./kartifacts/kernel_headers/usr/; \
        cp -vr /kbuilder/linux-v${kversion}/arch/x86/boot/bzImage ./kartifacts/; \
        cp -vr /kbuilder/linux-v${kversion}/arch/x86/boot/compressed/vmlinux ./kartifacts/; \
        cp -vr /kbuilder/linux-v${kversion}/arch/x86/boot/compressed/vmlinux.bin ./kartifacts/; \
        cp -vr /kbuilder/linux-v${kversion}/arch/x86/boot/compressed/vmlinux.bin.gz ./kartifacts/; \
    " ;
    docker cp kernel_build_v${kversion}:/kbuilder/kartifacts/ ./kernel_blobs/ ;
    # docker stop kernel_build_v${kversion} ;
    docker kill kernel_build_v${kversion} ;
    mkdir -vp ./kernel/ ;
    cp -vr ./kernel_blobs/bzImage ./kernel/bzImage ;
    cp -vr ./kernel_blobs/.config ./kernel/bzImage.config ;
    cp -vr ./kernel_blobs/nsm.ko ./kernel/nsm.ko ; chmod -v +x ./kernel/nsm.ko ;
    echo "reboot=k panic=30 pci=on nomodules console=ttyS0 i8042.noaux i8042.nomux i8042.nopnp i8042.dumbkbd random.trust_cpu=on" > ./kernel/cmdline ;
    mkdir -vp ./cpio/ ./cpio/init/ ./cpio/init_go/ ;
    cp -vr ./kernel_blobs/nsm.ko ./cpio/init/nsm.ko ; chmod -v +x ./cpio/init/nsm.ko ;
    cp -vr ./kernel_blobs/nsm.ko ./cpio/init_go/nsm.ko ; chmod -v +x ./cpio/init_go/nsm.ko ;
    mkdir -vp ./cpio/rootfs_kmods/rootfs/usr/ ;
    cp -vr ./kernel_blobs/kernel_modules/lib/ ./cpio/rootfs_kmods/rootfs/usr/ ;
    mkdir -vp ./cpio/ ./rootfs_base/ ./rootfs_base/dev/ ./rootfs_base/proc/ ./rootfs_base/rootfs/ ./rootfs_base/sys/ ;
    cp -vr ./rootfs_base/ ./cpio/ ;
}

# Building of enclave's image building/extraction tools and enclave's run-time Rust apps (Sentient Secure Enclaves Framework):
# Pipeline (SLC protocol),
# EIF_build & EIF_extract,
# PF-Proxies,
# SLC & content encryption (+ encryption/decryption protocol test tools, + multi-hop PRE re-encryption protocol test tools, + KMS test tools),
# Web-RA (+ NSM & TPM test tools, + KMS test tools),
# FS-Monitor (inotify) for RA DB,
# Nitro-CLI mod, etc.

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
    docker exec -i apps_rs_build bash -cis -- 'whoami; uname -a; pwd;' ;
    docker exec -i apps_rs_build bash -cis -- "mkdir -vp /app-builder" ;
    docker exec -i apps_rs_build bash -cis -- "cd /app-builder; git clone -o sentient.github https://github.com/andrcmdr/aws-nitro-enclaves-image-format.git ./eif_build" ;
    docker exec -i apps_rs_build bash -cis -- "cd /app-builder; git clone -o sentient.github https://github.com/andrcmdr/aws-nitro-enclaves-image-format-build-extract.git ./eif_extract" ;
    docker exec -i apps_rs_build bash -cis -- "cd /app-builder; git clone -o sentient.github https://github.com/andrcmdr/pipeline-tee.rs.git ./secure-enclaves-framework" ;
}

docker_apps_rs_build() {
    docker exec -i apps_rs_build bash -cis -- "cd /app-builder/eif_build; git checkout 2fb5bc408357259eb30c6682429f252f8992c405; cargo build --all --release;" ;
    docker exec -i apps_rs_build bash -cis -- "cd /app-builder/eif_extract; cargo build --all --release;" ;
    docker exec -i apps_rs_build bash -cis -- "cd /app-builder/secure-enclaves-framework; cargo build --all --release;" ;
    mkdir -vp ./eif_build/ ;
    docker cp apps_rs_build:/app-builder/eif_build/target/release/eif_build ./eif_build/ ;
    mkdir -vp ./eif_extract/ ;
    docker cp apps_rs_build:/app-builder/eif_extract/target/release/eif_extract ./eif_extract/ ;
    docker cp apps_rs_build:/app-builder/eif_extract/target/release/eif_build ./eif_extract/ ;
    mkdir -vp ./secure-enclaves-framework/ ;
    docker cp apps_rs_build:/app-builder/secure-enclaves-framework/target/release/pipeline ./secure-enclaves-framework/ ;
    docker cp apps_rs_build:/app-builder/secure-enclaves-framework/target/release/ip-to-vsock ./secure-enclaves-framework/ ;
    docker cp apps_rs_build:/app-builder/secure-enclaves-framework/target/release/ip-to-vsock-transparent ./secure-enclaves-framework/ ;
    docker cp apps_rs_build:/app-builder/secure-enclaves-framework/target/release/vsock-to-ip ./secure-enclaves-framework/ ;
    docker cp apps_rs_build:/app-builder/secure-enclaves-framework/target/release/vsock-to-ip-transparent ./secure-enclaves-framework/ ;
    docker cp apps_rs_build:/app-builder/secure-enclaves-framework/target/release/transparent-port-to-vsock ./secure-enclaves-framework/ ;
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
    mkdir -vp ./init_build/init_blobs/eif_build/ ./init_build/init_blobs/eif_extract/ ./init_build/init_blobs/init/ ./init_build/init_blobs/init_go/ ;
    DOCKER_BUILDKIT=1 sudo docker build --no-cache --output ./init_build/ --build-arg TARGET=all -f ./init_build/init-build-blobs.dockerfile -t "init-build-blobs" ./init_build/
    mkdir -vp ./cpio/ ./cpio/init/ ./cpio/init_go/ ;
    cp -vr ./init_build/init_blobs/init/init ./cpio/init/ ;
    cp -vr ./init_build/init_blobs/init_go/init ./cpio/init_go/ ;
    # mkdir -vp ./eif_build/ ./eif_extract/;
    # cp -vr ./init_build/init_blobs/eif_build/eif_build ./eif_build/ ;
    # cp -vr ./init_build/init_blobs/eif_extract/eif_build ./eif_extract/ ;
    # cp -vr ./init_build/init_blobs/eif_extract/eif_extract ./eif_extract/ ;
}

# Building enclave image (EIF):

docker_clear() {
    docker kill pipeline_toolkit ;
    docker rm --force pipeline_toolkit ;
    docker rmi --force pipeline-al2 ;

    docker kill eif_build_toolkit ;
    docker rm --force eif_build_toolkit ;
    docker rmi --force eif-builder-al2 ;
}

docker_build() {
    DOCKER_BUILDKIT=1 docker build --no-cache --build-arg FS=0 -f ./pipeline-al2.dockerfile -t "pipeline-al2" ./ ;
    docker create --name pipeline_toolkit pipeline-al2:latest ;

    DOCKER_BUILDKIT=1 docker build --no-cache --build-arg FS=0 -f ./eif-builder-al2.dockerfile -t "eif-builder-al2" ./ ;
    # docker create --name eif_build_toolkit eif-builder-al2:latest ;
}

init_and_rootfs_base_images_build() {
    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ -i -a stdin -a stdout eif-builder-al2 bash -cis -- "cd /eif_builder/cpio/; bsdtar -vpcf init.cpio --fflags --acls --xattrs --format newc -C ./init/ . 2>&1"
    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ -i -a stdin -a stdout eif-builder-al2 bash -cis -- "cd /eif_builder/cpio/; bsdtar -vpcf init.mtree --fflags --xattrs --format=mtree --options='mtree:all,mtree:indent' @init.cpio 2>&1 ;"

    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ -i -a stdin -a stdout eif-builder-al2 bash -cis -- "cd /eif_builder/cpio/; bsdtar -vpcf init_go.cpio --fflags --acls --xattrs --format newc -C ./init_go/ . 2>&1"
    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ -i -a stdin -a stdout eif-builder-al2 bash -cis -- "cd /eif_builder/cpio/; bsdtar -vpcf init_go.mtree --fflags --xattrs --format=mtree --options='mtree:all,mtree:indent' @init_go.cpio 2>&1 ;"

    mkdir -vp ./cpio/ ./rootfs_base/ ./rootfs_base/dev/ ./rootfs_base/proc/ ./rootfs_base/rootfs/ ./rootfs_base/sys/ ;
    cp -vr ./rootfs_base/ ./cpio/ ;
    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ -i -a stdin -a stdout eif-builder-al2 bash -cis -- "cd /eif_builder/cpio/; bsdtar -vpcf rootfs_base.cpio --fflags --acls --xattrs --format newc -C ./rootfs_base/ . 2>&1"
    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ -i -a stdin -a stdout eif-builder-al2 bash -cis -- "cd /eif_builder/cpio/; bsdtar -vpcf rootfs_base.mtree --fflags --xattrs --format=mtree --options='mtree:all,mtree:indent' @rootfs_base.cpio 2>&1 ;"

    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ -i -a stdin -a stdout eif-builder-al2 bash -cis -- "cd /eif_builder/cpio/; bsdtar -vpcf rootfs_kmods.cpio --fflags --acls --xattrs --format newc -C ./rootfs_kmods/ . 2>&1"
    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ -i -a stdin -a stdout eif-builder-al2 bash -cis -- "cd /eif_builder/cpio/; bsdtar -vpcf rootfs_kmods.mtree --fflags --xattrs --format=mtree --options='mtree:all,mtree:indent' @rootfs_kmods.cpio 2>&1 ;"
}

docker_to_rootfs_fs_image_build() {
    docker export pipeline_toolkit | docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ -i -a stdin -a stdout eif-builder-al2 bash -cis -- "bsdtar -vpcf ./cpio/rootfs.cpio --fflags --acls --xattrs --format newc -X patterns -s ',^,rootfs/,S' @- 2>&1"
    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ -i -a stdin -a stdout eif-builder-al2 bash -cis -- "cd /eif_builder/cpio/; bsdtar -vpcf rootfs.mtree --fflags --xattrs --format=mtree --options='mtree:all,mtree:indent' @rootfs.cpio 2>&1 ;"
}

ramdisk_image_build() {
    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ -i -a stdin -a stdout eif-builder-al2 bash -cis -- "cd /eif_builder/cpio/; bsdtar -vpcf rootfs_ramdisk.cpio --fflags --acls --xattrs --format newc @rootfs_base.cpio @rootfs.cpio @rootfs_kmods.cpio 2>&1"
    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ -i -a stdin -a stdout eif-builder-al2 bash -cis -- "cd /eif_builder/cpio/; bsdtar -vpcf rootfs_ramdisk.mtree --fflags --xattrs --format=mtree --options='mtree:all,mtree:indent' @rootfs_ramdisk.cpio 2>&1 ;"
}

eif_build_with_initc() {
    mkdir -vp ./eif/ ./eif/init_c_eif/ ;
    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ --mount type=bind,src="$(pwd)"/eif/,dst=/eif_builder/eif/ --mount type=bind,src="$(pwd)"/kernel/,dst=/eif_builder/kernel/ -i -a stdin -a stdout eif-builder-al2 bash -cis -- "cd /eif_builder/; \
    /usr/bin/time -v -o ./eif/init_c_eif/eif_build.log ./eif_build --arch 'x86_64' --build-time "$(date '+%FT%T.%N%:z')" --cmdline 'reboot=k panic=30 pci=on nomodules console=ttyS0 i8042.noaux i8042.nomux i8042.nopnp i8042.dumbkbd random.trust_cpu=on' --kernel ./kernel/bzImage --kernel_config ./kernel/bzImage.config --name 'app-builder-secure-enclaves-framework' --output ./eif/init_c_eif/app-builder-secure-enclaves-framework.eif --ramdisk ./cpio/init.cpio --ramdisk ./cpio/rootfs_ramdisk.cpio 2>&1 | tee ./eif/init_c_eif/app-builder-secure-enclaves-framework.eif.pcr; \
    /usr/bin/time -v -o ./eif/init_c_eif/describe-eif.log nitro-cli describe-eif --eif-path ./eif/init_c_eif/app-builder-secure-enclaves-framework.eif 2>&1 | tee ./eif/init_c_eif/app-builder-secure-enclaves-framework.eif.desc;"
    ln -vf -rs ./eif/init_c_eif/app-builder-secure-enclaves-framework.eif ./eif/app-builder-secure-enclaves-framework.eif
    eif_init='init_c_eif/';
}

eif_build_with_initgo() {
    mkdir -vp ./eif/ ./eif/init_go_eif/ ;
    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ --mount type=bind,src="$(pwd)"/eif/,dst=/eif_builder/eif/ --mount type=bind,src="$(pwd)"/kernel/,dst=/eif_builder/kernel/ -i -a stdin -a stdout eif-builder-al2 bash -cis -- "cd /eif_builder/; \
    /usr/bin/time -v -o ./eif/init_go_eif/eif_build.log ./eif_build --arch 'x86_64' --build-time "$(date '+%FT%T.%N%:z')" --cmdline 'reboot=k panic=30 pci=on nomodules console=ttyS0 i8042.noaux i8042.nomux i8042.nopnp i8042.dumbkbd random.trust_cpu=on' --kernel ./kernel/bzImage --kernel_config ./kernel/bzImage.config --name 'app-builder-secure-enclaves-framework' --output ./eif/init_go_eif/app-builder-secure-enclaves-framework.eif --ramdisk ./cpio/init_go.cpio --ramdisk ./cpio/rootfs_ramdisk.cpio 2>&1 | tee ./eif/init_go_eif/app-builder-secure-enclaves-framework.eif.pcr; \
    /usr/bin/time -v -o ./eif/init_go_eif/describe-eif.log nitro-cli describe-eif --eif-path ./eif/init_go_eif/app-builder-secure-enclaves-framework.eif 2>&1 | tee ./eif/init_go_eif/app-builder-secure-enclaves-framework.eif.desc;"
    ln -vf -rs ./eif/init_go_eif/app-builder-secure-enclaves-framework.eif ./eif/app-builder-secure-enclaves-framework.eif
    eif_init='init_go_eif/';
}

# Enclave run-time management commands:
# run enclave image file (EIF), connect/attach local terminal to enclave's console output, list running enclaves, terminate enclaves.

run_eif_image_debugmode_cli() {
    /usr/bin/time -v -o ./eif/run-enclave.log nitro-cli run-enclave --cpu-count $enclave_cpus --memory $enclave_mem --eif-path ./eif/${eif_init}app-builder-secure-enclaves-framework.eif --debug-mode --attach-console --enclave-cid $enclave_cid --enclave-name app_builder_secure_enclaves_framework_toolkit 2>&1 | tee ./eif/app-builder-secure-enclaves-framework.output
}

run_eif_image_debugmode() {
    /usr/bin/time -v -o ./eif/run-enclave.log nitro-cli run-enclave --cpu-count $enclave_cpus --memory $enclave_mem --eif-path ./eif/${eif_init}app-builder-secure-enclaves-framework.eif --debug-mode --enclave-cid $enclave_cid --enclave-name app_builder_secure_enclaves_framework_toolkit 2>&1 | tee ./eif/app-builder-secure-enclaves-framework.output
}

run_eif_image() {
    /usr/bin/time -v -o ./eif/run-enclave.log nitro-cli run-enclave --cpu-count $enclave_cpus --memory $enclave_mem --eif-path ./eif/${eif_init}app-builder-secure-enclaves-framework.eif --enclave-cid $enclave_cid --enclave-name app_builder_secure_enclaves_framework_toolkit 2>&1 | tee ./eif/app-builder-secure-enclaves-framework.output
}

attach_console_to_recent_enclave() {
    ENCLAVE_ID=$(nitro-cli describe-enclaves | jq -r ".[0].EnclaveID"); \
    nitro-cli console --enclave-id "${ENCLAVE_ID}" 2>&1 | tee -a ./eif/app-builder-secure-enclaves-framework.output
}

attach_console_to_enclave() {
    nitro-cli console --enclave-name app_builder_secure_enclaves_framework_toolkit 2>&1 | tee -a ./eif/app-builder-secure-enclaves-framework.output
}

list_enclaves() {
    nitro-cli describe-enclaves --metadata 2>&1 | tee -a ./eif/enclaves.list
}

drop_recent_enclave() {
    ENCLAVE_ID=$(nitro-cli describe-enclaves | jq -r ".[0].EnclaveID"); \
    sudo nitro-cli terminate-enclave --enclave-id "${ENCLAVE_ID}"
}

drop_enclave() {
    sudo nitro-cli terminate-enclave --enclave-name app_builder_secure_enclaves_framework_toolkit
}

drop_enclaves_all() {
    sudo nitro-cli terminate-enclave --all
}

# Template executor facade function.
runner_fn() {
    declare -rA functions=(

        # Help commands

        ["help"]="Print help"
        ["help_success"]="\nFunction successfully executed!\n"
        ["help_ext"]="Print extended help"
        ["help_ext_success"]="\nFunction successfully executed!\n"

        # Setup Nitro Enclaves into system

        ["install_nitro_enclaves"]="Setup Nitro Enclaves into system"
        ["install_nitro_enclaves_success"]="\nNitro Enclaves setup command execution has been successfully done!\n"

        # Kernel build commands

        ["docker_kcontainer_clear"]="Clear previous 'kernel_build' Docker container first"

        ["docker_kimage_clear"]="Clear previous 'kernel_build' Docker container image"

        ["docker_kimage_build"]="Build new 'kernel_build' Docker image and create container from it"

        ["docker_prepare_kbuildenv"]="Prepare 'kernel_build' environment in Docker container"

        ["docker_kernel_build"]="Build custom Linux kernel in Docker 'kernel_build' container isolated environment"
        ["docker_kernel_build_success"]="\nMaking of a kernel successfully done!\n"

        # Build commands for enclave's run-time Rust apps (SSE Framework) and for enclave's image (EIF) building tools

        ["docker_apps_rs_container_clear"]="Clear previous 'apps_rs_build' Docker container first"

        ["docker_apps_rs_image_clear"]="Clear previous 'apps_rs_build' Docker container image"

        ["docker_apps_rs_image_build"]="Build new 'apps_rs_build' Docker image and create container from it"

        ["docker_prepare_apps_rs_buildenv"]="Prepare apps repositories and environment in 'apps_rs_build' Docker container"

        ["docker_apps_rs_build"]="Build all apps for EIF enclave image building and enclave's run-time in 'apps_rs_build' Docker container isolated environment"

        # Init system build commands

        ["docker_init_clear"]="Clear previous 'init_build' Docker container and container image first"

        ["docker_init_build"]="Build custom init system for enclave image in Docker 'init_build' container isolated environment"

        # EIF enclave image build commands

        ["docker_clear"]="Clear previous rootfs Docker container and container image first"

        ["docker_build"]="Build rootfs Docker container image and create a container from it"

        ["init_and_rootfs_base_images_build"]="Build or rebuild init.c, init.go and rootfs base CPIO images"

        ["docker_to_rootfs_fs_image_build"]="Export Docker image rootfs filesystem to CPIO image (and make an mtree listing of CPIO archive)"

        ["ramdisk_image_build"]="Make a rootfs ramdisk image from rootfs base image and rootfs filesystem image (rootfs from Docker image, and including rootfs base image with Linux kernel modules)"

        ["eif_build_with_initc"]="Assemble and build EIF image from CPIO archive/image segments (with init.c)"

        ["eif_build_with_initgo"]="Assemble and build EIF image from CPIO archive/image segments (with init.go)"

        # Enclave run-time management commands:
        # run enclave image file (EIF), connect/attach local terminal to enclave's console output, list running enclaves, terminate enclaves.

        ["run_eif_image_debugmode_cli"]="Run EIF image in enclave (Nitro Enclaves, KVM based VM) in debug mode (with attaching console for enclave debug output)"

        ["run_eif_image_debugmode"]="Run EIF image in enclave (Nitro Enclaves, KVM based VM) in debug mode (without attaching console for enclave debug output)"

        ["run_eif_image"]="Run EIF image in enclave (Nitro Enclaves, KVM based VM) in production mode"

        ["attach_console_to_recent_enclave"]="Attach local console to recently created and running enclave for debug CLI dump (stdout)"

        ["attach_console_to_enclave"]="Attach local console to created and running enclave for debug CLI dump (stdout)"

        ["list_enclaves"]="List all running enclaves including its metadata"

        ["drop_recent_enclave"]="Terminate recently created and running enclave"

        ["drop_enclave"]="Terminate created and running enclave"

        ["drop_enclaves_all"]="Terminate all running enclaves"
    )

    # Commands executor

    if [[ ${debug} == "--debug" ]]; then
        echo -e "Function name to call: $1\n"
        echo -e "Current function name length: ${#1}\n"
        if [[ ${#functions[$1]} -ne 0 ]]; then
            echo -e "Current function signature length: ${#functions[$1]}\n"
        else
            echo -e "Current function signature length: ${#1}\n"
        fi
        echo -e "Functions associative array contains ${#functions[@]} functions\n"
    fi

    if [[ ${#functions[$1]} -ne 0 && local_shell -eq 0 ]]; then
        if [[ question -eq 1 ]]; then
            read -n 1 -s -p "${functions[$1]}? [y|n] :" choice
            if [[ $choice == "y" ]]; then
                echo -e "\n"
                eval $1 ; wait
                echo -e "${functions["$1_success"]}"
            else
                echo -e "\n"
            fi
        else
            echo -e "${functions[$1]} :\n"
            eval $1 ; wait
            echo -e "${functions["$1_success"]}"
            echo -e "\n"
        fi
    elif [[ local_shell -eq 1 ]]; then
        if [[ question -eq 1 ]]; then
            read -n 1 -s -p "Execute command '${*}' in local shell unsafe mode? [y|n] :" choice
            if [[ $choice == "y" ]]; then
                echo -e "\n"
                eval $@ ; wait
                echo -e "${functions["$1_success"]}"
            else
                echo -e "\n"
            fi
        else
            echo -e "Executing command '${*}' in local shell unsafe mode:\n"
            eval $@ ; wait
            echo -e "${functions["$1_success"]}"
            echo -e "\n"
        fi
    else 
        return 0
    fi
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

    # Switch on/off questions before execution of any command
    if [[ $cmd == "q" ]]; then
        # question=$(( ! $question ))
        question=$(( 1 - $question ))
        echo "question == $question"
        continue
    fi

    # Switch on/off local shell access, to evaluate and execute local shell commands as well in current shell
    if [[ $cmd == "lsh" ]]; then
        # local_shell=$(( ! $local_shell ))
        local_shell=$(( 1 - $local_shell ))
        echo "local_shell == $local_shell"
        continue
    fi

    # Setup Nitro Enclaves into system, automated guide
    if [[ $cmd == "make nitro" ]]; then
        echo -e "Setup Nitro Enclaves into system, automated guide\n"

        question=1

        runner_fn install_nitro_enclaves

        question=0

        continue
    fi

    # Kernel build automated guide
    if [[ $cmd == "make kernel" ]]; then
        echo -e "Kernel build automated guide\n"

        question=1

        runner_fn docker_kcontainer_clear

        runner_fn docker_kimage_clear

        runner_fn docker_kimage_build

        runner_fn docker_prepare_kbuildenv

        runner_fn docker_kernel_build

        question=0

        continue
    fi

    # Build automated guide for enclave's run-time Rust apps (SSE Framework) and for enclave's image (EIF) building tools
    if [[ $cmd == "make apps" ]]; then
        echo -e "Build automated guide for enclave's run-time Rust apps (SSE Framework) and for enclave's image (EIF) building tools\n"

        question=1

        runner_fn docker_apps_rs_container_clear

        runner_fn docker_apps_rs_image_clear

        runner_fn docker_apps_rs_image_build

        runner_fn docker_prepare_apps_rs_buildenv

        runner_fn docker_apps_rs_build

        question=0

        continue
    fi

    # Init system build automated guide
    if [[ $cmd == "make init" ]]; then
        echo -e "Init system build automated guide\n"

        question=1

        runner_fn docker_init_clear

        runner_fn docker_init_build

        question=0

        continue
    fi

    # EIF enclave image build automated guide
    if [[ $cmd == "make eif" ]]; then
        echo -e "EIF enclave image build automated guide\n"

        question=1

        runner_fn docker_clear

        runner_fn docker_build

        runner_fn init_and_rootfs_base_images_build

        runner_fn docker_to_rootfs_fs_image_build

        runner_fn ramdisk_image_build

        runner_fn eif_build_with_initc

        runner_fn eif_build_with_initgo

        question=0

        continue
    fi

    # Enclave run-time management automated guide:
    # run enclave image file (EIF), connect/attach local terminal to enclave's console output, list running enclaves, terminate enclaves.
    if [[ $cmd == "make enclave" ]]; then
        echo -e "Enclave run-time management automated guide:\n"
        echo -e "run enclave image file (EIF), connect/attach local terminal to enclave's console output, list running enclaves, terminate enclaves.\n"

        question=1

        runner_fn run_eif_image_debugmode_cli

        runner_fn run_eif_image_debugmode

        runner_fn run_eif_image

        runner_fn attach_console_to_recent_enclave

        runner_fn attach_console_to_enclave

        runner_fn list_enclaves

        runner_fn drop_recent_enclave

        runner_fn drop_enclave

        runner_fn drop_enclaves_all

        question=0

        continue
    fi

    #  Automatically setup, build, deploy and run all stack components in unattended mode
    if [[ $cmd == "make" ]]; then
        echo -e "Automatically setup, build, deploy and run all stack components in unattended mode\n"

        question=0

        runner_fn docker_kcontainer_clear

        runner_fn docker_kimage_clear

        runner_fn docker_kimage_build

        runner_fn docker_prepare_kbuildenv

        runner_fn docker_kernel_build

        runner_fn docker_apps_rs_container_clear

        runner_fn docker_apps_rs_image_clear

        runner_fn docker_apps_rs_image_build

        runner_fn docker_prepare_apps_rs_buildenv

        runner_fn docker_apps_rs_build

        runner_fn docker_init_clear

        runner_fn docker_init_build

        runner_fn docker_clear

        runner_fn docker_build

        runner_fn init_and_rootfs_base_images_build

        runner_fn docker_to_rootfs_fs_image_build

        runner_fn ramdisk_image_build

        runner_fn eif_build_with_initc

        runner_fn eif_build_with_initgo

        sleep 3;

        eif_init='init_c_eif/';

        runner_fn run_eif_image_debugmode_cli

        question=0

        continue
    fi

    # Automatically clear all Docker containers and all Docker images
    # created during automated unattended installation process of setup, build, deploy and run all Secure Enclaves Framework stack components
    if [[ $cmd == "make clear" ]]; then
        echo -e "Automatically clear all Docker containers and all Docker images\n"

        question=0

        runner_fn docker_kcontainer_clear

        runner_fn docker_kimage_clear

        runner_fn docker_apps_rs_container_clear

        runner_fn docker_apps_rs_image_clear

        runner_fn docker_init_clear

        runner_fn docker_clear

        question=0

        continue
    fi

    if [[ ${#cmd} -ne 0 ]]; then
        runner_fn $cmd ; wait ; continue
    else
        continue
    fi

done

