# Setting up the X Agent 🚀

## Building the X Agent EIF 🛠️
```bash
sudo ./rbuilds.sh --cmd "make_eif" --dockerfile "/home/ec2-user/pipeline/secure-enclaves-framework/reference_apps/X_Agent/x_agent.dockerfile" --network --init-c
```

## Running the X Agent in the enclave 🌟
```bash
sudo ./rbuilds.sh --cmd "run_eif_image_debugmode_cli" --mem 50000 --cpus 10 --dockerfile "/home/ec2-user/pipeline/secure-enclaves-framework/reference_apps/X_Agent/x_agent.dockerfile" --network --init-c
```

## Passing the `.env` file 🔑
```bash
./pipeline send-file --port 53000 --cid 127 --localpath <path_to_env_file_directory>/.env --remotepath /apps/X_Agent/.env
```
Example:
```bash
./pipeline send-file --port 53000 --cid 127 --localpath ~/reference_apps/X_Agent/.env --remotepath /apps/X_Agent/.env
```

## Running the X Agent 🚀

```bash
cd /apps/X_Agent && ./.venv/bin/python3 -u agent.py --username DobbyReborn 2>&1 | tee agent.log
```

## Stopping the X Agent 🛑

### Finding the PID of the Agent
```bash
ps -aux | grep agent.py 
```

### Killing the Agent
```bash
kill -9 <pid>
```

## Dropping the recent enclave running Agent 🗑️
```bash
./rbuilds.sh --cmd "drop_recent_enclave"
```