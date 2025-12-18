## **Usage:**

`rbuilds.sh` script itself should be run from project root directory, to form FS directories properly.

`rbuilds.sh` script options can be used like this:
```bash
./rbuilds.sh --tty --debug --cmd "make_nitro" 2>&1 3>&1

./rbuilds.sh --tty --debug --cmd "make_clear" 2>&1 3>&1
```

*Note:* `3>&1` needed for `--tty` flag, as it enables terminal `tty` device with file descriptor `3` for `bash` and `docker` output, 'cause it is executed not in local shell.

For building all stages, run the command:
```bash
mkdir -vp ./eif/ ; /usr/bin/time -v -o ./eif/make_build.log ./rbuilds.sh --tty --debug --dockerfile ./pipeline-slc-network-al2023.dockerfile --network --init-c --cmd "make_all" 2>&1 3>&1 | tee ./eif/make_build.output
```

For different build stages the template command will be the same (as most of this commands use internally same set of variables, set/override by CLI options):
```bash
./rbuilds.sh --tty --debug --dockerfile ./pipeline-slc-network-al2023.dockerfile --network --init-c --cmd "make_kernel" 2>&1 3>&1 

./rbuilds.sh --tty --debug --dockerfile ./pipeline-slc-network-al2023.dockerfile --network --init-c --cmd "make_apps" 2>&1 3>&1 

./rbuilds.sh --tty --debug --dockerfile ./pipeline-slc-network-al2023.dockerfile --network --init-c --cmd "make_init" 2>&1 3>&1 

./rbuilds.sh --tty --debug --dockerfile ./pipeline-slc-network-al2023.dockerfile --network --init-c --cmd "make_eif" 2>&1 3>&1 

./rbuilds.sh --tty --debug --dockerfile ./pipeline-slc-network-al2023.dockerfile --network --init-c --cmd "make_enclave" 2>&1 3>&1
```

For managing enclave's state, run (one of these) commands:
```bash
./rbuilds.sh --tty --debug --network --init-c --cmd "run_eif_image_debugmode_cli" 2>&1 3>&1

./rbuilds.sh --tty --debug --network --init-c --cmd "run_eif_image" 2>&1 3>&1

./rbuilds.sh --tty --debug --network --init-c --cmd "attach_console_to_enclave" 2>&1 3>&1

./rbuilds.sh --tty --debug --network --init-c --cmd "list_enclaves" 2>&1 3>&1

./rbuilds.sh --tty --debug --network --init-c --cmd "drop_enclave" 2>&1 3>&1

./rbuilds.sh --tty --debug --network --init-c --cmd "drop_enclaves_all" 2>&1 3>&1
```

For more information about CLI options, interactive shell mode and automation command shell interface, run these usage help commands:
```bash
./rbuilds.sh --man

./rbuilds.sh --info
```

## **Advanced usage:**

Also `rbuild.sh` supports an interactive shell mode, for interactive builds and commands shell interface mode for interactive automation of builds (and build stages):

### Interactive mode shell with timings and console dump - you'll see the shell prompt:
```bash
mkdir -vp ./eif/;
/usr/bin/time -v -o ./eif/make_build.log ./rbuild.sh 2>&1 | tee ./eif/make_build.output
```

### Automation command shell interface:
```bash
{ echo "attach_console_to_enclave"; } | ./rbuild.sh 2>&1
{ echo "list_enclaves"; } | ./rbuild.sh 2>&1
{ echo "drop_enclave"; } | ./rbuild.sh 2>&1
{ echo "drop_enclaves_all"; } | ./rbuild.sh 2>&1
```

### Automation command shell interface with timings and console dump:
```bash
{ echo "attach_console_to_enclave"; } | /usr/bin/time -v -o ./eif/make_build.log ./rbuild.sh 2>&1 | tee ./eif/make_build.output
{ echo "list_enclaves"; } | /usr/bin/time -v -o ./eif/make_build.log ./rbuild.sh 2>&1 | tee ./eif/make_build.output
{ echo "drop_enclave"; } | /usr/bin/time -v -o ./eif/make_build.log ./rbuild.sh 2>&1 | tee ./eif/make_build.output
{ echo "drop_enclaves_all"; } | /usr/bin/time -v -o ./eif/make_build.log ./rbuild.sh 2>&1 | tee ./eif/make_build.output
```

### Automation command shell interface for building stages make commands:
```bash
{  echo "make eif";
  echo "y";
  echo "y";
  echo "y";
  echo "y";
  echo "y";
  echo "y";
  echo "y"; } | /usr/bin/time -v -o ./make_build.log ./rbuild.sh 2>&1 | tee ./make_build.output

{ echo "make all"; } | /usr/bin/time -v -o ./make_build.log ./rbuild.sh 2>&1 | tee ./make_build.output

mkdir -vp ./eif/ ; { echo "make all"; } | /usr/bin/time -v -o ./eif/make_build.log ./rbuild.sh 2>&1 | tee ./eif/make_build.output
```

### Automation command shell interface with access to local commands execution (use responsibly):
```bash
{ echo "lsh"; echo "ls -lah"; } | /usr/bin/time -v -o ./make_build.log ./rbuild.sh 2>&1 | tee ./make_build.output
```
