import React, { useState, useEffect } from 'react';
import axios from 'axios';

interface MarketOutcome {
  outcome_id: string;
  name: string;
  price: number;
  shares: number;
  volume: number;
  last_price: number;
  bid?: number;
  ask?: number;
}

interface PredictionMarket {
  market_id: string;
  question: string;
  category: string;
  outcomes: MarketOutcome[];
  liquidity: number;
  volume_24h: number;
  end_date: number;
  resolution_date?: number;
  status: string;
  fee_bps: number;
}

interface PredictionSignal {
  signal_id: string;
  market_id: string;
  outcome_id: string;
  action: string;
  target_price: number;
  confidence: number;
  expected_value: number;
  kelly_fraction: number;
  timestamp: number;
  reasoning: string;
}

interface MarketStats {
  total_markets: number;
  active_markets: number;
  total_liquidity: number;
  volume_24h: number;
}

const API_URL = 'http://localhost:8080';

const PredictionMarkets: React.FC = () => {
  const [markets, setMarkets] = useState<PredictionMarket[]>([]);
  const [selectedMarket, setSelectedMarket] = useState<PredictionMarket | null>(null);
  const [signals, setSignals] = useState<PredictionSignal[]>([]);
  const [stats, setStats] = useState<MarketStats | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchMarkets();
    fetchStats();
    const interval = setInterval(() => {
      fetchMarkets();
      fetchStats();
    }, 30000); // Refresh every 30 seconds

    return () => clearInterval(interval);
  }, []);

  const fetchMarkets = async () => {
    try {
      const response = await axios.get(`${API_URL}/prediction-markets`);
      if (response.data.success) {
        setMarkets(response.data.data);
      }
    } catch (err) {
      console.error('Error fetching markets:', err);
    }
  };

  const fetchStats = async () => {
    try {
      const response = await axios.get(`${API_URL}/prediction-markets/stats`);
      if (response.data.success) {
        setStats(response.data.data);
      }
    } catch (err) {
      console.error('Error fetching stats:', err);
    }
  };

  const fetchSignals = async (marketId: string) => {
    setLoading(true);
    setError(null);
    try {
      const response = await axios.get(`${API_URL}/prediction-markets/signals/${marketId}`);
      if (response.data.success) {
        setSignals(response.data.data);
      } else {
        setError(response.data.message);
      }
    } catch (err) {
      setError('Failed to fetch signals');
      console.error('Error fetching signals:', err);
    } finally {
      setLoading(false);
    }
  };

  const handleMarketClick = async (market: PredictionMarket) => {
    setSelectedMarket(market);
    await fetchSignals(market.market_id);
  };

  const executeTrade = async (marketId: string, outcomeId: string, action: string) => {
    try {
      const response = await axios.post(`${API_URL}/prediction-markets/trade`, {
        market_id: marketId,
        outcome_id: outcomeId,
        action: action,
        amount: '100' // Default $100
      });
      if (response.data.success) {
        alert(`Trade executed successfully! Trade ID: ${response.data.data}`);
      } else {
        alert(`Trade failed: ${response.data.message}`);
      }
    } catch (err) {
      alert('Failed to execute trade');
      console.error('Error executing trade:', err);
    }
  };

  const formatDate = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleDateString();
  };

  const formatCurrency = (value: number) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(value);
  };

  const formatPercent = (value: number) => {
    return `${(value * 100).toFixed(1)}%`;
  };

  const getCategoryColor = (category: string) => {
    const colors: Record<string, string> = {
      Crypto: '#f59e0b',
      Politics: '#3b82f6',
      Sports: '#10b981',
      Entertainment: '#ec4899',
      Economics: '#8b5cf6',
      Science: '#14b8a6',
    };
    return colors[category] || '#6b7280';
  };

  const getActionColor = (action: string) => {
    if (action.includes('buy')) return '#10b981';
    if (action.includes('sell')) return '#ef4444';
    return '#6b7280';
  };

  return (
    <div className="prediction-markets-container">
      <div className="section-header">
        <h2>📊 Prediction Markets</h2>
        <p className="section-subtitle">
          Trade prediction markets with EV-based strategies and Kelly Criterion position sizing
        </p>
      </div>

      {/* Stats Row */}
      {stats && (
        <div className="stats-row">
          <div className="stat-card">
            <div className="stat-label">Total Markets</div>
            <div className="stat-value">{stats.total_markets}</div>
          </div>
          <div className="stat-card">
            <div className="stat-label">Active Markets</div>
            <div className="stat-value">{stats.active_markets}</div>
          </div>
          <div className="stat-card">
            <div className="stat-label">Total Liquidity</div>
            <div className="stat-value">{formatCurrency(stats.total_liquidity)}</div>
          </div>
          <div className="stat-card">
            <div className="stat-label">24h Volume</div>
            <div className="stat-value">{formatCurrency(stats.volume_24h)}</div>
          </div>
        </div>
      )}

      <div className="markets-content">
        {/* Markets List */}
        <div className="markets-list">
          <h3>Active Markets</h3>
          {markets.length === 0 ? (
            <p className="empty-state">No active markets found</p>
          ) : (
            markets.map((market) => (
              <div
                key={market.market_id}
                className={`market-card ${selectedMarket?.market_id === market.market_id ? 'selected' : ''}`}
                onClick={() => handleMarketClick(market)}
              >
                <div className="market-header">
                  <span
                    className="market-category"
                    style={{ backgroundColor: getCategoryColor(market.category) }}
                  >
                    {market.category}
                  </span>
                  <span className="market-status">{market.status}</span>
                </div>
                <div className="market-question">{market.question}</div>
                <div className="market-outcomes">
                  {market.outcomes.map((outcome) => (
                    <div key={outcome.outcome_id} className="outcome-row">
                      <span className="outcome-name">{outcome.name}</span>
                      <span className="outcome-price">{formatPercent(outcome.price)}</span>
                      <span className="outcome-change" style={{
                        color: outcome.price > outcome.last_price ? '#10b981' : '#ef4444'
                      }}>
                        {outcome.price > outcome.last_price ? '↑' : '↓'}
                        {formatPercent(Math.abs(outcome.price - outcome.last_price))}
                      </span>
                    </div>
                  ))}
                </div>
                <div className="market-stats">
                  <span>Liquidity: {formatCurrency(market.liquidity)}</span>
                  <span>Volume: {formatCurrency(market.volume_24h)}</span>
                  <span>Ends: {formatDate(market.end_date)}</span>
                </div>
              </div>
            ))
          )}
        </div>

        {/* Market Details & Signals */}
        <div className="market-details">
          {selectedMarket ? (
            <>
              <h3>Market Details & Signals</h3>
              <div className="detail-card">
                <h4>{selectedMarket.question}</h4>
                <div className="detail-info">
                  <div className="detail-row">
                    <span>Category:</span>
                    <span style={{ color: getCategoryColor(selectedMarket.category) }}>
                      {selectedMarket.category}
                    </span>
                  </div>
                  <div className="detail-row">
                    <span>Fee:</span>
                    <span>{(selectedMarket.fee_bps / 100).toFixed(2)}%</span>
                  </div>
                  <div className="detail-row">
                    <span>End Date:</span>
                    <span>{formatDate(selectedMarket.end_date)}</span>
                  </div>
                </div>

                {/* Outcomes with Trade Buttons */}
                <div className="outcomes-trading">
                  <h5>Outcomes</h5>
                  {selectedMarket.outcomes.map((outcome) => (
                    <div key={outcome.outcome_id} className="outcome-trade-card">
                      <div className="outcome-trade-info">
                        <div className="outcome-trade-name">{outcome.name}</div>
                        <div className="outcome-trade-price">
                          Price: {formatPercent(outcome.price)}
                        </div>
                        {outcome.bid && outcome.ask && (
                          <div className="outcome-spread">
                            Bid: {formatPercent(outcome.bid)} | Ask: {formatPercent(outcome.ask)}
                          </div>
                        )}
                      </div>
                      <div className="outcome-trade-actions">
                        <button
                          className="trade-btn buy-btn"
                          onClick={() => executeTrade(selectedMarket.market_id, outcome.outcome_id, 'buy_yes')}
                        >
                          Buy Yes
                        </button>
                        <button
                          className="trade-btn sell-btn"
                          onClick={() => executeTrade(selectedMarket.market_id, outcome.outcome_id, 'sell_yes')}
                        >
                          Sell Yes
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              </div>

              {/* Signals Section */}
              <div className="signals-section">
                <h4>Trading Signals</h4>
                {loading ? (
                  <p className="loading-state">Loading signals...</p>
                ) : error ? (
                  <p className="error-state">{error}</p>
                ) : signals.length === 0 ? (
                  <p className="empty-state">No signals found for this market</p>
                ) : (
                  signals.map((signal) => (
                    <div key={signal.signal_id} className="signal-card">
                      <div className="signal-header">
                        <span
                          className="signal-action"
                          style={{ backgroundColor: getActionColor(signal.action) }}
                        >
                          {signal.action.replace('_', ' ').toUpperCase()}
                        </span>
                        <span className="signal-confidence">
                          Confidence: {formatPercent(signal.confidence)}
                        </span>
                      </div>
                      <div className="signal-metrics">
                        <div className="signal-metric">
                          <span className="metric-label">Expected Value:</span>
                          <span className={`metric-value ${signal.expected_value > 0 ? 'positive' : 'negative'}`}>
                            {signal.expected_value > 0 ? '+' : ''}{formatPercent(signal.expected_value)}
                          </span>
                        </div>
                        <div className="signal-metric">
                          <span className="metric-label">Kelly Fraction:</span>
                          <span className="metric-value">{formatPercent(signal.kelly_fraction)}</span>
                        </div>
                        <div className="signal-metric">
                          <span className="metric-label">Target Price:</span>
                          <span className="metric-value">{formatPercent(signal.target_price)}</span>
                        </div>
                      </div>
                      <div className="signal-reasoning">
                        <strong>Reasoning:</strong> {signal.reasoning}
                      </div>
                      <div className="signal-timestamp">
                        {new Date(signal.timestamp * 1000).toLocaleString()}
                      </div>
                    </div>
                  ))
                )}
              </div>
            </>
          ) : (
            <div className="empty-selection">
              <p>Select a market to view details and trading signals</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default PredictionMarkets;
