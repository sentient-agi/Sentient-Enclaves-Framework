from dotenv import load_dotenv
from model import OAICompatibleModelEndpoint
from twitter_handler import TwitterHandler
import os
import asyncio
import time
from utils import load_saved_data, save_replied_tweets, construct_prompt_for_tweet_reply, obtain_replies_to_process, construct_prompt_for_tweet
from contract_handler import ContractHandler
from datetime import datetime
from logger import logger
import argparse
# Load environment variables
load_dotenv()
REPLIED_TWEETS_FILE = "replied_tweets.json"
REPLIED_POSTS_FILE = "replied_posts.json"

def perform_blockchain_transaction_for_tweet(contract_handler, tweet_id: str):
    logger.info("Performing blockchain transaction for tweet %s", tweet_id)
    tx_hash = contract_handler.emit_event(tweet_id)
    receipt = contract_handler.node.eth.wait_for_transaction_receipt(tx_hash)
    return receipt
   
def process_tweet_reply(twitter, model, contract_handler, tweet, reply):
    reply_id = str(reply['id'])
    reply_text = reply['text']
    tweet_text = tweet['text']
    user_handle = reply['user']
    try:
        # Step 1: Generate response
        logger.debug("Generating response for reply text: %s", reply_text)
        logger.debug("Tweet text: %s", tweet_text)
        logger.debug("User handle: %s", user_handle)
        prompt = construct_prompt_for_tweet_reply(tweet_text, reply_text, user_handle)
        logger.debug("Prompt: %s", prompt)
        response = model.chat_completion(prompt)
        logger.debug("Response: %s", response)
        
        # Step 2: Post Twitter reply
        success = twitter.post_reply(
            tweet_id=reply_id,
            reply_text=response
        )
        if not success:
            raise Exception("Failed to post Twitter reply")
        # Step 3: Perform blockchain transaction only after successful reply
        receipt = perform_blockchain_transaction_for_tweet(contract_handler, reply_id)
        if not receipt or receipt.get("status") != 1:
            logger.warning("Warning: Twitter reply posted but blockchain transaction failed for %s", reply_id)
        return True
        
    except Exception as e:
        logger.error("Error processing reply %s: %s", reply_id, str(e))
        return False

def process_twitter_replies(twitter, model, contract_handler, replied_tweets, replies):
    """Main function to process Twitter replies using our model"""
    try:
        user_id = twitter.user_id
        # Get recent tweets
        logger.info("Getting recent tweets")
        recent_tweets = twitter.get_recent_tweets(hours_ago=7*24, user_id=user_id )
        if not recent_tweets:
            raise Exception("No tweets found. Please verify Twitter credentials and account status.")
        logger.info("Found %d recent tweets", len(recent_tweets))
        logger.debug("Recent tweets:%s", recent_tweets)
        
        # Add debug logging
        logger.debug("Current replies variable:%s", replies)
        
        # Only make a request if we don't have any replies queued up
        if not replies:
            logger.info("No unprocessed replies found. Fetching new replies...")
            tweet_ids = [tweet['id'] for tweet in recent_tweets]
            twitter.get_replies_to_tweets(tweet_ids, replies, user_id)
        else:
            logger.info("Found %d unprocessed replies. Processing them now...", len(replies))
        
        if not replies:
            logger.info("No new replies found. Sleeping for now!!")
            return

        logger.info("Found %d replies to recent tweets", len(replies))

        selected_replies = obtain_replies_to_process(replies, 2)

        for reply in selected_replies:
            original_tweet = None
            for tweet in recent_tweets:
                if tweet['id'] == reply['original_tweet_id']:
                    original_tweet = tweet
                    break
            
            logger.debug("Original tweet:%s", original_tweet)
            if not original_tweet:
                logger.warning("Original tweet not found for reply %s", reply['id'])
                continue
            status = process_tweet_reply(twitter, model, contract_handler, original_tweet, reply)
            if status:
                replied_tweets[str(reply['id'])] = {
                    'timestamp': str(datetime.now()),
                    'original_tweet_id': original_tweet['id']
                }
                save_replied_tweets(replied_tweets, REPLIED_TWEETS_FILE)

    except Exception as e:
        logger.error("Error fetching recent tweets: %s", e)
        return

def process_twitter_posts(twitter, model, contract_handler, replied_posts, posts, user_id, user_handle):
    logger.info("Processing Twitter posts from user:%s", user_id)
    if not posts:
        logger.info("No unprocessed posts found. Fetching new posts...")
        posts = twitter.get_posts_from_user(user_id, posts)
    else:
        logger.info("Found %d unprocessed posts. Processing them now...", len(posts))
        logger.debug("Posts:%s", posts)
    
    if not posts:
        logger.info("No new posts found. Sleeping for now!!")
        return
    
    logger.info("Found %d posts to process", len(posts))

    selected_posts = obtain_replies_to_process(posts, 1)
    
    for post in selected_posts:
        post_id = post['id']
        post_text = post['text']
        logger.info("Replying to post:%s", post_text)
        # Generate response
        logger.debug("Generating response for post_text:%s", post_text)
        prompt = construct_prompt_for_tweet(post_text, user_handle)
        logger.debug("Prompt: %s", prompt)
        response = model.chat_completion(prompt)
    
        logger.debug("Response:%s", response)

        # Post reply
        success = twitter.post_reply(
            tweet_id=post_id,
            reply_text=response
        )

        if not success:
            raise Exception("Failed to post tweet")
        
        # Perform blockchain transaction
        receipt = perform_blockchain_transaction_for_tweet(contract_handler, post_id)
        if not receipt or receipt.get("status") != 1:
            logger.warning("Warning: Twitter reply posted but blockchain transaction failed for %s", post_id)
        
        if success: 
            replied_posts[str(post_id)] = {
                'timestamp': str(datetime.now()),
                'original_post_id': post_id
            }
        
        # Save replied posts. Maybe this can be avoided?
        save_replied_tweets(replied_posts, REPLIED_POSTS_FILE)

def main():
    """Main entry point"""
     # Add argument parser
    parser = argparse.ArgumentParser(description='Twitter Agent with multiple operation modes')
    parser.add_argument('--mode', choices=['reply_all', 'track_user'], default='track_user',
                      help='Operation mode: track_user (default) or reply_all')
    parser.add_argument('--username', type=str, help='Username to track in track_user mode')
    args = parser.parse_args()
    
    if args.mode == 'track_user' and not args.username:
        parser.error("track_user mode requires --username argument")
        
    username = args.username
    logger.info("Tracking user:%s", username)
    try:
        # Initialize OpenAI compatible model endpoint
        model = OAICompatibleModelEndpoint(
            api_key=os.getenv("MODEL_API_KEY"),
            base_url=os.getenv("SERVER_BASE_URL"),
            model=os.getenv("MODEL_NAME")
        )
        
        # Initialize the replied tweets hashmap
        replied_tweets = load_saved_data(REPLIED_TWEETS_FILE)
        replies = []
  
        replied_posts = load_saved_data(REPLIED_POSTS_FILE)
        posts = []
        # Initialize Twitter handler with replied tweets hashmap
        twitter = TwitterHandler(
            api_key=os.getenv("TWITTER_API_KEY"),
            api_secret=os.getenv("TWITTER_API_SECRET"),
            access_token=os.getenv("TWITTER_ACCESS_TOKEN"),
            access_token_secret=os.getenv("TWITTER_ACCESS_TOKEN_SECRET"),
            bearer_token=os.getenv("TWITTER_BEARER_TOKEN"),
            replied_tweets=replied_tweets,
            replied_posts=replied_posts
        )
        
        contract_handler = ContractHandler(
            private_key=os.getenv("AGENT_PRIVATE_KEY"),
            contract_address=os.getenv("CONTRACT_ADDRESS"),
            contract_abi_path=os.getenv("CONTRACT_ABI_PATH"),
            rpc_url=os.getenv("RPC_URL"),
            chain_id=os.getenv("CHAIN_ID")
        )
        
        user_id = twitter.get_user_id(username)
        user_id = "1879064952037871616"
        
        # Run the Twitter processing loop
        while True:
            if args.mode == 'reply_all':
                process_twitter_replies(twitter, model, contract_handler, replied_tweets, replies)
                logger.debug("Replied tweets:%s", replied_tweets)
                logger.debug("Replies:%s", replies)
            
            elif args.mode == 'track_user':
                process_twitter_posts(twitter, model, contract_handler, replied_posts, posts, user_id, username)
                logger.debug("Replied posts:%s", replied_posts)
                logger.debug("Posts:%s", posts)

            for i in range(15):
                logger.info("Waiting for %d minutes before checking again...", 15 - i)
                time.sleep(60)
    except KeyboardInterrupt:
        logger.info("Shutting down gracefully...")
        save_replied_tweets(replied_tweets, REPLIED_TWEETS_FILE)
        save_replied_tweets(replied_posts, REPLIED_POSTS_FILE)
    except Exception as e:
        logger.error("Unexpected error: %s", e)
        save_replied_tweets(replied_tweets, REPLIED_TWEETS_FILE)
        save_replied_tweets(replied_posts, REPLIED_POSTS_FILE)

if __name__ == "__main__":
    main()

