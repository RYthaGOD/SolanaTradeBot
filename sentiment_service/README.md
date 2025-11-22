# Twitter Sentiment Analysis Service

ML-based sentiment analysis service for crypto/memecoin Twitter data using DistilRoBERTa fine-tuned for financial news.

## Setup

1. **Install dependencies:**
```bash
pip install -r requirements.txt
```

2. **Configure Twitter API credentials:**

Create a `login.csv` file with a single column `key` and 5 rows:
- Row 1: Consumer Key
- Row 2: Consumer Secret
- Row 3: Access Token
- Row 4: Access Token Secret
- Row 5: Bearer Token

Or set environment variables:
- `TWITTER_CONSUMER_KEY`
- `TWITTER_CONSUMER_SECRET`
- `TWITTER_ACCESS_TOKEN`
- `TWITTER_ACCESS_TOKEN_SECRET`
- `TWITTER_BEARER_TOKEN`

3. **Run the service:**
```bash
uvicorn main:app --host 0.0.0.0 --port 8000
```

## API Endpoints

### Health Check
```
GET /health
```

### Sentiment Analysis
```
POST /api/sentiment
Content-Type: application/json

{
  "symbol": "DOGE",
  "search_term": "#Dogecoin OR $DOGE -is:retweet lang:en",
  "community_accounts": ["dogecoin", "Shibtoken"]
}
```

### Response
```json
{
  "symbol": "DOGE",
  "weighted_polarity": 0.45,
  "sentiment": "Positive",
  "volume_growth": 12.5,
  "community_growth": {
    "avg_followers": 150000.0,
    "total_followers": 1500000,
    "top_accounts": [...]
  },
  "engagement_metrics": {
    "avg_likes": 125.5,
    "avg_retweets": 15.2,
    "total_tweets": 450
  },
  "timestamp": "2024-01-15T10:30:00"
}
```

## Integration with Rust Backend

The Rust backend calls this service via HTTP. Set the service URL:
```bash
TWITTER_SENTIMENT_SERVICE_URL=http://localhost:8000
```

