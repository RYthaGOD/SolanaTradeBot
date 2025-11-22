"""
Twitter Sentiment Analyzer
ML-based sentiment analysis using DistilRoBERTa fine-tuned for financial news
"""

import tweepy
import pandas as pd
import numpy as np
import re
import datetime
from typing import Optional, List, Dict, Any
import logging
from transformers import AutoTokenizer, AutoModelForSequenceClassification
import torch

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Global model loading (load once, reuse)
_tokenizer = None
_model = None


def load_model():
    """Load ML model once and cache it"""
    global _tokenizer, _model
    if _tokenizer is None or _model is None:
        logger.info("Loading DistilRoBERTa financial sentiment model...")
        _tokenizer = AutoTokenizer.from_pretrained(
            "mrm8488/distilroberta-finetuned-financial-news-sentiment-analysis"
        )
        _model = AutoModelForSequenceClassification.from_pretrained(
            "mrm8488/distilroberta-finetuned-financial-news-sentiment-analysis"
        )
        _model.eval()  # Set to evaluation mode
        logger.info("✅ Model loaded successfully")
    return _tokenizer, _model


def load_credentials():
    """Load Twitter API credentials from login.csv"""
    try:
        log = pd.read_csv('login.csv')
        return {
            'consumer_key': log['key'][0],
            'consumer_secret': log['key'][1],
            'access_token': log['key'][2],
            'access_token_secret': log['key'][3],
            'bearer_token': log['key'][4]
        }
    except Exception as e:
        logger.error(f"Failed to load credentials: {e}")
        # Try environment variables as fallback
        import os
        return {
            'consumer_key': os.getenv('TWITTER_CONSUMER_KEY', ''),
            'consumer_secret': os.getenv('TWITTER_CONSUMER_SECRET', ''),
            'access_token': os.getenv('TWITTER_ACCESS_TOKEN', ''),
            'access_token_secret': os.getenv('TWITTER_ACCESS_TOKEN_SECRET', ''),
            'bearer_token': os.getenv('TWITTER_BEARER_TOKEN', '')
        }


def get_twitter_client():
    """Get authenticated Twitter API client"""
    creds = load_credentials()
    return tweepy.Client(
        bearer_token=creds['bearer_token'],
        consumer_key=creds['consumer_key'],
        consumer_secret=creds['consumer_secret'],
        access_token=creds['access_token'],
        access_token_secret=creds['access_token_secret'],
        wait_on_rate_limit=True
    )


def clean_tweet(tweet: str) -> str:
    """Clean tweet text for analysis"""
    tweet = re.sub(r'https?://\S+', '', tweet)  # Remove URLs
    tweet = re.sub(r'@[A-Za-z0-9_]+', '', tweet)  # Remove mentions
    return tweet.strip()  # Preserve hashtags, emojis, caps


def get_ml_sentiment(tweet: str) -> float:
    """
    Get ML-based sentiment score using DistilRoBERTa
    
    Returns:
        float: Polarity score from -1.0 (negative) to 1.0 (positive)
    """
    tokenizer, model = load_model()
    
    inputs = tokenizer(tweet, return_tensors="pt", truncation=True, max_length=512)
    
    with torch.no_grad():
        outputs = model(**inputs)
    
    logits = outputs.logits
    probs = torch.softmax(logits, dim=-1)
    labels = ['negative', 'neutral', 'positive']  # Model's label order
    
    # Calculate polarity: positive probability - negative probability
    positive_prob = probs[0][labels.index('positive')].item()
    negative_prob = probs[0][labels.index('negative')].item()
    score = positive_prob - negative_prob  # Range: -1 to 1
    
    return score


def fetch_tweets(client: tweepy.Client, search_term: str, max_tweets: int = 500) -> pd.DataFrame:
    """Fetch tweets with pagination"""
    all_tweets = []
    next_token = None
    
    while len(all_tweets) < max_tweets and (next_token or not all_tweets):
        try:
            tweets_response = client.search_recent_tweets(
                query=search_term,
                tweet_fields=['text', 'public_metrics', 'created_at'],
                expansions=['author_id'],
                user_fields=['public_metrics'],
                max_results=100,
                next_token=next_token
            )
            
            if not tweets_response.data:
                break
            
            users = {u['id']: u for u in tweets_response.includes.get('users', [])}
            
            for tweet in tweets_response.data:
                user = users.get(tweet.author_id)
                if user and user.public_metrics['followers_count'] > 100:
                    all_tweets.append({
                        'Tweets': tweet.text,
                        'Created_At': tweet.created_at,
                        'Likes': tweet.public_metrics['like_count'],
                        'Retweets': tweet.public_metrics['retweet_count'],
                        'Author_Followers': user.public_metrics['followers_count']
                    })
            
            next_token = tweets_response.meta.get('next_token')
            
        except Exception as e:
            logger.error(f"Error fetching tweets: {e}")
            break
    
    if not all_tweets:
        return pd.DataFrame()
    
    df = pd.DataFrame(all_tweets).drop_duplicates(subset=['Tweets'])
    return df


def get_community_metrics(client: tweepy.Client, community_accounts: Optional[List[str]]) -> Dict[str, Any]:
    """Get follower and engagement metrics for community accounts"""
    if not community_accounts:
        return {
            'avg_followers': 0.0,
            'total_followers': 0,
            'top_accounts': []
        }
    
    user_data = []
    for username in community_accounts:
        try:
            user = client.get_user(username=username, user_fields=['public_metrics'])
            if user.data:
                metrics = user.data.public_metrics
                user_data.append({
                    'username': username,
                    'followers': metrics['followers_count'],
                    'tweet_count': metrics['tweet_count'],
                    'listed_count': metrics['listed_count']
                })
        except Exception as e:
            logger.warn(f"Failed to fetch user {username}: {e}")
            continue
    
    if not user_data:
        return {
            'avg_followers': 0.0,
            'total_followers': 0,
            'top_accounts': []
        }
    
    followers_df = pd.DataFrame(user_data)
    followers_df.sort_values('followers', ascending=False, inplace=True)
    
    return {
        'avg_followers': followers_df['followers'].mean(),
        'total_followers': int(followers_df['followers'].sum()),
        'top_accounts': followers_df.head(10).to_dict('records')
    }


def get_volume_growth(client: tweepy.Client, search_term: str) -> float:
    """Get tweet volume growth rate"""
    try:
        counts_response = client.get_recent_tweets_count(query=search_term, granularity='day')
        if not counts_response.data:
            return 0.0
        
        tweet_counts = pd.DataFrame([
            {'Date': count['end'][:10], 'Count': count['tweet_count']}
            for count in counts_response.data
        ])
        
        if len(tweet_counts) < 2:
            return 0.0
        
        tweet_counts['Date'] = pd.to_datetime(tweet_counts['Date'])
        tweet_counts.sort_values('Date', inplace=True)
        tweet_counts['Growth'] = tweet_counts['Count'].pct_change() * 100
        
        # Return average growth over available period
        recent_growth = tweet_counts['Growth'].tail(7).mean()
        return float(recent_growth) if not pd.isna(recent_growth) else 0.0
        
    except Exception as e:
        logger.warn(f"Failed to get volume growth: {e}")
        return 0.0


def analyze_symbol(
    symbol: str,
    search_term: Optional[str] = None,
    community_accounts: Optional[List[str]] = None
) -> Dict[str, Any]:
    """
    Main analysis function for a symbol
    
    Args:
        symbol: Token symbol to analyze (e.g., "DOGE", "SHIB")
        search_term: Custom search query (optional)
        community_accounts: List of Twitter usernames to track (optional)
    
    Returns:
        Dictionary with sentiment analysis results
    """
    logger.info(f"Starting sentiment analysis for {symbol}")
    
    # Build search term if not provided
    if not search_term:
        search_term = f'#{symbol} OR ${symbol} -is:retweet lang:en'
    
    # Get Twitter client
    client = get_twitter_client()
    
    # Fetch tweets
    logger.info(f"Fetching tweets for {symbol}...")
    df = fetch_tweets(client, search_term, max_tweets=500)
    
    if df.empty:
        logger.warn(f"No tweets found for {symbol}")
        return {
            'symbol': symbol,
            'weighted_polarity': 0.0,
            'sentiment': 'Neutral',
            'volume_growth': 0.0,
            'community_growth': {
                'avg_followers': 0.0,
                'total_followers': 0,
                'top_accounts': []
            },
            'engagement_metrics': {
                'avg_likes': 0.0,
                'avg_retweets': 0.0,
                'total_tweets': 0
            },
            'timestamp': datetime.datetime.now().isoformat()
        }
    
    # Clean tweets
    df['Cleaned_Tweets'] = df['Tweets'].apply(clean_tweet)
    
    # ML-based sentiment analysis
    logger.info(f"Analyzing sentiment for {len(df)} tweets...")
    df['Polarity'] = df['Cleaned_Tweets'].apply(get_ml_sentiment)
    
    # Classify sentiment
    def get_sentiment(score: float) -> str:
        if score <= -0.05:
            return 'Negative'
        elif score >= 0.05:
            return 'Positive'
        else:
            return 'Neutral'
    
    df['Sentiment'] = df['Polarity'].apply(get_sentiment)
    
    # Calculate weighted polarity
    df['Weighted_Polarity'] = df['Polarity'] * (
        df['Likes'] + df['Retweets'] + (df['Author_Followers'] / 1000) + 1
    )
    total_weight = (df['Likes'] + df['Retweets'] + (df['Author_Followers'] / 1000) + 1).sum()
    avg_weighted_polarity = df['Weighted_Polarity'].sum() / total_weight if total_weight > 0 else 0.0
    
    # Engagement metrics
    avg_likes = float(df['Likes'].mean())
    avg_retweets = float(df['Retweets'].mean())
    
    # Community growth metrics
    community_metrics = get_community_metrics(client, community_accounts)
    
    # Volume growth
    volume_growth = get_volume_growth(client, search_term)
    
    # Determine overall sentiment
    sentiment = get_sentiment(avg_weighted_polarity)
    
    result = {
        'symbol': symbol,
        'weighted_polarity': float(avg_weighted_polarity),
        'sentiment': sentiment,
        'volume_growth': float(volume_growth),
        'community_growth': {
            'avg_followers': float(community_metrics['avg_followers']),
            'total_followers': int(community_metrics['total_followers']),
            'top_accounts': [
                {
                    'username': acc['username'],
                    'followers': int(acc['followers']),
                    'tweet_count': int(acc['tweet_count']),
                    'listed_count': int(acc.get('listed_count', 0))
                }
                for acc in community_metrics['top_accounts']
            ]
        },
        'engagement_metrics': {
            'avg_likes': avg_likes,
            'avg_retweets': avg_retweets,
            'total_tweets': len(df)
        },
        'timestamp': datetime.datetime.now().isoformat()
    }
    
    logger.info(f"✅ Analysis complete: {sentiment} (polarity: {avg_weighted_polarity:.3f})")
    return result

