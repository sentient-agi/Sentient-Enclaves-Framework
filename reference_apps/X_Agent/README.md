# 🤖 Twitter x Blockchain Agent

This is a simple agent that interacts with replies to tweets from a specified user. For each successful reply generated using an LLM, the agent will post a reply to the tweet and perform a blockchain transaction consuming some amount of gas.

> [!NOTE]
> This agent is designed to be run as a reference application on Sentinent' Secure Enclaves Framework. For a full-fledged agent framework, refer to [Autonomous Agents](https://github.com/sentient-agi/autonomous-agents/).

## Agent Features:
* Allows fetching recent tweets from a user in the last 7 days
* For each tweet, fetches all replies to that tweet
* For each reply, generates a reply using an LLM
* Posts the reply to the tweet
* Performs a blockchain transaction consuming some amount of gas


# 🚀 Quickstart

## Setting Up Credentials 🔐

### 1.&nbsp;&nbsp;Create secrets file

Create the `.env` file by copying the contents of `.env.example`. This is where you will store all of your agent's credentials.
```
cp .env.example .env
```

### 2.&nbsp;&nbsp;Get X credentials
> [!WARNING]
> **We suggest creating a new X account for your agent.**

In order to interact with the X platform, your agent needs X developer credentials from the X developer portal [here](https://developer.x.com/en/portal/dashboard).

From the *Dashboard* page, click on the gear icon to access the *Settings* page for your default project.

Set the user authentication settings for your app as follows:
- App permissions: "Read and write"
- Type of App: "Web App, Automated App or Bot"
- Callback URI / Redirect URL: http://localhost:8000
- Website URL: http://example.com

Generate all of the required credentials on the *Keys and tokens* page. Add them to the `.env` file.

### 3.&nbsp;&nbsp;Get model inference credentials
Add your Fireworks or other inference provider base url, model name and API key to the `.env` file.

### 4.&nbsp;&nbsp;Get blockchain credentials
Add your blockchain node url, contract address, and private key to the `.env` file.

## Running Locally 💻
> [!NOTE]
> **Before you proceed, make sure that you have installed `python`.**

### 1.&nbsp;&nbsp;Install `pdm`
On Unix / MacOS:
```
curl -sSL https://pdm-project.org/install-pdm.py | python3 -
```

### 2.&nbsp;&nbsp;Install dependencies
```
pdm install
```


### 3.&nbsp;&nbsp;Activate the Python virtual environment
On Unix / MacOS:
```
source .venv/bin/activate
```

### 4. Set up logging
Default logging is set to INFO level. You can change this to DEBUG level by setting the `LOG_LEVEL` environment variable to `DEBUG` as follows:
```
export LOG_LEVEL=DEBUG
```

### 5.&nbsp;&nbsp;Run your agent
```
python3 -u agent.py
```

# 🧐 Technical Information

## X API
> [!NOTE]
> **It is important to consider X API [rate limits](https://docs.x.com/x-api/fundamentals/rate-limits#v2-limits). This example agent is written such that it can be run using the free plan. Every time the agent runs, with free plan, it makes calls to different endpoints. Depending on the number of replies to the tweets, it may hit the rate limits, prompting the agent to wait for 15 minutes before running again.** 

These are the endpoints that this agent uses:

### Get user tweets
- Endpoint: `GET /2/users/{id}/tweets`
- Documentation:
    - https://docs.x.com/x-api/posts/user-posts-timeline-by-user-id
- Rate limits:
    - Free: 1 requests / 15 mins
    - Basic ($200/month): 5 requests / 15 mins
    - Pro ($5000/month): 900 requests / 15 mins


### Search for tweets
- Endpoint: `GET /2/tweets/search/recent`
- Documentation:
    - https://docs.x.com/x-api/posts/recent-search
    - https://docs.x.com/x-api/posts/search/integrate/build-a-query
- Rate limits:
    - Free: 1 requests / 15 minutes
    - Basic ($200/month): 60 requests / 15 mins
    - Pro ($5000/month): 300 requests / 15 mins
  
### Post a tweet
- Endpoint: `POST /2/tweets`
- Documentation:
    - https://docs.x.com/x-api/posts/creation-of-a-post
- Rate limits:
    - Free: 17 requests / 24 hours
    - Basic ($200/month): 100 requests / 24 hours
    - Pro ($5000/month): 100 requests / 15 minutes
