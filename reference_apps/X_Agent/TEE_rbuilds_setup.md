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
An application can be run inside enclave in either `debug` mode or `non-debug` mode.  `non-debug` mode is recommended for production purposes.

### `debug` mode

> [!NOTE]
> [`debug`](https://docs.aws.amazon.com/enclaves/latest/user/cmd-nitro-run-enclave.html#cmd-nitro-run-enclave-options) mode allows for attaching to the enclave's console and observing the application's logs. This mode is recommended for development purposes.
> This mode cannot generate `PCR` values needed for base-image attestation.

```bash
sudo ./rbuilds.sh --mem 50000 --cpus 10 --dockerfile "/home/ec2-user/sentient-enclaves-framework/reference_apps/X_Agent/x_agent.dockerfile" --fw_network --init-c  --cmd "run_eif_image_debugmode_cli"
```

### `non-debug` mode

> [!NOTE]
> `non-debug` mode is recommended for production purposes.
> This mode provides access to `PCR` values needed for base-image attestation.

```bash
sudo ./rbuilds.sh --mem 50000 --cpus 10 --dockerfile "/home/ec2-user/sentient-enclaves-framework/reference_apps/X_Agent/x_agent.dockerfile" --fw_network --init-c  --cmd "run_eif_image"
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
> Either [`networking.sh`](../../rbuilds/network.init/networking.sh) can be used for accessing enclave's shell interface for issuing commands or [`pipeline`](../../rbuilds/network.init/pipeline) utility to send commands inside enclave. Both ways are demonstrated. While in `non-debug` mode, `pipeline` is the recommended way to send commands inside enclave.


## Verifying X_Agent Integrity within the Enclave 🛡️
> [!IMPORTANT]
> For the Agent to be attestable, it must be running in `non-debug` mode. 🔒

### Confirming External Data Integrity 📄
Applications inside enclaves might need external configuration or inputs (like a `.env` file). To ensure this external information hasn't been tampered with, it requires verification. The `ra-web-srv` service facilitates this using hash validation and Verifiable Random Function (VRF) proof generation.

The RA server starts automatically upon enclave initialization. Follow these steps to verify the `.env` file:

#### 1. Generate Hash & VRF Proofs 🔑
Use the `/generate` endpoint. This command processes the specified file `(./X_Agent/.env)`:

```bash
# Instructs the enclave's RA server to generate proofs for the .env file
./pipeline run --port 53000 --cid 127 --command "curl -s -i -k -X POST -H 'Content-Type: application/json' -d '{ \"path\": \"./X_Agent/.env\" }' https://127.0.0.1:8443/generate"
```

#### 2. Retrieve the Attestation Proof 📜
> [!NOTE]
> Check that the `proof` is generated before requesting the attestation using `/ready` endpoint.

Fetch the generated proof using the `/proof` endpoint:
```bash
./pipeline run --port 53000 --cid 127 --command "curl -s -i -k -X GET https://127.0.0.1:8443/proof/?path=./X_Agent/.env"
```
Expect a response similar to this:
```json
{
    "hash":"bc091981667d727d44807e0fc052e4667e517e12b16e083b1db6ea78b9e9341611ec599f56ffa77c5d48bdca7ad137833ddb0263dd6039b150a610db7919da1d",
    "path":"./X_Agent/.env",
    "proof":"0272255a5baa03f627226870fdb14ec409585d0ceefd05ddc06c669c52fc869095168034dad96fe97740b192211bf1f1477a1b7e15880b31bee8b87f8c7cb0d5a28d0476376d368b46b9d1bd29da02ec21"
}
```

#### 3. Validate the Hash ✅
The server uses SHA-512 for hashing. Compare the hash from the response with a locally computed hash of the original file:

```bash
sha3sum -a 512 ~/reference_apps/X_Agent/.env
``` 

Local command output:
```bash
bc091981667d727d44807e0fc052e4667e517e12b16e083b1db6ea78b9e9341611ec599f56ffa77c5d48bdca7ad137833ddb0263dd6039b150a610db7919da1d  *./X_Agent/.env
```

As the hashes match, the file's integrity is confirmed. 🎉

> [!NOTE]
> VRF proof verification will be done by verifying the VRF proof using the `verify` endpoint. For more details, consult to the [ATTESTATION API SPECIFICATION](../../docs/md/ATTESTATION_WEB_API.md) documentation.


### Validating the Application's Base Image 📦
Attestation of application's base image on nitro enclaves is done by matching the `PCR` values of the running enclave with the `PCR` values for the base `eif` file. `PCR0`, `PCR1`, `PCR2` are the `PCR`'s of interest for [attestation](https://docs.aws.amazon.com/enclaves/latest/user/set-up-attestation.html).

#### 1. Request the Enclave's Attestation Document 📄
Use the `/doc` endpoint to get the running enclave's measurements, including PCRs:


```bash
./pipeline run --port 53000 --cid 127 --command "curl -s -i -k -X GET https://127.0.0.1:8443/doc/?path=./X_Agent/x_agent.eif&view=json_hex"
```

The response contains PCR values (truncated example):

```json
{
    // ... other fields ...
    "PCRs":{
        "0":"bc0acfdeaa10d267ede8681f50b3b800336a3b585d016d4a3990d0baa8dfe9545498ef9ded1af24136f2929f1602554a",
        "1":"bd78456d3ac7bce218c532a1882cff7f7e76e28c8d898eed888cfbb44ee97bd3f27c7fbae6d52bce4205595779f40c59",
        "2":"6a32eb123d5bd397a2289cbc0b89d8c4d701a5e915d4f3b8ad75d746c6b72989ad18d5d56990e4a20355926ab87701c1",
        // ... other PCRs ...
    }
    // ... other fields ...
}
```
> [!IMPORTANT]
> The response above is a truncated version of the actual response. It is manually formatted for better readability. Actual response is a JSON object with all the fields and `\` as separators.

#### 2. Compare PCR Values 🔍
Verify the PCRs from the attestation document against the known-good measurements of original Enclave Image File `.eif`. Obtain the reference PCRs using `nitro-cli`:
```bash
# Describes the original EIF file to get its expected PCR measurements
nitro-cli describe-eif --eif-path ../eif/init_c_eif/app-builder-secure-enclaves-framework.eif
```
Expected output from `nitro-cli`:
```json
{
  "EifVersion": 4,
  "Measurements": {
    "HashAlgorithm": "Sha384 { ... }",
    "PCR0": "bc0acfdeaa10d267ede8681f50b3b800336a3b585d016d4a3990d0baa8dfe9545498ef9ded1af24136f2929f1602554a",
    "PCR1": "bd78456d3ac7bce218c532a1882cff7f7e76e28c8d898eed888cfbb44ee97bd3f27c7fbae6d52bce4205595779f40c59",
    "PCR2": "6a32eb123d5bd397a2289cbc0b89d8c4d701a5e915d4f3b8ad75d746c6b72989ad18d5d56990e4a20355926ab87701c1"
  },
  // ... other fields ...
}
```
Matching `PCR0`, `PCR1`, and `PCR2` values confirm that the base image running in the enclave is unmodified and trustworthy. 🎉


## Running the X Agent 🚀
After attesting the base image and the external data, the agent can be run inside the enclave.
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