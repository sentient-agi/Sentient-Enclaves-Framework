import tweepy
from typing import List, Dict
from datetime import datetime, timedelta, timezone
# import time
from logger import logger

class TwitterHandler:
    def __init__(self, api_key: str, api_secret: str, access_token: str, access_token_secret: str, bearer_token: str = None, replied_tweets: dict = None, replied_posts: dict = None):
        # Initialize Twitter API v2 with both OAuth 1.0a and Bearer token
        self.client = tweepy.Client(
            bearer_token=bearer_token,
            consumer_key=api_key,
            consumer_secret=api_secret,
            access_token=access_token,
            access_token_secret=access_token_secret,
            wait_on_rate_limit=True  # Add this to handle rate limits automatically
        )
        try:
            logger.info("Getting user info")
            me = self.client.get_me()
            if me.errors:
                logger.error("Error getting user info: %s", me.errors)
                raise Exception("Failed to get user info")
            self.user_id = me.data.id
            logger.info("Successfully authenticated agent as user ID: %s", self.user_id)
        except Exception as e:
            logger.error("Authentication error: %s", e)
            raise
        
        # Store replied tweets hashmap
        self.replied_tweets = replied_tweets or {}
        # Store replied posts hashmap
        self.replied_posts = replied_posts or {}

    def get_recent_tweets(self, user_id: str, hours_ago: int = 24) -> List[Dict]:
        """Get your tweets from the last n hours using v2 endpoint, ordered oldest first"""
        start_time = datetime.now(timezone.utc) - timedelta(hours=hours_ago)
        
        logger.info("Getting tweets")
        tweets = self.client.get_users_tweets(
            id=user_id,
            max_results=100,
            start_time=start_time,
            tweet_fields=['created_at']
        )

        logger.info("tweets: %s", tweets)
        if not tweets.data:
            return []
            
        # Sort tweets by created_at in ascending order (oldest first)
        sorted_tweets = sorted(tweets.data, key=lambda tweet: tweet.created_at)
        
        return [{
            'id': str(tweet.id),
            'text': tweet.text,
        } for tweet in sorted_tweets]

    def get_user_id(self, username: str) -> str:
        """Get the user ID for a given username"""
        user = self.client.get_user(username=username)
        if user.errors:
            logger.error("Error getting user ID for username:%s: %s", username, user.errors)
            raise Exception("Failed to get user ID for username:%s", username)
        logger.info("User ID for username:%s: %s", username, user.data.id)
        return user.data.id

    def get_replies_to_tweets(self, tweet_ids: List[str],  replies: List[Dict], user_id: str, max_results: int = 10):
        """Get replies to a list of tweets"""       
        # Build OR query for all conversation IDs
        query = " OR ".join(f"conversation_id:{tweet_id.strip()}" for tweet_id in tweet_ids)

        search_results = self.client.search_recent_tweets(
            query=query,
            tweet_fields=['created_at', 'author_id', 'in_reply_to_user_id', 'conversation_id'],
            expansions=['author_id'],
            max_results=max_results
        )

        logger.debug("Search results: %s", search_results)
        if not search_results.data:
            return replies

        # Create user lookup dict
        users = {user.id: user.username for user in search_results.includes['users']}

        for tweet in search_results.data:
            reply_ids = [reply['id'] for reply in replies]            
            if tweet.in_reply_to_user_id == user_id and str(tweet.id) not in self.replied_tweets and str(tweet.id) not in reply_ids:
                replies.append({
                    'id': str(tweet.id),
                    'text': tweet.text,
                    'user': users[tweet.author_id],
                    'original_tweet_id': str(tweet.conversation_id)
                })


    def post_reply(self, tweet_id: str, reply_text: str) -> bool:
        """Post a reply to a specific tweet"""
        try:
            reply = f"{reply_text}"
            self.client.create_tweet(
                in_reply_to_tweet_id=tweet_id,
                text=reply
            )
            return True
        except Exception as e:
            logger.error("Error posting reply: %s", e)
            return False
    
    def get_posts_from_user(self, user_id, posts, max_results = 10, hours_ago = 24) -> List[Dict]:
        """Get posts from the last n hours using v2 endpoint, ordered oldest first"""
        logger.debug("Getting posts from user:%s since last %s hours", user_id, hours_ago)
        start_time = datetime.now(timezone.utc) - timedelta(hours=hours_ago)
        
        search_results = self.client.get_users_tweets(
            id=user_id,
            max_results=max_results,
            start_time=start_time,
            exclude=['replies', 'retweets'],
            tweet_fields=['created_at']
        )
        logger.debug("Search results: %s", search_results)
        
        if not search_results.data:
            return posts
        post_ids = [post['id'] for post in posts]
        for post in search_results.data:
            if str(post.id) not in post_ids and str(post.id) not in self.replied_posts:
                posts.append({
                    'id': str(post.id),
                    'text': post.text,
                })
        logger.debug("New posts: %s", posts)
        return posts