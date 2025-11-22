"""
Twitter Sentiment Analysis Service
FastAPI service for ML-based sentiment analysis of crypto/memecoin tweets
"""

from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
from typing import Optional, List
import asyncio
import logging
from datetime import datetime

from analyzer import analyze_symbol

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

app = FastAPI(title="Twitter Sentiment Analysis Service", version="1.0.0")

# CORS middleware for cross-origin requests
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],  # In production, specify allowed origins
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


class SentimentRequest(BaseModel):
    symbol: str
    search_term: Optional[str] = None
    community_accounts: Optional[List[str]] = None


class AccountData(BaseModel):
    username: str
    followers: int
    tweet_count: int
    listed_count: int


class CommunityMetrics(BaseModel):
    avg_followers: float
    total_followers: int
    top_accounts: List[AccountData]


class EngagementMetrics(BaseModel):
    avg_likes: float
    avg_retweets: float
    total_tweets: int


class SentimentResponse(BaseModel):
    symbol: str
    weighted_polarity: float
    sentiment: str  # "Positive", "Negative", "Neutral"
    volume_growth: float
    community_growth: CommunityMetrics
    engagement_metrics: EngagementMetrics
    timestamp: str


@app.get("/health")
async def health():
    """Health check endpoint"""
    return {"status": "healthy", "service": "twitter-sentiment", "timestamp": datetime.now().isoformat()}


@app.post("/api/sentiment", response_model=SentimentResponse)
async def analyze_sentiment(request: SentimentRequest):
    """
    Analyze Twitter sentiment for a given symbol
    
    Args:
        request: SentimentRequest with symbol and optional search parameters
    
    Returns:
        SentimentResponse with comprehensive sentiment analysis
    """
    try:
        logger.info(f"Analyzing sentiment for symbol: {request.symbol}")
        
        # Run sentiment analysis in thread pool to avoid blocking
        result = await asyncio.to_thread(
            analyze_symbol,
            request.symbol,
            request.search_term,
            request.community_accounts
        )
        
        logger.info(f"✅ Sentiment analysis complete for {request.symbol}: {result['sentiment']}")
        return SentimentResponse(**result)
        
    except Exception as e:
        logger.error(f"❌ Error analyzing sentiment for {request.symbol}: {e}", exc_info=True)
        raise HTTPException(status_code=500, detail=f"Sentiment analysis failed: {str(e)}")


@app.get("/")
async def root():
    """Root endpoint with service information"""
    return {
        "service": "Twitter Sentiment Analysis Service",
        "version": "1.0.0",
        "endpoints": {
            "health": "/health",
            "sentiment": "/api/sentiment (POST)"
        }
    }


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000)

