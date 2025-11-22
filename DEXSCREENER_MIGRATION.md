# DEX Screener API Migration Summary

## Overview
Migrated from Mobula API to official DEX Screener API as fallback for token discovery and analysis.

## API Reference
- **Documentation**: https://docs.dexscreener.com/api/reference
- **Base URL**: `https://api.dexscreener.com`
- **Rate Limits**:
  - Search/Pairs endpoints: **300 requests per minute**
  - Profile/Boost endpoints: **60 requests per minute**

## Changes Made

### 1. Rate Limiting Implementation
- **Separate rate limiters** for different endpoint types:
  - `check_search_rate_limit()`: 280 req/min (conservative, below 300 limit) for search/pairs endpoints
  - `check_profile_rate_limit()`: 55 req/min (conservative, below 60 limit) for profile/boost endpoints

### 2. API Endpoints Updated
- **Search**: `GET /latest/dex/search?q={query}` (300/min)
- **Get Token Pairs**: `GET /token-pairs/v1/{chainId}/{tokenAddress}` (300/min)
- **Get Pair**: `GET /latest/dex/pairs/{chainId}/{pairId}` (300/min)
- **Trending**: Uses search endpoint with popular token queries

### 3. Response Structures
- Updated from `MobulaResponse` to `DexScreenerSearchResponse`
- Search response: `{ "schemaVersion": "text", "pairs": [...] }`
- Token pairs response: Direct array `[{ pair objects... }]`

### 4. Rate Limit Enforcement
- Automatic tracking per endpoint type
- Automatic waiting when approaching limits
- Thread-safe implementation using Arc<Mutex<>>

## Remaining TODOs
- [ ] Update all log messages referencing "Mobula" to "DEX Screener"
- [ ] Update response parsing in `search_tokens()` to use `DexScreenerSearchResponse`
- [ ] Update response parsing in `get_token_pairs()` to use array format
- [ ] Update response parsing in `get_pair()` to match DEX Screener format
- [ ] Test all endpoints with actual DEX Screener API

## Notes
- DEX Screener API does not require API key (unlike Mobula)
- Response structure is slightly different from Mobula
- Rate limits are stricter (300/min vs 1000/min for Mobula free tier)

