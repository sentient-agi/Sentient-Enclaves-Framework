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


## Attesting the X Agent 🛡️
> [!WARNING]
> For the Agent to be attested, it must be running in `non-debug` mode.

### Attesting the external data
When running applications inside enclave, it's possible to pass external data to the enclave. For verifying the integrity of the data, the data should be attested. `ra-web-srv` provides capabilities to attest the data using hash validation and VRF proof generation. Here we are attesting the `.env` file.
The RA server would be automatically started when the enclave is initialized. To request the attestation for the data, following steps should be followed:

#### Generate the hash and VRF proofs for the data using the `/generate` endpoint.
The command below will generate the hash and VRF proofs for all the data in the `X_Agent` directory.

```bash
./pipeline run --port 53000 --cid 127 --command "curl -s -i -k -X POST -H 'Content-Type: application/json' -d '{ \"path\": \"./X_Agent/.env\" }' https://127.0.0.1:8443/generate"
```

#### Request the attestation for the data using the `/proof` endpoint.
> [!NOTE]
> Check that the `proof` is generated before requesting the attestation using `/ready` endpoint.

```bash
./pipeline run --port 53000 --cid 127 --command "curl -s -i -k -X GET https://127.0.0.1:8443/proof/?path=./X_Agent/.env"
```
We get the following data in response:
```json
{
    "hash":"bc091981667d727d44807e0fc052e4667e517e12b16e083b1db6ea78b9e9341611ec599f56ffa77c5d48bdca7ad137833ddb0263dd6039b150a610db7919da1d",
    "path":"./X_Agent/.env",
    "proof":"0272255a5baa03f627226870fdb14ec409585d0ceefd05ddc06c669c52fc869095168034dad96fe97740b192211bf1f1477a1b7e15880b31bee8b87f8c7cb0d5a28d0476376d368b46b9d1bd29da02ec21"
}
```

#### Verifying the hash
The hash generator in the server uses SHA-512 to generate the hash of the data. We can verify the file integrity by comparing the hash in the response with the hash of the file on the host.

```bash
sha3sum -a 512 ~/reference_apps/X_Agent/.env
``` 

This returns the following output:
```bash
bc091981667d727d44807e0fc052e4667e517e12b16e083b1db6ea78b9e9341611ec599f56ffa77c5d48bdca7ad137833ddb0263dd6039b150a610db7919da1d  *./X_Agent/.env
```

We can see that the hash in the response matches the hash in the output proving the file integrity. 
> [!NOTE]
> VRF proof verification will be done by verifying the VRF proof using the `verify` endpoint. For more details, please refer to the [ATTESTATION API SPECIFICATION](../../docs/md/ATTESTATION_WEB_API.md) documentation.


### Attesting the Application's base image
Attestation of application's base image on nitro enclaves is done by matching the `PCR` values of the running enclave with the `PCR` values for the base `eif` file. `PCR0`, `PCR1`, `PCR2` are the `PCR`'s of interest for [attestation](https://docs.aws.amazon.com/enclaves/latest/user/set-up-attestation.html).

#### Request the attestation for the base image 
To request the attestation for the base image, `/doc` endpoint is used. 

```bash
./pipeline run --port 53000 --cid 127 --command "curl -s -i -k -X GET https://127.0.0.1:8443/doc/?path=./X_Agent/x_agent.eif&view=json_hex"
```

This returns the following data in response:
```json
{
    // Other fields...
    "PCRs":{
        "0":"bc0acfdeaa10d267ede8681f50b3b800336a3b585d016d4a3990d0baa8dfe9545498ef9ded1af24136f2929f1602554a",
        "1":"bd78456d3ac7bce218c532a1882cff7f7e76e28c8d898eed888cfbb44ee97bd3f27c7fbae6d52bce4205595779f40c59",
        "2":"6a32eb123d5bd397a2289cbc0b89d8c4d701a5e915d4f3b8ad75d746c6b72989ad18d5d56990e4a20355926ab87701c1",
        "3":"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "4":"13aea116f07fd29b71bbfb05f8374bf3c3c2149d941d105909ab20178230d46d240022e36e0ed8a0d619c4c4a554423a",
        // Other PCR values...
    }
    // Other fields...
}
```
> [!NOTE]
> The response above is a truncated version of the actual response. It is formatted for better readability. Actual response is a JSON object with all the fields and `\` as separators.

#### Verifying the PCR values
The PCR values are verified by comparing the `PCR` values in the response with the `PCR` values for the base `eif` file. To obtain the `PCR` values for the base `eif` file, the following command is used:
```bash
nitro-cli describe-eif --eif-path ../eif/init_c_eif/app-builder-secure-enclaves-framework.eif
```
We get the following output:
```json
{
  "EifVersion": 4,
  "Measurements": {
    "HashAlgorithm": "Sha384 { ... }",
    "PCR0": "bc0acfdeaa10d267ede8681f50b3b800336a3b585d016d4a3990d0baa8dfe9545498ef9ded1af24136f2929f1602554a",
    "PCR1": "bd78456d3ac7bce218c532a1882cff7f7e76e28c8d898eed888cfbb44ee97bd3f27c7fbae6d52bce4205595779f40c59",
    "PCR2": "6a32eb123d5bd397a2289cbc0b89d8c4d701a5e915d4f3b8ad75d746c6b72989ad18d5d56990e4a20355926ab87701c1"
  },
  // Other fields...
}
```
We can see that the `PCR` values in the response match the `PCR` values in the output. This proves that the base image is intact and has not been tampered with.


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