# Twitter Sentiment Integration Guide

## Overview

The system now includes ML-based Twitter sentiment analysis for crypto/memecoin trading signals. This integration uses a Python FastAPI service with a fine-tuned DistilRoBERTa model for financial sentiment analysis.

## Architecture

```
Python Service (FastAPI) → Twitter API → ML Model (DistilRoBERTa)
     ↓
Rust Backend → HTTP Client → TwitterSentimentClient
     ↓
Trading Engine → Enhanced MemeSentiment → Trading Signals
```

## Components

### 1. Python Sentiment Service

**Location:** `sentiment_service/`

**Files:**
- `main.py` - FastAPI service with `/api/sentiment` endpoint
- `analyzer.py` - ML-based sentiment analysis using DistilRoBERTa
- `requirements.txt` - Python dependencies

**Features:**
- ML-based sentiment analysis (DistilRoBERTa fine-tuned for financial news)
- Twitter API v2 integration
- Community growth tracking
- Volume growth analysis
- Engagement metrics

### 2. Rust Twitter Sentiment Client

**Location:** `backend/src/twitter_sentiment.rs`

**Key Types:**
- `TwitterSentimentClient` - HTTP client for Python service
- `TwitterSentimentData` - Sentiment analysis results
- `CommunityMetrics` - Follower and engagement data
- `EngagementMetrics` - Likes, retweets, tweet counts

### 3. Enhanced MemeSentiment

**Location:** `backend/src/pumpfun.rs`

**New Fields:**
- `twitter_sentiment: Option<TwitterSentimentData>` - Full Twitter analysis
- `twitter_weighted_polarity: Option<f64>` - ML polarity score (-1.0 to 1.0)
- `community_growth_pct: Option<f64>` - Twitter volume growth percentage

## Setup Instructions

### Step 1: Install Python Dependencies

```bash
cd sentiment_service
pip install -r requirements.txt
```

### Step 2: Configure Twitter API Credentials

Create `sentiment_service/login.csv` with a single column `key` and 5 rows:
1. Consumer Key
2. Consumer Secret
3. Access Token
4. Access Token Secret
5. Bearer Token

**OR** set environment variables:
- `TWITTER_CONSUMER_KEY`
- `TWITTER_CONSUMER_SECRET`
- `TWITTER_ACCESS_TOKEN`
- `TWITTER_ACCESS_TOKEN_SECRET`
- `TWITTER_BEARER_TOKEN`

### Step 3: Start Python Service

```bash
cd sentiment_service
uvicorn main:app --host 0.0.0.0 --port 8000
```

### Step 4: Configure Rust Backend

Add to `.env`:
```bash
TWITTER_SENTIMENT_SERVICE_URL=http://localhost:8000
```

### Step 5: Run Rust Backend

```bash
cd backend
cargo run --bin agentburn-backend
```

The backend will automatically detect if the Twitter sentiment service is available and log the status.

## Usage

### Automatic Integration

The Twitter sentiment is automatically integrated into memecoin analysis:

1. **MemeAnalyzer** uses Twitter sentiment when available
2. **PumpFunClient** can enhance sentiment with `analyze_sentiment_with_twitter()`
3. **Specialized Providers** benefit from enhanced sentiment scores

### Manual Usage

```rust
use crate::twitter_sentiment::TwitterSentimentClient;

let client = TwitterSentimentClient::new("http://localhost:8000".to_string());

// Get sentiment for a symbol
let sentiment = client.get_sentiment("DOGE", None, None).await?;

// Use in analysis
let enhanced_sentiment = pumpfun_client
    .analyze_sentiment_with_twitter(&launch, Some(&client))
    .await;
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

**Response:**
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

## How It Works

### 1. Sentiment Analysis Flow

1. **Tweet Fetching**: Service fetches up to 500 recent tweets for the symbol
2. **Text Cleaning**: Removes URLs, mentions, preserves hashtags and emojis
3. **ML Analysis**: DistilRoBERTa model analyzes each tweet
4. **Weighted Scoring**: Combines ML scores with engagement metrics (likes, retweets, followers)
5. **Aggregation**: Calculates overall sentiment and growth metrics

### 2. Integration with Trading Signals

- **Sentiment Score Adjustment**: Twitter polarity (-1.0 to 1.0) adjusts base sentiment score by ±20 points
- **Hype Level**: High Twitter engagement (>100 avg likes + positive sentiment) boosts hype level
- **Safety Checks**: Negative Twitter sentiment (< -0.2) can fail safety checks
- **Social Signals**: Twitter metrics added to social signals list

### 3. Error Handling

- **Service Unavailable**: System gracefully falls back to base sentiment analysis
- **Retry Logic**: Uses conservative retry config (2 attempts, exponential backoff)
- **Circuit Breaker**: Integrated with existing error handling patterns
- **Timeout Protection**: 30-second timeout for API calls

## Performance Considerations

### Model Loading
- DistilRoBERTa model is loaded once and cached
- First request may be slower (~5-10 seconds for model download)
- Subsequent requests are fast (~1-2 seconds per analysis)

### Rate Limiting
- Twitter API v2 free tier: 500 tweets per request
- Service respects Twitter rate limits automatically
- Consider upgrading Twitter API tier for higher volume

### Caching (Future Enhancement)
- Consider caching sentiment results for 5-10 minutes
- Reduces API calls and improves response time

## Troubleshooting

### Service Not Available
```
⚠️  Twitter Sentiment service not available at http://localhost:8000
💡 To enable Twitter sentiment: Start the Python service (see sentiment_service/README.md)
```

**Solution:** Start the Python service or check the URL in `.env`

### Model Download Issues
If the model fails to download:
1. Check internet connection
2. Verify `transformers` library is installed
3. Model will be cached after first download

### Twitter API Errors
- Verify credentials in `login.csv` or environment variables
- Check Twitter API rate limits
- Ensure Twitter API v2 access is enabled

## Future Enhancements

1. **Caching Layer**: Cache sentiment results to reduce API calls
2. **Batch Analysis**: Analyze multiple symbols in one request
3. **Historical Tracking**: Store sentiment history for trend analysis
4. **Real-time Updates**: WebSocket integration for live sentiment updates
5. **Multi-Model Ensemble**: Combine multiple sentiment models for better accuracy

## Security Notes

- Twitter API credentials should be stored securely
- Use environment variables in production
- Never commit `login.csv` to version control
- Service should run behind authentication in production

## References

- **DistilRoBERTa Model**: `mrm8488/distilroberta-finetuned-financial-news-sentiment-analysis`
- **Twitter API v2**: https://developer.twitter.com/en/docs/twitter-api
- **FastAPI**: https://fastapi.tiangolo.com/
- **Transformers**: https://huggingface.co/docs/transformers/

