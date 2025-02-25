# Setting up the X Agent 🚀

## Building the X Agent EIF 🛠️
```bash
sudo ./rbuilds.sh --dockerfile "/home/ec2-user/sentient-enclaves-framework/reference_apps/X_Agent/x_agent.dockerfile" --fw_network --init-c --cmd "make_eif"
```
> [!NOTE]
> 1. Make sure `--cmd` is last argument when it depends on values passed on command line to the `rbuilds.sh` script.
> 2. This agent requires no external inputs/instructions as it's executed. If it's desired to support that functionality either pass `--network` flag instead of `--fw_network` or pass `--rev_network` flag with `--fw_network`. Take a look at other [reference applications](../) to setup a 2-way proxy.
> 3. Make sure the dockerfile path is correct and accessible to `rbuilds.sh`.

## Running the enclave 🌟
```bash
sudo ./rbuilds.sh --mem 50000 --cpus 10 --dockerfile "/home/ec2-user/sentient-enclaves-framework/reference_apps/X_Agent/x_agent.dockerfile" --fw_network --init-c  --cmd "run_eif_image_debugmode_cli"
```

## Passing the `.env` file 🔑
Issue the following command to pass the `.env` file to the X Agent in the enclave in [network.init](../../rbuilds/network.init) folder. `wget` can also be used to download the file from a remote server.
```bash
./pipeline send-file --port 53000 --cid 127 --localpath <path_to_env_file_directory>/.env --remotepath /apps/X_Agent/.env
```
Example:
```bash
./pipeline send-file --port 53000 --cid 127 --localpath ~/reference_apps/X_Agent/.env --remotepath /apps/X_Agent/.env
```

---
> [!IMPORTANT]
> Either [`networking.sh`](../../rbuilds/network.init/networking.sh) can be used for accessing enclave's shell interface for issuing commands or [`pipeline`](../../rbuilds/network.init/pipeline) utility to send commands inside enclave. Both ways are demonstrated.

## Running the X Agent 🚀
> [!NOTE]
> Use command chaining to execute commands in folders other than the `$HOME` directory.
* Using `networking.sh` to get access to enclave's shell:
    ```bash
    cd /apps/X_Agent && ./.venv/bin/python3 -u agent.py --username DobbyReborn 2>&1 | tee agent.log
    ```
* Using `pipeline` utility
    ```bash
    ./pipeline run --port 53000 --cid 127 --no-wait --command "cd /apps/X_Agent && ./.venv/bin/python3 -u agent.py --username DobbyReborn 2>&1 | tee agent.log"
    ```

## Stopping the X Agent 🛑
Kill the agent using `kill` command in enclave.
* Using `networking.sh`
    ```bash
    kill -9 $(ps aux | grep "[.]/\.venv/bin/python3 -u agent\.py" | awk '{print $2}')
    ```
* Using `pipeline` utility:
    ```bash
    ./pipeline run --port 53000 --cid 127 --no-wait --command "kill -9 \$(ps aux | grep '[.]/\.venv/bin/python3 -u agent\.py' | awk '{print \$2}')"
    ```
> [!NOTE]
> Here `$` needs to be escaped correctly when `pipeline` is used.

## Dropping the recent enclave running Agent
```bash
./rbuilds.sh --cmd "drop_recent_enclave"
```