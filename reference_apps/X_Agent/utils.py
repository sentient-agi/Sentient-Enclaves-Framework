import json
from pathlib import Path
import random
from logger import logger

def load_saved_data(file_name):
    if Path(file_name).exists():
        try:
            with open(file_name, 'r') as f:
                data = f.read().strip()
                if not data:
                    logger.info(f"{file_name} is empty. Initializing with empty dictionary.")
                    return {}
                return json.loads(data)
        except json.JSONDecodeError as e:
            logger.warning(f"Error decoding JSON from {file_name}: {e}")
            logger.warning("Initializing with empty dictionary.")
            return {}
        except Exception as e:
            logger.error(f"Unexpected error loading {file_name}: {e}")
            return {}
    else:
        logger.info(f"{file_name} does not exist. Creating a new one.")
        return {}

def save_replied_tweets(replied_tweets, file_name):
    with open(file_name, 'w') as f:
        json.dump(replied_tweets, f)

def construct_prompt_for_tweet_reply(tweet_text: str, reply_text: str, user_handle: str):
    return f"""
    Generate a friendly and engaging response to this tweet reply.
    Original tweet: {tweet_text}
    Reply from @{user_handle}: {reply_text}
    Keep the response under 280 characters and maintain a helpful, professional tone.
    """

def construct_prompt_for_tweet(tweet_text: str, user_handle: str):
    return f"""
    Generate a friendly and engaging response to this tweet. It should be quirky and short.
    Tweet: {tweet_text}
    User handle: @{user_handle}
    It should be quirky and short.
    Keep the response under 280 characters and maintain a cheerful tone.
    """

def obtain_replies_to_process(tweets_list, to_process=2):
    """Select random tweets to process and remove them from the main tweets_list list"""
    if not tweets_list:
        return []
    logger.debug("Original tweets_list", tweets_list)
    # If we have fewer tweets_list than requested, process all of them
    num_to_process = min(len(tweets_list), to_process)

    # Randomly select tweets_list
    selected_tweets_list = random.sample(tweets_list, num_to_process)

    # Remove selected tweets_list from the original list
    for reply in selected_tweets_list:
        tweets_list.remove(reply)
    
    logger.debug("Selected tweets_list", selected_tweets_list)
    logger.debug("Remaining tweets_list", tweets_list)
    return selected_tweets_list
