#!/bin/bash
##!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

# Kernel full version, either substituted from CLI as second parameter to 'rbuild.sh' shell build script, or default version will be substituted
declare kversion_full='6.12.0'  # Linux kernel full version, including major.minor.patch, from Semantic Versioning notation
# Validation of kernel full version, using PCRE pattern matching
declare kversion="$(echo -E "${kversion_full}" | grep -iP '^(0|[1-9][0-9]*)(\.)(0|[1-9][0-9]*)(\.([1-9][0-9]*|0))?$')"
# Validation of kernel version, using PCRE pattern matching, for downloading kernel archive
# Linux kernel archive version, including major.minor version (from Semantic Versioning notation),
# but excluding x.x.patch version for x.x.0 versions of the kernel
declare kversion_archive="$(echo -E "${kversion}" | grep -iPo '^(0|[1-9][0-9]*)(\.)(0|[1-9][0-9]*)(\.[1-9][0-9]*|(?=\.0))?')"

declare kbuild_user="sentient_build" # Username for kernel build
declare kbuild_host="sentient_builder" # Hostname for kernel build

declare enclave_mem='838656' # MiBs of memory allocated for Nitro Enclaves runt-time
declare enclave_cpus='64' # Number of CPUs allocated for Nitro Enclaves runt-time
declare enclave_cid='127' # Enclave's VSock CID for SLC data connect

# Print help for commands

help() {
    echo -e "\nShell script to build custom kernel, Rust apps (SSE Framework) for eclave's run-time, init system for enclave, and to build enclave images (EIF) reproducibly.\n"
    echo -e "Type 'help' to print help, 'help_ext' to print extended help, and 'help_ext_man' to print extended help with man strings.\n"
    echo -e "Type 'q' to switch on/off questions before execution of any command.\n"
    echo -e "Type 'lsh' to switch on/off local shell access, to evaluate and execute local shell commands as well in current shell.\n"
    echo -e "\n"
    echo -e "Input 'make' or 'make all' to automatically setup, build, deploy and run all stack components in unattended mode.\n"
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
    echo -e "\nList of commands for manual stages execution:

        Print help and print extended help commands:

        help
        help_ext
        help_ext_man

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

        docker_init_container_clear
        docker_init_image_clear
        docker_init_build

        Build enclave image file (EIF) stages:

        docker_eif_build_container_clear
        docker_eif_build_image_clear
        docker_container_apps_image_build
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

        Macro commands:

        make_nitro
        make_kernel
        make_apps
        make_init
        make_eif
        make_all
        make_enclave
        make_clear

    "
}

help_ext_man() {
    echo -e "\nAll commands with its meaning (man strings/messages) from 'functions' dictrionary structure (associative array):\n"
    for key in "${fn_signatures[@]}"; do
        echo -e "       $key :: ${functions[$key]}\n"
    done
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
    echo -e "$(id -unr):$(id -gnr)\n"
    echo -e "$USER\n"
    id -Gnr $USER | grep -iP --color "(docker)|(ne)"
    id $USER | grep -iP --color "(docker)|(ne)"
    groups $USER | grep -iP --color "(docker)|(ne)"

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
    docker rmi --force kernel-build-toolkit-al2023:v${kversion} ;
}

docker_kimage_build() {
    DOCKER_BUILDKIT=1 docker build --no-cache -f ./rust-build-toolkit-al2023.dockerfile -t "kernel-build-toolkit-al2023:v${kversion}" ./ ;
    # -ti
    docker create --name kernel_build_v${kversion} kernel-build-toolkit-al2023:v${kversion} sleep infinity; sleep 1;
    # docker create --name kernel_build_v${kversion} kernel-build-toolkit-al2023:v${kversion} tail -f /dev/null; sleep 1;
    # -tid
    # docker run -d --name kernel_build_v${kversion} kernel-build-toolkit-al2023:v${kversion} sleep infinity & disown; sleep 1;
    # docker run -d --name kernel_build_v${kversion} kernel-build-toolkit-al2023:v${kversion} tail -f /dev/null & disown; sleep 1;
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
    if [[ ${network} -ne 0 || ${reverse_network} -ne 0 || ${forward_network} -ne 0 ]]; then
        docker cp ./kernel_config/artifacts_kmods/.config kernel_build_v${kversion}:/kbuilder/ ;
    else
        docker cp ./kernel_config/kernel_wo_net/.config kernel_build_v${kversion}:/kbuilder/ ;
    fi
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
    cp -vrf ./kernel_blobs/kartifacts/ -T ./kernel_blobs/ ;
    rm -rf ./kernel_blobs/kartifacts/ ;
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
    docker rmi --force apps-rs-build-toolkit-al2023 ;
}

docker_apps_rs_image_build() {
    DOCKER_BUILDKIT=1 docker build --no-cache -f ./rust-build-toolkit-al2023.dockerfile -t "apps-rs-build-toolkit-al2023" ./ ;
    # -ti
    docker create --name apps_rs_build apps-rs-build-toolkit-al2023 sleep infinity; sleep 1;
    # docker create --name apps_rs_build apps-rs-build-toolkit-al2023 tail -f /dev/null; sleep 1;
    # -tid
    # docker run -d --name apps_rs_build apps-rs-build-toolkit-al2023 sleep infinity & disown; sleep 1;
    # docker run -d --name apps_rs_build apps-rs-build-toolkit-al2023 tail -f /dev/null & disown; sleep 1;
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

    mkdir -vp ./secure-enclaves-framework/.config/ ./network.init/.config/ ./network.init/pf-proxy/ ;

    cp -vr ../pipeline/.config/config.toml ./secure-enclaves-framework/.config/ ;
    cp -vr ../pipeline/.config/config.toml ./network.init/.config/ ;

    cp -vr ../.bin/pipeline-dir ./secure-enclaves-framework/ ;
    cp -vr ../.bin/shell.sh ./secure-enclaves-framework/ ;
    cp -vr ../.bin/pipeline-dir ./network.init/ ;
    cp -vr ../.bin/shell.sh ./network.init/ ;

    cp -vr ./secure-enclaves-framework/pipeline ./network.init/ ;
    cp -vr ./secure-enclaves-framework/ip-to-vsock ./network.init/pf-proxy/ip2vs ;
    cp -vr ./secure-enclaves-framework/ip-to-vsock-transparent ./network.init/pf-proxy/ip2vs-tp ;
    cp -vr ./secure-enclaves-framework/transparent-port-to-vsock ./network.init/pf-proxy/tpp2vs ;
    cp -vr ./secure-enclaves-framework/vsock-to-ip ./network.init/pf-proxy/vs2ip ;
    cp -vr ./secure-enclaves-framework/vsock-to-ip-transparent ./network.init/pf-proxy/vs2ip-tp ;
}

# Building Init system for enclave:

docker_init_container_clear() {
    docker kill init-build-blobs ;
    docker rm --force init-build-blobs ;
}

docker_init_image_clear() {
    docker rmi --force init-build-blobs ;
}

docker_init_build() {
    mkdir -vp ./init_build/init_blobs/eif_build/ ./init_build/init_blobs/eif_extract/ ./init_build/init_blobs/init/ ./init_build/init_blobs/init_go/ ;
    DOCKER_BUILDKIT=1 sudo docker build --no-cache --output ./init_build/ --build-arg TARGET=all -f ./init_build/init-build-blobs.dockerfile -t "init-build-blobs" ./init_build/
    mkdir -vp ./cpio/ ./cpio/init/ ./cpio/init_go/ ;

    sudo find ./init_build/init_blobs/ -type f -exec chmod -v u+rw,g=,o= '{}' \;
    sudo find ./init_build/init_blobs/ -type d -exec chmod -v u=rwx,g=,o= '{}' \;
    sudo find ./init_build/init_blobs/ -type f -exec chown -v $(id -unr):$(id -gnr) '{}' \;
    sudo find ./init_build/init_blobs/ -type d -exec chown -v $(id -unr):$(id -gnr) '{}' \;

    sudo find ./cpio/init/ -type f -exec chmod -v u+rw,g=,o= '{}' \;
    sudo find ./cpio/init/ -type d -exec chmod -v u=rwx,g=,o= '{}' \;
    sudo find ./cpio/init/ -type f -exec chown -v $(id -unr):$(id -gnr) '{}' \;
    sudo find ./cpio/init/ -type d -exec chown -v $(id -unr):$(id -gnr) '{}' \;

    sudo find ./cpio/init_go/ -type f -exec chmod -v u+rw,g=,o= '{}' \;
    sudo find ./cpio/init_go/ -type d -exec chmod -v u=rwx,g=,o= '{}' \;
    sudo find ./cpio/init_go/ -type f -exec chown -v $(id -unr):$(id -gnr) '{}' \;
    sudo find ./cpio/init_go/ -type d -exec chown -v $(id -unr):$(id -gnr) '{}' \;

    # sudo find ./init_build/ -type f -exec chmod -v u+rw,g=,o= '{}' \;
    # sudo find ./init_build/ -type d -exec chmod -v u=rwx,g=,o= '{}' \;
    # sudo find ./init_build/ -type f -exec chown -v $(id -unr):$(id -gnr) '{}' \;
    # sudo find ./init_build/ -type d -exec chown -v $(id -unr):$(id -gnr) '{}' \;

    # sudo find ./cpio/ -type f -exec chmod -v u+rw,g=,o= '{}' \;
    # sudo find ./cpio/ -type d -exec chmod -v u=rwx,g=,o= '{}' \;
    # sudo find ./cpio/ -type f -exec chown -v $(id -unr):$(id -gnr) '{}' \;
    # sudo find ./cpio/ -type d -exec chown -v $(id -unr):$(id -gnr) '{}' \;

    cp -vr ./init_build/init_blobs/init/init ./cpio/init/ ;
    cp -vr ./init_build/init_blobs/init_go/init ./cpio/init_go/ ;
    # mkdir -vp ./eif_build/ ./eif_extract/;
    # cp -vr ./init_build/init_blobs/eif_build/eif_build ./eif_build/ ;
    # cp -vr ./init_build/init_blobs/eif_extract/eif_build ./eif_extract/ ;
    # cp -vr ./init_build/init_blobs/eif_extract/eif_extract ./eif_extract/ ;
}

# Building enclave image (EIF):

declare image_name="";
declare container_name="";

docker_eif_build_container_clear() {
    dockerfile=$(echo -E "${1:-"$dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*([^\s]*?)(\.dockerfile)$");
    image_name=$(echo -E "${1:-"$dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*\K([^\s]*?)(?=\.dockerfile)");
    if [[ -z ${dockerfile} ]]; then
        dockerfile=$(echo -E "${dockerfile:-"./pipeline-al2023.dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*([^\s]*?)(\.dockerfile)$");
        image_name=$(echo -E "${dockerfile:-"./pipeline-al2023.dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*\K([^\s]*?)(?=\.dockerfile)");
    fi
    container_name="${image_name}_toolkit";

    docker kill $container_name ;
    docker rm --force $container_name ;

    docker kill eif_build_toolkit ;
    docker rm --force eif_build_toolkit ;
}

docker_eif_build_image_clear() {
    dockerfile=$(echo -E "${1:-"$dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*([^\s]*?)(\.dockerfile)$");
    image_name=$(echo -E "${1:-"$dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*\K([^\s]*?)(?=\.dockerfile)");
    if [[ -z ${dockerfile} ]]; then
        dockerfile=$(echo -E "${dockerfile:-"./pipeline-al2023.dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*([^\s]*?)(\.dockerfile)$");
        image_name=$(echo -E "${dockerfile:-"./pipeline-al2023.dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*\K([^\s]*?)(?=\.dockerfile)");
    fi
    container_name="${image_name}_toolkit";

    docker rmi --force $image_name ;

    docker rmi --force eif-builder-al2023 ;
}

docker_container_apps_image_build() {
    dockerfile=$(echo -E "${1:-"$dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*([^\s]*?)(\.dockerfile)$");
    image_name=$(echo -E "${1:-"$dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*\K([^\s]*?)(?=\.dockerfile)");
    if [[ -z ${dockerfile} ]]; then
        dockerfile=$(echo -E "${dockerfile:-"./pipeline-al2023.dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*([^\s]*?)(\.dockerfile)$");
        image_name=$(echo -E "${dockerfile:-"./pipeline-al2023.dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*\K([^\s]*?)(?=\.dockerfile)");
    fi
    container_name="${image_name}_toolkit";

    if [[ ${network} -ne 0 ]]; then
        cp -vrf ./network.init/init_revp+tpp.sh ./network.init/init.sh ;
    elif [[ ${reverse_network} -ne 0 ]]; then
        cp -vrf ./network.init/init_revp.sh ./network.init/init.sh ;
    elif [[ ${forward_network} -ne 0 ]]; then
        cp -vrf ./network.init/init_tpp.sh ./network.init/init.sh ;
    else
        cp -vrf ./network.init/init_wo_net.sh ./network.init/init.sh ;
    fi

    DOCKER_BUILDKIT=1 docker build --no-cache --build-arg FS=0 -f $dockerfile -t "$image_name" ./ ;
    docker create --name $container_name $image_name:latest ;

    DOCKER_BUILDKIT=1 docker build --no-cache --build-arg FS=0 -f ./eif-builder-al2023.dockerfile -t "eif-builder-al2023" ./ ;
    # docker create --name eif_build_toolkit eif-builder-al2023:latest ;
}

init_and_rootfs_base_images_build() {
    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ -i -a stdin -a stdout eif-builder-al2023 bash -cis -- "cd /eif_builder/cpio/; bsdtar -vpcf init.cpio --fflags --acls --xattrs --format newc -C ./init/ . 2>&1"
    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ -i -a stdin -a stdout eif-builder-al2023 bash -cis -- "cd /eif_builder/cpio/; bsdtar -vpcf init.mtree --fflags --xattrs --format=mtree --options='mtree:all,mtree:indent' @init.cpio 2>&1 ;"

    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ -i -a stdin -a stdout eif-builder-al2023 bash -cis -- "cd /eif_builder/cpio/; bsdtar -vpcf init_go.cpio --fflags --acls --xattrs --format newc -C ./init_go/ . 2>&1"
    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ -i -a stdin -a stdout eif-builder-al2023 bash -cis -- "cd /eif_builder/cpio/; bsdtar -vpcf init_go.mtree --fflags --xattrs --format=mtree --options='mtree:all,mtree:indent' @init_go.cpio 2>&1 ;"

    mkdir -vp ./cpio/ ./rootfs_base/ ./rootfs_base/dev/ ./rootfs_base/proc/ ./rootfs_base/rootfs/ ./rootfs_base/sys/ ;
    cp -vr ./rootfs_base/ ./cpio/ ;
    if [[ ${network} -ne 0 || ${reverse_network} -ne 0 || ${forward_network} -ne 0 ]]; then
        cp -vr ./rootfs_base_net/ -T ./cpio/rootfs_base/ ;
    fi

    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ -i -a stdin -a stdout eif-builder-al2023 bash -cis -- "cd /eif_builder/cpio/; bsdtar -vpcf rootfs_base.cpio --fflags --acls --xattrs --format newc -C ./rootfs_base/ . 2>&1"
    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ -i -a stdin -a stdout eif-builder-al2023 bash -cis -- "cd /eif_builder/cpio/; bsdtar -vpcf rootfs_base.mtree --fflags --xattrs --format=mtree --options='mtree:all,mtree:indent' @rootfs_base.cpio 2>&1 ;"

    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ -i -a stdin -a stdout eif-builder-al2023 bash -cis -- "cd /eif_builder/cpio/; bsdtar -vpcf rootfs_kmods.cpio --fflags --acls --xattrs --format newc -C ./rootfs_kmods/ . 2>&1"
    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ -i -a stdin -a stdout eif-builder-al2023 bash -cis -- "cd /eif_builder/cpio/; bsdtar -vpcf rootfs_kmods.mtree --fflags --xattrs --format=mtree --options='mtree:all,mtree:indent' @rootfs_kmods.cpio 2>&1 ;"
}

docker_to_rootfs_fs_image_build() {
    dockerfile=$(echo -E "${1:-"$dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*([^\s]*?)(\.dockerfile)$");
    image_name=$(echo -E "${1:-"$dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*\K([^\s]*?)(?=\.dockerfile)");
    if [[ -z ${dockerfile} ]]; then
        dockerfile=$(echo -E "${dockerfile:-"./pipeline-al2023.dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*([^\s]*?)(\.dockerfile)$");
        image_name=$(echo -E "${dockerfile:-"./pipeline-al2023.dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*\K([^\s]*?)(?=\.dockerfile)");
    fi
    container_name="${image_name}_toolkit";

    docker export $container_name | docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ -i -a stdin -a stdout eif-builder-al2023 bash -cis -- "bsdtar -vpcf ./cpio/rootfs.cpio --fflags --acls --xattrs --format newc -X patterns -s ',^,rootfs/,S' @- 2>&1"
    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ -i -a stdin -a stdout eif-builder-al2023 bash -cis -- "cd /eif_builder/cpio/; bsdtar -vpcf rootfs.mtree --fflags --xattrs --format=mtree --options='mtree:all,mtree:indent' @rootfs.cpio 2>&1 ;"
}

ramdisk_image_build() {
    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ -i -a stdin -a stdout eif-builder-al2023 bash -cis -- "cd /eif_builder/cpio/; bsdtar -vpcf rootfs_ramdisk.cpio --fflags --acls --xattrs --format newc @rootfs_base.cpio @rootfs.cpio @rootfs_kmods.cpio 2>&1"
    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ -i -a stdin -a stdout eif-builder-al2023 bash -cis -- "cd /eif_builder/cpio/; bsdtar -vpcf rootfs_ramdisk.mtree --fflags --xattrs --format=mtree --options='mtree:all,mtree:indent' @rootfs_ramdisk.cpio 2>&1 ;"
}

eif_build_with_initc() {
    mkdir -vp ./eif/ ./eif/init_c_eif/ ;

    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ --mount type=bind,src="$(pwd)"/eif/,dst=/eif_builder/eif/ --mount type=bind,src="$(pwd)"/kernel/,dst=/eif_builder/kernel/ -i -a stdin -a stdout eif-builder-al2023 bash -cis -- "cd /eif_builder/; \
    /usr/bin/time -v -o ./eif/init_c_eif/eif_build.log ./eif_build --arch 'x86_64' --build-time "$(date '+%FT%T.%N%:z')" --cmdline 'reboot=k panic=30 pci=on nomodules console=ttyS0 i8042.noaux i8042.nomux i8042.nopnp i8042.dumbkbd random.trust_cpu=on' --kernel ./kernel/bzImage --kernel_config ./kernel/bzImage.config --name 'app-builder-secure-enclaves-framework' --output ./eif/init_c_eif/app-builder-secure-enclaves-framework.eif --ramdisk ./cpio/init.cpio --ramdisk ./cpio/rootfs_ramdisk.cpio 2>&1 | tee ./eif/init_c_eif/app-builder-secure-enclaves-framework.eif.pcr; \
    /usr/bin/time -v -o ./eif/init_c_eif/describe-eif.log nitro-cli describe-eif --eif-path ./eif/init_c_eif/app-builder-secure-enclaves-framework.eif 2>&1 | tee ./eif/init_c_eif/app-builder-secure-enclaves-framework.eif.desc;"

    ln -vf -rs ./eif/init_c_eif/app-builder-secure-enclaves-framework.eif ./eif/app-builder-secure-enclaves-framework.eif
    eif_init='init_c_eif/';
}

eif_build_with_initgo() {
    mkdir -vp ./eif/ ./eif/init_go_eif/ ;

    docker run --rm --name eif_build_toolkit --mount type=bind,src="$(pwd)"/cpio/,dst=/eif_builder/cpio/ --mount type=bind,src="$(pwd)"/eif/,dst=/eif_builder/eif/ --mount type=bind,src="$(pwd)"/kernel/,dst=/eif_builder/kernel/ -i -a stdin -a stdout eif-builder-al2023 bash -cis -- "cd /eif_builder/; \
    /usr/bin/time -v -o ./eif/init_go_eif/eif_build.log ./eif_build --arch 'x86_64' --build-time "$(date '+%FT%T.%N%:z')" --cmdline 'reboot=k panic=30 pci=on nomodules console=ttyS0 i8042.noaux i8042.nomux i8042.nopnp i8042.dumbkbd random.trust_cpu=on' --kernel ./kernel/bzImage --kernel_config ./kernel/bzImage.config --name 'app-builder-secure-enclaves-framework' --output ./eif/init_go_eif/app-builder-secure-enclaves-framework.eif --ramdisk ./cpio/init_go.cpio --ramdisk ./cpio/rootfs_ramdisk.cpio 2>&1 | tee ./eif/init_go_eif/app-builder-secure-enclaves-framework.eif.pcr; \
    /usr/bin/time -v -o ./eif/init_go_eif/describe-eif.log nitro-cli describe-eif --eif-path ./eif/init_go_eif/app-builder-secure-enclaves-framework.eif 2>&1 | tee ./eif/init_go_eif/app-builder-secure-enclaves-framework.eif.desc;"

    ln -vf -rs ./eif/init_go_eif/app-builder-secure-enclaves-framework.eif ./eif/app-builder-secure-enclaves-framework.eif
    eif_init='init_go_eif/';
}

# Enclave run-time management commands:
# run enclave image file (EIF), connect/attach local terminal to enclave's console output, list running enclaves, terminate enclaves.

run_eif_image_debugmode_cli() {
    if [[ ${network} -ne 0 ]]; then
        cd ./network.init/;
        bash ./pf-rev-host.sh 2>&1 & disown;
        bash ./pf-tp-host.sh 2>&1 & disown;
        bash ./pf-host.sh 2>&1 & disown;
        cd ../ ;
    elif [[ ${reverse_network} -ne 0 ]]; then
        cd ./network.init/;
        bash ./pf-rev-host.sh 2>&1 & disown;
        cd ../ ;
    elif [[ ${forward_network} -ne 0 ]]; then
        cd ./network.init/;
        bash ./pf-tp-host.sh 2>&1 & disown;
        bash ./pf-host.sh 2>&1 & disown;
        cd ../ ;
    fi

    /usr/bin/time -v -o ./eif/run-enclave.log nitro-cli run-enclave --cpu-count $enclave_cpus --memory $enclave_mem --eif-path ./eif/${eif_init}app-builder-secure-enclaves-framework.eif --debug-mode --attach-console --enclave-cid $enclave_cid --enclave-name app_builder_secure_enclaves_framework_toolkit 2>&1 | tee ./eif/app-builder-secure-enclaves-framework.output
}

run_eif_image_debugmode() {
    if [[ ${network} -ne 0 ]]; then
        cd ./network.init/;
        bash ./pf-rev-host.sh 2>&1 & disown;
        bash ./pf-tp-host.sh 2>&1 & disown;
        bash ./pf-host.sh 2>&1 & disown;
        cd ../ ;
    elif [[ ${reverse_network} -ne 0 ]]; then
        cd ./network.init/;
        bash ./pf-rev-host.sh 2>&1 & disown;
        cd ../ ;
    elif [[ ${forward_network} -ne 0 ]]; then
        cd ./network.init/;
        bash ./pf-tp-host.sh 2>&1 & disown;
        bash ./pf-host.sh 2>&1 & disown;
        cd ../ ;
    fi

    /usr/bin/time -v -o ./eif/run-enclave.log nitro-cli run-enclave --cpu-count $enclave_cpus --memory $enclave_mem --eif-path ./eif/${eif_init}app-builder-secure-enclaves-framework.eif --debug-mode --enclave-cid $enclave_cid --enclave-name app_builder_secure_enclaves_framework_toolkit 2>&1 | tee ./eif/app-builder-secure-enclaves-framework.output
}

run_eif_image() {
    if [[ ${network} -ne 0 ]]; then
        cd ./network.init/;
        bash ./pf-rev-host.sh 2>&1 & disown;
        bash ./pf-tp-host.sh 2>&1 & disown;
        bash ./pf-host.sh 2>&1 & disown;
        cd ../ ;
    elif [[ ${reverse_network} -ne 0 ]]; then
        cd ./network.init/;
        bash ./pf-rev-host.sh 2>&1 & disown;
        cd ../ ;
    elif [[ ${forward_network} -ne 0 ]]; then
        cd ./network.init/;
        bash ./pf-tp-host.sh 2>&1 & disown;
        bash ./pf-host.sh 2>&1 & disown;
        cd ../ ;
    fi

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

# Macro commands

# Setup Nitro Enclaves into system, automated guide
make_nitro() {
    echo -e "Setup Nitro Enclaves into system, automated guide\n"

    # question=0

    runner_fn install_nitro_enclaves

    # question=0
}

# Kernel build automated guide
make_kernel() {
    echo -e "Kernel build automated guide\n"

    # question=0

    runner_fn docker_kcontainer_clear

    runner_fn docker_kimage_clear

    runner_fn docker_kimage_build

    runner_fn docker_prepare_kbuildenv

    runner_fn docker_kernel_build

    # question=0
}

# Build automated guide for enclave's run-time Rust apps (SSE Framework) and for enclave's image (EIF) building tools
make_apps() {
    echo -e "Build automated guide for enclave's run-time Rust apps (SSE Framework) and for enclave's image (EIF) building tools\n"

    # question=0

    runner_fn docker_apps_rs_container_clear

    runner_fn docker_apps_rs_image_clear

    runner_fn docker_apps_rs_image_build

    runner_fn docker_prepare_apps_rs_buildenv

    runner_fn docker_apps_rs_build

    # question=0
}

# Init system build automated guide
make_init() {
    echo -e "Init system build automated guide\n"

    # question=0

    runner_fn docker_init_container_clear

    runner_fn docker_init_image_clear

    runner_fn docker_init_build

    # question=0
}

# EIF enclave image build automated guide
make_eif() {
    echo -e "EIF enclave image build automated guide\n"

    dockerfile=$(echo -E "${1:-"$dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*([^\s]*?)(\.dockerfile)$");
    if [[ -z ${dockerfile} ]]; then
        dockerfile=$(echo -E "${dockerfile:-"./pipeline-al2023.dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*([^\s]*?)(\.dockerfile)$");
    fi

    # question=0

    runner_fn docker_eif_build_container_clear "${dockerfile}"

    runner_fn docker_eif_build_image_clear "${dockerfile}"

    runner_fn docker_container_apps_image_build "${dockerfile}"

    runner_fn init_and_rootfs_base_images_build

    runner_fn docker_to_rootfs_fs_image_build "${dockerfile}"

    runner_fn ramdisk_image_build

    runner_fn eif_build_with_initc

    runner_fn eif_build_with_initgo

    # question=0
}

# Automatically setup, build, deploy and run all stack components in unattended mode
make_all() {
    echo -e "Automatically setup, build, deploy and run all stack components in unattended mode\n"

    dockerfile=$(echo -E "${1:-"$dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*([^\s]*?)(\.dockerfile)$");
    if [[ -z ${dockerfile} ]]; then
        dockerfile=$(echo -E "${dockerfile:-"./pipeline-al2023.dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*([^\s]*?)(\.dockerfile)$");
    fi

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

    runner_fn docker_init_container_clear

    runner_fn docker_init_image_clear

    runner_fn docker_init_build

    runner_fn docker_eif_build_container_clear "${dockerfile}"

    runner_fn docker_eif_build_image_clear "${dockerfile}"

    runner_fn docker_container_apps_image_build "${dockerfile}"

    runner_fn init_and_rootfs_base_images_build

    runner_fn docker_to_rootfs_fs_image_build "${dockerfile}"

    runner_fn ramdisk_image_build

    runner_fn eif_build_with_initc

    runner_fn eif_build_with_initgo

    sleep 3;

    runner_fn run_eif_image_debugmode_cli

    question=0
}

# Enclave run-time management automated guide:
# run enclave image file (EIF), connect/attach local terminal to enclave's console output, list running enclaves, terminate enclaves.
make_enclave() {
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
}

# Automatically clear all Docker containers and all Docker images
# created during automated unattended installation process of setup, build, deploy and run all Secure Enclaves Framework stack components
make_clear() {
    echo -e "Automatically clear all Docker containers and all Docker images\n"

    dockerfile=$(echo -E "${1:-"$dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*([^\s]*?)(\.dockerfile)$");
    if [[ -z ${dockerfile} ]]; then
        dockerfile=$(echo -E "${dockerfile:-"./pipeline-al2023.dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*([^\s]*?)(\.dockerfile)$");
    fi

    # question=0

    runner_fn docker_kcontainer_clear

    runner_fn docker_kimage_clear

    runner_fn docker_apps_rs_container_clear

    runner_fn docker_apps_rs_image_clear

    runner_fn docker_init_container_clear

    runner_fn docker_init_image_clear

    runner_fn docker_eif_build_container_clear "${dockerfile}"

    runner_fn docker_eif_build_image_clear "${dockerfile}"

    # question=0
}

# Function signatures to formatting man help messages/strings output in correct order
declare -ra fn_signatures=(

    # Help commands

    "help"
    "help_success"
    "help_ext"
    "help_ext_success"
    "help_ext_man"
    "help_ext_man_success"

    # Setup Nitro Enclaves into system

    "install_nitro_enclaves"
    "install_nitro_enclaves_success"

    # Kernel build commands

    "docker_kcontainer_clear"

    "docker_kimage_clear"

    "docker_kimage_build"

    "docker_prepare_kbuildenv"

    "docker_kernel_build"
    "docker_kernel_build_success"

    # Build commands for enclave's run-time Rust apps (SSE Framework) and for enclave's image (EIF) building tools

    "docker_apps_rs_container_clear"

    "docker_apps_rs_image_clear"

    "docker_apps_rs_image_build"

    "docker_prepare_apps_rs_buildenv"

    "docker_apps_rs_build"

    # Init system build commands

    "docker_init_container_clear"

    "docker_init_image_clear"

    "docker_init_build"

    # EIF enclave image build commands

    "docker_eif_build_container_clear"

    "docker_eif_build_image_clear"

    "docker_container_apps_image_build"

    "init_and_rootfs_base_images_build"

    "docker_to_rootfs_fs_image_build"

    "ramdisk_image_build"

    "eif_build_with_initc"

    "eif_build_with_initgo"

    # Enclave run-time management commands:
    # run enclave image file (EIF), connect/attach local terminal to enclave's console output, list running enclaves, terminate enclaves.

    "run_eif_image_debugmode_cli"

    "run_eif_image_debugmode"

    "run_eif_image"

    "attach_console_to_recent_enclave"

    "attach_console_to_enclave"

    "list_enclaves"

    "drop_recent_enclave"

    "drop_enclave"

    "drop_enclaves_all"

    # Macro commands

    "make_nitro"

    "make_kernel"

    "make_apps"

    "make_init"

    "make_eif"

    "make_all"

    "make_enclave"

    "make_clear"

)

# Functions list with man help messages/strings
declare -rA functions=(

    # Help commands

    ["help"]="Print help"
    ["help_success"]="\nFunction successfully executed!\n"
    ["help_ext"]="Print extended help"
    ["help_ext_success"]="\nFunction successfully executed!\n"
    ["help_ext_man"]="Print extended help with man strings"
    ["help_ext_man_success"]="\nFunction successfully executed!\n"

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

    ["docker_init_container_clear"]="Clear previous 'init_build' Docker container first"

    ["docker_init_image_clear"]="Clear previous 'init_build' Docker container image"

    ["docker_init_build"]="Build custom init system for enclave image in Docker 'init_build' container isolated environment"

    # EIF enclave image build commands

    ["docker_eif_build_container_clear"]="Clear previous rootfs Docker container first"

    ["docker_eif_build_image_clear"]="Clear previous rootfs Docker container image"

    ["docker_container_apps_image_build"]="Build rootfs Docker container image and create a container from it"

    ["init_and_rootfs_base_images_build"]="Build or rebuild init.c, init.go and rootfs base CPIO images"

    ["docker_to_rootfs_fs_image_build"]="Export Docker image rootfs filesystem to CPIO image (and make an mtree listing of CPIO archive)"

    ["ramdisk_image_build"]="Make a rootfs ramdisk image from rootfs base image and rootfs filesystem image (rootfs from Docker image, and including rootfs base image with Linux kernel modules)"

    ["eif_build_with_initc"]="Assemble and build EIF image from CPIO archive/image segments (with init.c)"

    ["eif_build_with_initgo"]="Assemble and build EIF image from CPIO archive/image segments (with init.go)"

    # Enclave run-time management commands:
    # run enclave image file (EIF), connect/attach local terminal to enclave's console output, list running enclaves, terminate enclaves.

    ["run_eif_image_debugmode_cli"]="Run EIF image in enclave (Nitro Enclaves, KVM based VM) in debug mode (with attaching console for enclave debug output) and with host networking support (when '--network' flag is enabled)"

    ["run_eif_image_debugmode"]="Run EIF image in enclave (Nitro Enclaves, KVM based VM) in debug mode (without attaching console for enclave debug output) and with host networking support (when '--network' flag is enabled)"

    ["run_eif_image"]="Run EIF image in enclave (Nitro Enclaves, KVM based VM) in production mode and with host networking support (when '--network' flag is enabled)"

    ["attach_console_to_recent_enclave"]="Attach local console to recently created and running enclave for debug CLI dump (stdout)"

    ["attach_console_to_enclave"]="Attach local console to created and running enclave for debug CLI dump (stdout)"

    ["list_enclaves"]="List all running enclaves including its metadata"

    ["drop_recent_enclave"]="Terminate recently created and running enclave"

    ["drop_enclave"]="Terminate created and running enclave"

    ["drop_enclaves_all"]="Terminate all running enclaves"

    # Macro commands

    ["make_nitro"]="Setup Nitro Enclaves into system, automated guide"

    ["make_kernel"]="Kernel build automated guide"

    ["make_apps"]="Build automated guide for enclave's run-time Rust apps (SSE Framework) and for enclave's image (EIF) building tools"

    ["make_init"]="Init system build automated guide"

    ["make_eif"]="EIF enclave image build automated guide"

    ["make_all"]="Automatically setup, build, deploy and run all stack components in unattended mode"

    ["make_enclave"]="Enclave run-time management automated guide:"

    ["make_clear"]="Automatically clear all Docker containers and all Docker images"

)

# Template executor facade function.
runner_fn() {

    # Commands executor

    # Verbose messages mode for debugging of commands runner for functions
    if [[ ${debug} -ne 0 ]]; then
        echo -e "\n"
        echo -e "Function name to call: ${1}\n"
        echo -e "Current function signature/name length: ${#1}\n"
        echo -e "Command to call: ${@}\n"
        echo -e "Current command tokens/words length: ${#@}\n"
        if [[ ${#functions[$1]} -ne 0 ]]; then
            echo -e "Current function message length: ${#functions[$1]}\n"
        else
            echo -e "Current function signature/name length: ${#1}\n"
        fi
        echo -e "Functions associative array contains ${#functions[@]} functions\n"
    fi

    # Running exact functions with validation or without validation (for lacal shell commands)
    if [[ ${#functions[$1]} -ne 0 && local_shell -eq 0 ]]; then
        if [[ question -eq 1 ]]; then
            read -n 1 -s -p "${functions[$1]}? [y|n] :" choice
            if [[ $choice == "y" ]]; then
                echo -e "\n"
                # [[ ${tty_dev} -ne 0  ]] && eval "${@}" >&3 2>&3 ; wait || eval "${@}" ; wait
                if [[ ${tty_dev} -ne 0 ]]; then
                    eval "${@}" >&3 2>&3 ; wait
                else
                    eval "${@}" ; wait
                fi
                echo -e "${functions["$1_success"]}"
            else
                echo -e "\n"
            fi
        else
            echo -e "${functions[$1]} :\n"
            # [[ ${tty_dev} -ne 0  ]] && eval "${@}" >&3 2>&3 ; wait || eval "${@}" ; wait
            if [[ ${tty_dev} -ne 0 ]]; then
                eval "${@}" >&3 2>&3 ; wait
            else
                eval "${@}" ; wait
            fi
            echo -e "${functions["$1_success"]}"
            echo -e "\n"
        fi
    elif [[ local_shell -eq 1 ]]; then
        if [[ question -eq 1 ]]; then
            if [[ ${#functions[$1]} -ne 0 ]]; then
                echo -e "${functions[$1]}? :\n"
            fi
            read -n 1 -s -p "Execute command '${*}' in local shell unsafe mode? [y|n] :" choice
            if [[ $choice == "y" ]]; then
                echo -e "\n"
                # [[ ${tty_dev} -ne 0  ]] && eval "${@}" >&3 2>&3 ; wait || eval "${@}" ; wait
                if [[ ${tty_dev} -ne 0 ]]; then
                    eval "${@}" >&3 2>&3 ; wait
                else
                    eval "${@}" ; wait
                fi
                echo -e "${functions["$1_success"]}"
            else
                echo -e "\n"
            fi
        else
            if [[ ${#functions[$1]} -ne 0 ]]; then
                echo -e "${functions[$1]} :\n"
            fi
            echo -e "Executing command '${*}' in local shell unsafe mode:\n"
            # [[ ${tty_dev} -ne 0  ]] && eval "${@}" >&3 2>&3 ; wait || eval "${@}" ; wait
            if [[ ${tty_dev} -ne 0 ]]; then
                eval "${@}" >&3 2>&3 ; wait
            else
                eval "${@}" ; wait
            fi
            echo -e "${functions["$1_success"]}"
            echo -e "\n"
        fi
    else
        return 0
    fi
}

# Installing essential dependencies for build script
if [[ "$(which sed)" == *"/bin/which: no sed in"* ]]; then
    echo -e "Will install essential package 'sed' for providing 'sed' tool\n"
    sudo dnf install -y sed
fi
if [[ "$(which grep)" == *"/bin/which: no grep in"* ]]; then
    echo -e "Will install essential package 'grep' for providing 'grep' tool\n"
    sudo dnf install -y grep
fi
if [[ "$(which pcregrep)" == *"/bin/which: no pcregrep in"* ]]; then
    echo -e "Will install essential package 'pcre-tools' for providing 'pcregrep' tool\n"
    sudo dnf install -y pcre-tools
fi

# Global variables

# Dockerfile to build Docker container image, create container and extract rootfs to build initrd initramfs ramdisk for EIF image
declare dockerfile="";

# Flag for marking dockerfile building with networking support and networking tools.
# Then build enclave image (EIF) with networking abilities (with forward and reverse port forwarding proxies).
# Then run forward and reverse port forwarding proxies on a host as well, with running enclave.
# Activate reverse port forwarding proxy
declare reverse_network=0;
# Activate forward port forwarding proxy
declare forward_network=0;
# Activate both, forward and reverse port forwarding proxies
declare network=0;

# Subdirectory with EIF image built with particular 'init' system (written in C, Go, Rust)
declare eif_init='init_c_eif/';

# Verbose messages mode for debugging
declare debug=0;
# Ask a question before execution of any command
declare question=0;
# Evaluate and execute local shell commands as well in current shell
declare local_shell=0;
# Should exit after command execution through CLI argument
declare should_exit=0;
# TTY allocation for build script IO
declare tty_dev=0;

# CLI arguments & flags parser

# Declare an associative array for options and a regular indexed array for positional arguments
declare -A args=()
declare -a posargs=()

# Variable to track the current option being processed
declare prev_arg=""

for arg in "$@"; do
    if [[ "$arg" == "--"* ]] || [[ "$arg" == "-"* ]]; then
        # If previous option exists, mark it as a flag (no value)
        if [[ -n "$prev_arg" ]]; then
            args["$prev_arg"]=1 # Flag presense value
        fi
        prev_arg="$arg"
    else
        # If we were expecting an option value
        if [[ -n "$prev_arg" ]]; then
            args["$prev_arg"]="$arg"
            prev_arg=""
        else
            # This is a positional argument
            posargs+=("$arg")
        fi
    fi
done

# Handle the last option if it was a flag
if [[ -n "$prev_arg" ]]; then
    args["$prev_arg"]=1 # Flag presense value
fi

# Output parsed arguments for debugging
if [[ ${args["--debug"]} -eq 1 || ${args["--verbose"]} -eq 1 || ${args["-v"]} -eq 1 ]]; then
    echo -e "Parsed options:"
    for key in "${!args[@]}"; do
        echo -e "  $key = ${args[$key]}"
    done
    echo -e "\nPositional arguments:"
    printf "  '%s'\n" "${posargs[@]}"
fi

# Override default variables values, provide dockerfile, execute commands
for key in "${!args[@]}"; do
    if [[ ${args["--debug"]} -eq 1 || ${args["--verbose"]} -eq 1 || ${args["-v"]} -eq 1 ]]; then
        echo -e "\nArg:\n$key = ${args[$key]}\n"
    fi

    case "$key" in
        "-?" | "-h" | "--help") # Print help
            runner_fn help
            exit 0
            ;;
        "-??" | "-hh" | "-he" | "--helpext" | "--help-ext" | "--help_ext") # Print extended help
            runner_fn help_ext
            exit 0
            ;;
        "--man" | "-???" | "-hhh" | "--helpextman" | "--help-ext-man" | "--help_ext_man") # Print extended help with man messages/strings
            runner_fn help_ext_man
            exit 0
            ;;
        "--debug" | "-v" | "--verbose") # Verbose messages mode for debugging
            debug=1
            ;;
        "--question" | "--questions" | "-q")  # Ask a question before execution of any command
            question=1
            ;;
        "--local-shell" | "--local_shell" | "--lsh" | "-lsh") # Evaluate and execute local shell commands as well in current shell
            local_shell=1
            ;;
        "--tty" | "--tty-dev" | "--tty_dev" | "--terminal" | "--term") # TTY allocation for build script IO
            tty_dev=1
            ;;
        "--kernel" | "--kernel-version" | "--kernel_version" |"-k") # Linux kernel full version
            if [[ -n "${args[$key]}" ]]; then
                kversion_full="${args[$key]:-'6.12.0'}"
                # Version validation
                kversion="$(echo -E "${kversion_full}" | grep -iP '^(0|[1-9][0-9]*)(\.)(0|[1-9][0-9]*)(\.([1-9][0-9]*|0))?$')"
                kversion="${kversion:-'6.12.0'}"
                # Archival kernel version extraction
                kversion_archive="$(echo -E "${kversion}" | grep -iPo '^(0|[1-9][0-9]*)(\.)(0|[1-9][0-9]*)(\.[1-9][0-9]*|(?=\.0))?')"
            else
                echo -e "Kernel full version should be non-empty\n"
            fi
            ;;
        "--user" | "--kbuild-user" | "--kbuild_user" | "--kuser" | "-u") # Username for Linux kernel build
            if [[ -n "${args[$key]}" ]]; then
                kbuild_user="${args[$key]:-'sentient_build'}"
            else
                echo -e "Username for Linux kernel build should be non-empty\n"
            fi
            ;;
        "--host" | "--kbuild-host" | "--kbuild_host" | "--khost" | "-h") # Hostname for Linux kernel build
            if [[ -n "${args[$key]}" ]]; then
                kbuild_host="${args[$key]:-'sentient_builder'}"
            else
                echo -e "Hostname for Linux kernel build should be non-empty\n"
            fi
            ;;
        "--memory" | "--mem" | "--ram" | "-m" | "--enclave-memory" | "--enclave_memory" | "--enclave-mem" | "--enclave_mem") # Enclave run-time memory allocation size in MiBs
            if [[ -n "${args[$key]}" ]]; then
                enclave_mem="${args[$key]:-'838656'}"
            else
                echo -e "Enclave run-time memory allocation size in MiBs should be non-empty\n"
            fi
            ;;
        "--cpus" | "--cpu" | "--cores" | "--cpu-cores" | "--cpu_cores" | "--enclave-cpus" | "--enclave_cpus") # Number of CPU cores allocation for enclave's run-time
            if [[ -n "${args[$key]}" ]]; then
                enclave_cpus="${args[$key]:-'64'}"
            else
                echo -e "Number of CPU cores allocation for enclave's run-time should be non-empty\n"
            fi
            ;;
        "--cid" | "--enclave-cid" | "--enclave_cid") # Enclave's VSock CID for SLC data connection
            if [[ -n "${args[$key]}" ]]; then
                enclave_cid="${args[$key]:-'127'}"
            else
                echo -e "Enclave's VSock CID for SLC data connection should be non-empty\n"
            fi
            ;;
        "--dockerfile" | "-d") # Build EIF image from Docker container extracted rootfs, created from Docker image, formed by dockerfile scenario
            if [[ -n "${args[$key]}" ]]; then
                dockerfile=$(echo -E "${args[$key]}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*([^\s]*?)(\.dockerfile)$");
                if [[ -z ${dockerfile} ]]; then
                    dockerfile=$(echo -E "${dockerfile:-"./pipeline-al2023.dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*([^\s]*?)(\.dockerfile)$");
                fi
            else
                echo -e "Dockerfile name and path should be provided along with the '--dockerfile|-d' argument\n"
            fi
            ;;
        # Flag for marking dockerfile building with networking support and networking tools.
        # Then build enclave image (EIF) with networking abilities (with forward and reverse port forwarding proxies).
        # Then run forward and reverse port forwarding proxies on a host as well, with running enclave.
        # Activate reverse port forwarding proxy
        "--revnet" | "--rev_net" | "--rev-net" | "--rev_network" | "--rev-network" | "--reverse_net" | "--reverse-net" | "--reverse_network" | "--reverse-network" | "-rn")
            reverse_network=1
            ;;
        # Activate forward port forwarding proxy
        "--fwnet" | "--fw_net" | "--fw-net" | "--fw_network" | "--fw-network" | "--forward_net" | "--forward-net" | "--forward_network" | "--forward-network" | "-fn")
            forward_network=1
            ;;
        # Activate both, forward and reverse port forwarding proxies
        "--net" | "--network" | "--networking" | "-n")
            network=1
            ;;
        "--init-c" | "--init_c" | "--clang") # Build EIF image with init.c as init system and run enclave from this EIF image
            eif_init='init_c_eif/';
            ;;
        "--init-go" | "--init_go" | "--golang" | "--go") # Build EIF image with init.go as init system and run enclave from this EIF image
            eif_init='init_go_eif/';
            ;;
        # Build EIF image with init.rs as init system and run enclave from this EIF image
        "--init-rs" | "--init_rs" | "--init-rust" | "--init_rust" | "--rust" | "--rs")
            eif_init='init_rs_eif/';
            ;;
        "--cmd" | "--command" | "-c") # Execute command (can be pointed multiple times for several commands execution sequentially)
            if [[ -n "${args[$key]}" ]]; then
                runner_fn "${args[$key]}"
                should_exit=1
            else
                echo -e "Command should be non-empty\n"
            fi
            ;;
        *)
            echo -e "Argument/parameter/flag $key isn't supported\n"
            ;;
    esac
done

# Use of positional parameters
for key in "${!posargs[@]}"; do
    if [[ ${args["--debug"]} -eq 1 || ${args["--verbose"]} -eq 1 || ${args["-v"]} -eq 1 ]]; then
        echo -e "\nPosArg:\n$key = ${posargs[$key]}\n"
    fi

    case "${posargs[$key]}" in
        *.dockerfile) # Build EIF image from Docker container extracted rootfs, created from Docker image, formed by dockerfile scenario
            dockerfile=$(echo -E "${posargs[$key]}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*([^\s]*?)(\.dockerfile)$");
            if [[ -z ${dockerfile} ]]; then
                dockerfile=$(echo -E "${dockerfile:-"./pipeline-al2023.dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*([^\s]*?)(\.dockerfile)$");
            fi
            ;;
        *)
            echo -e "Positional argument/parameter ${posargs[$key]} isn't supported\n"
            ;;
    esac
done

# TTY device allocation for IO
if [[ ${tty_dev} -ne 0 ]]; then
    # Define the TTY device (adjust it as needed)
    # declare TTY_DEVICE="/dev/pts/0"
    declare TTY_DEVICE="/dev/tty"

    # Ensure script is running with a TTY
    if [ ! -t 0 ]; then
        exec < "$TTY_DEVICE"
    fi
    if [ ! -t 1 ]; then
        exec > "$TTY_DEVICE"
    fi
    if [ ! -t 2 ]; then
        exec 2> "$TTY_DEVICE"
    fi

    # Open the TTY device for reading and writing
    exec 3<> "$TTY_DEVICE"

    # Set the TTY device as the script's input/output
    exec <&3
    exec >&3
fi

if [[ ${should_exit} -ne 0 ]]; then
    exit 0
fi

# Command execution cycle and parser of commands from external command list sent by stdin pipe
while true; do
    if [[ ${tty_dev} -ne 0 ]]; then
        read -p "$(whoami | tr -d '\n')@$(hostname -s | tr -d '\n'):$(pwd | tr -d '\n') $( [[ "$(whoami | tr -d '\n')" == "root" ]] && echo -e "#" || echo -e "\$" )> " cmd <&3
    else
        read -p "$(whoami | tr -d '\n')@$(hostname -s | tr -d '\n'):$(pwd | tr -d '\n') $( [[ "$(whoami | tr -d '\n')" == "root" ]] && echo -e "#" || echo -e "\$" )> " cmd
    fi

    # Type 'break' or 'exit', or push 'Ctrl+C' key sequence to exit from this shell
    if [[ $cmd == "break" || $cmd == "exit" ]]; then
        break
    fi

    # Print the filename of the terminal connected/attached to the standard input (to this shell)
    if [[ $cmd == "tty" ]]; then
        tty ;
        continue
    fi

    # Debug mode
    if [[ $cmd == "debug" ]]; then
        # debug=$(( ! $debug ))
        debug=$(( 1 - $debug ))
        echo "debug == $debug"
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

    # Trigger for marking dockerfile building with networking support and networking tools.
    # Then build enclave image (EIF) with networking abilities (with forward and reverse port forwarding proxies).
    # Then run forward and reverse port forwarding proxies on a host as well, with running enclave.

    # Activate reverse port forwarding proxy
    if [[ $cmd == "reverse_network" ]]; then
        # reverse_network=$(( ! $reverse_network ))
        reverse_network=$(( 1 - $reverse_network ))
        echo "reverse_network == $reverse_network"
        continue
    fi

    # Activate forward port forwarding proxy
    if [[ $cmd == "forward_network" ]]; then
        # forward_network=$(( ! $forward_network ))
        forward_network=$(( 1 - $forward_network ))
        echo "forward_network == $forward_network"
        continue
    fi

    # Activate both, forward and reverse port forwarding proxies
    if [[ $cmd == "network" ]]; then
        # network=$(( ! $network ))
        network=$(( 1 - $network ))
        echo "network == $network"
        continue
    fi

    # Setup Nitro Enclaves into system, automated guide
    if [[ $cmd == "make nitro" ]]; then

        runner_fn make_nitro

        continue
    fi

    # Kernel build automated guide
    if [[ $cmd == "make kernel" ]]; then

        runner_fn make_kernel

        continue
    fi

    # Build automated guide for enclave's run-time Rust apps (SSE Framework) and for enclave's image (EIF) building tools
    if [[ $cmd == "make apps" ]]; then

        runner_fn make_apps

        continue
    fi

    # Init system build automated guide
    if [[ $cmd == "make init" ]]; then

        runner_fn make_init

        continue
    fi

    # EIF enclave image build automated guide
    if [[ $cmd == "make eif" || $cmd == "make eif"*".dockerfile" ]]; then

        dockerfile=$(echo -E "${cmd}" | sed -E "s/((make\s?)|(make\s?eif\s?))//gI" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*([^\s]*?)(\.dockerfile)$");
        if [[ -z ${dockerfile} ]]; then
            dockerfile=$(echo -E "${dockerfile:-"./pipeline-al2023.dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*([^\s]*?)(\.dockerfile)$");
        fi

        runner_fn make_eif "${dockerfile}"

        continue
    fi

    # Enclave run-time management automated guide:
    # run enclave image file (EIF), connect/attach local terminal to enclave's console output, list running enclaves, terminate enclaves.
    if [[ $cmd == "make enclave" ]]; then

        runner_fn make_enclave

        continue
    fi

    # Automatically setup, build, deploy and run all stack components in unattended mode
    if [[ $cmd == "make" || $cmd == "make all" || $cmd == "make all"*".dockerfile" || $cmd == "make ="*".dockerfile" || $cmd == "make :="*".dockerfile" ]]; then

        dockerfile=$(echo -E "${cmd}" | sed -E "s/((make\s?)|(make\s?all\s?)|(make\s?=\s?)|(make\s?:=\s?))//gI" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*([^\s]*?)(\.dockerfile)$");
        if [[ -z ${dockerfile} ]]; then
            dockerfile=$(echo -E "${dockerfile:-"./pipeline-al2023.dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*([^\s]*?)(\.dockerfile)$");
        fi

        runner_fn make_all "${dockerfile}"

        continue
    fi

    # Automatically clear all Docker containers and all Docker images
    # created during automated unattended installation process of setup, build, deploy and run all Secure Enclaves Framework stack components
    if [[ $cmd == "make clear" || $cmd == "make clear"*".dockerfile" ]]; then

        dockerfile=$(echo -E "${cmd}" | sed -E "s/((make\s?)|(make\s?clear\s?))//gI" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*([^\s]*?)(\.dockerfile)$");
        if [[ -z ${dockerfile} ]]; then
            dockerfile=$(echo -E "${dockerfile:-"./pipeline-al2023.dockerfile"}" | pcregrep --color -Mio -e "^(\.\/)?([^\s]*?\/)*([^\s]*?)(\.dockerfile)$");
        fi

        runner_fn make_clear "${dockerfile}"

        continue
    fi

    if [[ ${#cmd} -ne 0 ]]; then
        runner_fn $cmd ; wait ; continue
    else
        continue
    fi

done

if [[ ${tty_dev} -ne 0 ]]; then
# Close the TTY device
    exec 3>&-
fi

exit 0

