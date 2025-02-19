# Setting up an AI agent in the enclave 🤖
> [!IMPORTANT]
> This guide addresses setting up X agent using `pipeline-slc-network-al2023.dockerfile` as the enclave image. Instead refer to [TEE_rbuilds_setup.md](TEE_rbuilds_setup.md) to use the preferred way of utilising [x_agent.dockerfile](x_agentr.dockerfile) as the base image for enclave.

The AI agent that we are setting up is a Python application that uses a custom barebones framework for running AI agents. The agent interacts with twitter API, generates tweets and posts them to twitter and also interacts with blockchains to perform transactions for noting inference requests with the blockchain. The rest of the commands should be run in the enclave's shell that we have opened in the previous section.

## Get the AI agent source code 📥
Clone the repository containing the AI agent source code. Perform git clone command in the enclave's shell.

### Example
```bash
$ git clone https://github.com/shivraj-sj/reference_apps.git && \
    cd reference_apps/X_Agent
```

## Install dependencies 📦
We use `pdm` to install the dependencies. Issue the following command to install `pdm`.

### Install `pdm`
```bash
$ curl -sSL https://pdm-project.org/install-pdm.py | python3 -
```
`pdm` will be installed in the `/app/.local/bin` directory.
> [!NOTE]
> As this is a restricted shell, each command is stateless and all commands are executed relative to the current directory `/apps`. Commands like `cd` need command chaining. Use command chaining to run multiple commands.

### Install packages 📚
Use the `pdm` binary present in the `/apps/.local/bin` directory to install the necessary packages for running the AI agent.
```bash
$ /apps/.local/bin/pdm install -p dobby_agent
```

## Passing the `.env` file 🔑
The `.env` file is used to pass the API keys for the different services. Take a look at `.env.example` file to see the format of the `.env` file. Populate the `.env` file with the appropriate API keys. 

The `.env` file should be present in the `dobby_agent` directory. This can be moved using the `pipeline` application. Issue the following command in a different terminal to move the `.env` file to the enclave.
```bash
./pipeline send-file --port 53000 --cid 127 --localpath <path_to_env_file_directory>/.env --remotepath /apps/.env
```
Example:
```bash
./pipeline send-file --port 53000 --cid 127 --localpath ~/dobby_agent/.env --remotepath /apps/dobby_agent/.env
```

## Run the AI agent 🚀
Specify `-u` flag to run the AI agent in unbuffered mode.

> [!NOTE]
> Issue `nw` to run the AI agent in non-blocking mode.
```bash
cd /apps/dobby_agent && ./.venv/bin/python3 -u agent.py --username DobbyReborn 2>&1 | tee agent.log
```

The agent will fetch recent posts from user `DobbyReborn`. A random post is chosen to be replied to so as not to overshoot the rate limits. Using an inference request to an inference endpoint, the agent will generate a tweet. It notes down the ID of the tweet for which it has generated a tweet and passes it to the blockchain to note down the inference request. Finally, it will post the generated tweet to twitter.

## Check the logs for the AI agent 📋
```bash
cat dobby_agent/logs/twitter_agent.log
```
### Example output
```bash

2025-02-07 12:37:02,004 - twitter_agent - INFO - agent.py:174 - Tracking user:DobbyReborn
2025-02-07 12:37:02,069 - twitter_agent - INFO - utils.py:23 - replied_tweets.json does not exist. Creating a new one.
2025-02-07 12:37:02,069 - twitter_agent - INFO - utils.py:23 - replied_posts.json does not exist. Creating a new one.
2025-02-07 12:37:02,070 - twitter_agent - INFO - twitter_handler.py:19 - Getting user info
2025-02-07 12:37:07,765 - twitter_agent - INFO - twitter_handler.py:25 - Successfully authenticated agent as user ID: 1878674353342341120
2025-02-07 12:37:08,033 - twitter_agent - INFO - twitter_handler.py:65 - User ID for username:DobbyReborn: 1879064952037871616
2025-02-07 12:37:08,033 - twitter_agent - INFO - agent.py:110 - Processing Twitter posts from user:1879064952037871616
2025-02-07 12:37:08,033 - twitter_agent - INFO - agent.py:112 - No unprocessed posts found. Fetching new posts...
2025-02-07 12:37:08,320 - twitter_agent - INFO - agent.py:122 - Found 2 posts to process
2025-02-07 12:37:08,320 - twitter_agent - INFO - agent.py:129 - Replying to post:I tried to organize my thoughts, but they went on a field trip without me. 🧠✈️
2025-02-07 12:37:10,523 - twitter_agent - INFO - agent.py:18 - Performing blockchain transaction for tweet 1887839946528190671
2025-02-07 12:37:14,092 - twitter_agent - INFO - agent.py:224 - Waiting for 15 minutes before checking again...
2025-02-07 12:38:14,152 - twitter_agent - INFO - agent.py:224 - Waiting for 14 minutes before checking again...
2025-02-07 12:39:14,212 - twitter_agent - INFO - agent.py:224 - Waiting for 13 minutes before checking again...
```

> [!WARNING]
>  Beware of twitter's rate limits. The agent will keep generating tweets until the rate limits are exhausted


