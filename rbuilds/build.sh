#!/bin/bash
##!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

declare kversion='6.12'

if [[ "$1" == "?" || "$1" == "-?" || "$1" == "h" || "$1" == "-h" || "$1" == "help" || "$1" == "--help" ]]; then
    echo -e "Simple shell to build enclave images (EIF)."
    echo -e "Input 'make' command to start building image."
    echo -e "Enter 'break' or 'exit' for exit from this shell."
    exit 0
fi

docker_kcontainer_clear() {
    docker kill kernel_build_v${kversion} ;
    docker rm --force kernel_build_v${kversion} ;
}

docker_kimage_clear() {
    # whoami; uname -a; pwd;
    docker rmi --force app-build-toolkit-al2023:v${kversion} ;
}

docker_kimage_build() {
    DOCKER_BUILDKIT=1 docker build --no-cache -f ./rust-build-toolkit-al2023.dockerfile -t "app-build-toolkit-al2023:v${kversion}" ./ ;
    # -ti
    docker create --name kernel_build_v${kversion} app-build-toolkit-al2023:v${kversion} sleep infinity; sleep 1;
    # docker create --name kernel_build_v${kversion} app-build-toolkit-al2023:v${kversion} tail -f /dev/null; sleep 1;
    # -tid
    # docker run -d --name kernel_build_v${kversion} app-build-toolkit-al2023:v${kversion} sleep infinity & disown; sleep 1;
    # docker run -d --name kernel_build_v${kversion} app-build-toolkit-al2023:v${kversion} tail -f /dev/null & disown; sleep 1;
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
    # docker cp ./kernel/artifacts_static/.config kernel_build_v${kversion}:/kbuilder/ ;
    docker cp ./kernel/artifacts_kmods/.config kernel_build_v${kversion}:/kbuilder/ ;
    docker exec -ti kernel_build_v${kversion} bash -cis -- "cp -vr /kbuilder/.config /kbuilder/linux-v${kversion}/.config" ;
}

docker_kernel_build() {
    docker exec -ti kernel_build_v${kversion} bash -cis -- "cd /kbuilder/linux-v${kversion}/; \
        mkdir -vp ./kernel_blobs; \
        mkdir -vp ./kernel_modules; \
        mkdir -vp ./kernel_headers; \
        export KBUILD_BUILD_TIMESTAMP="$(date -u '+%FT%T.%N%:z')"; \
        export KBUILD_BUILD_USER="sentient_build"; \
        export KBUILD_BUILD_HOST="sentient_builder"; \
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
    mkdir -vp ./eif_build/ ;
    cp -vr ./init_build/blobs/eif_build/eif_build ./eif_build/ ;
    cp -vr ./init_build/blobs/eif_extract/eif_extract ./eif_build/ ;
}

docker_clear() {
    docker kill pipeline_toolkit ;
    docker rm --force pipeline_toolkit ;
    docker rmi --force pipeline-al2023 ;
}

docker_build() {
    DOCKER_BUILDKIT=1 docker build --no-cache --build-arg FS=0 -f ./pipeline-al2023.dockerfile -t "pipeline-al2023" ./ ;
    docker create --name pipeline_toolkit pipeline-al2023:latest ;
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

run_eif_image() {
    /usr/bin/time -v -o ./run-enclave.log nitro-cli run-enclave --cpu-count 64 --memory 838656 --eif-path ./app-builder-pipeline.eif --debug-mode --attach-console --enclave-cid 127  --enclave-name pipeline_toolkit 2>&1 | tee app-builder-pipeline.output
}

attach_console_to_enclave() {
    ENCLAVE_ID=$(nitro-cli describe-enclaves | jq -r ".[0].EnclaveID"); \
    nitro-cli console --enclave-id "${ENCLAVE_ID}" 2>&1 | tee -a app-builder-pipeline.output
}

while true; do
    read -p "$(whoami | tr -d '\n')@$(hostname -s | tr -d '\n'):$(pwd | tr -d '\n') $( [[ "$(whoami | tr -d '\n')" == "root" ]] && echo -e "#" || echo -e "\$" )> " cmd

    if [[ $cmd == "break" || $cmd == "exit" ]]; then
        break
    fi
    if [[ $cmd == "tty" ]]; then
        tty ;
        continue
    fi

    if [[ $cmd == "docker_clear" ]]; then
        docker_clear ;
        continue
    fi
    if [[ $cmd == "docker_build" ]]; then
        docker_build ;
        continue
    fi
    if [[ $cmd == "init_and_rootfs_base_images_build" ]]; then
        init_and_rootfs_base_images_build ;
        continue
    fi
    if [[ $cmd == "docker_to_rootfs_fs_image_build" ]]; then
        docker_to_rootfs_fs_image_build ;
        continue
    fi
    if [[ $cmd == "ramdisk_image_build" ]]; then
        ramdisk_image_build ;
        continue
    fi
    if [[ $cmd == "eif_build_with_initc" ]]; then
        eif_build_with_initc ;
        continue
    fi
    if [[ $cmd == "eif_build_with_initgo" ]]; then
        eif_build_with_initgo ;
        continue
    fi
    if [[ $cmd == "run_eif_image" ]]; then
        run_eif_image ;
        continue
    fi
    if [[ $cmd == "attach_console_to_enclave" ]]; then
        attach_console_to_enclave ;
        continue
    fi

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

        read -n 1 -s -p "Run EIF image in enclave (Nitro KVM based VM)? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            run_eif_image ; wait
        else
            echo -e "\n"
        fi

        read -n 1 -s -p "Attach local console to enclave's debug dump (stdout)? [y|n] : " choice
        if [[ $choice == "y" ]]; then
            echo -e "\n"
            attach_console_to_enclave ; wait
        else
            echo -e "\n"
        fi

        continue
    fi

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
        else
            echo -e "\n"
        fi

        echo -e "\nMaking of a kernel successfully done!\n"

        continue
    fi

    if [[ $cmd == "docker_kcontainer_clear" ]]; then
        docker_kcontainer_clear ;
        continue
    fi
    if [[ $cmd == "docker_kimage_clear" ]]; then
        docker_kimage_clear ;
        continue
    fi
    if [[ $cmd == "docker_kimage_build" ]]; then
        docker_kimage_build ;
        continue
    fi
    if [[ $cmd == "docker_prepare_kbuildenv" ]]; then
        docker_prepare_kbuildenv ;
        continue
    fi
    if [[ $cmd == "docker_kernel_build" ]]; then
        docker_kernel_build ;
        continue
    fi

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

    if [[ $cmd == "docker_init_clear" ]]; then
        docker_init_clear ;
        continue
    fi
    if [[ $cmd == "docker_init_build" ]]; then
        docker_init_build ;
        continue
    fi

done

