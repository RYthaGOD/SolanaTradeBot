import { useState, useEffect } from 'react'
import axios from 'axios'

interface MemeToken {
  symbol: string
  address: string
  price: number
  volume_24h: number
  price_change_24h: number
  market_cap: number
  liquidity: number
  risk_level: 'LOW' | 'MEDIUM' | 'HIGH' | 'EXTREME'
  confidence: number
  recommended_size: number
}

export default function MemeAnalyzer() {
  const [tokens, setTokens] = useState<MemeToken[]>([])
  const [loading, setLoading] = useState(true)
  const [analyzing, setAnalyzing] = useState(false)
  const [searchSymbol, setSearchSymbol] = useState('')
  const [analysisResult, setAnalysisResult] = useState<any>(null)

  useEffect(() => {
    fetchTopMemecoins()
  }, [])

  const fetchTopMemecoins = async () => {
    setLoading(true)
    try {
      // Simulated top meme coins (in production, would fetch from PumpFun/DEX Screener)
      setTokens([
        {
          symbol: 'BONK',
          address: '0x1234...5678',
          price: 0.000015,
          volume_24h: 45000000,
          price_change_24h: 12.5,
          market_cap: 850000000,
          liquidity: 12000000,
          risk_level: 'MEDIUM',
          confidence: 0.72,
          recommended_size: 500
        },
        {
          symbol: 'WIF',
          address: '0x2345...6789',
          price: 0.00035,
          volume_24h: 28000000,
          price_change_24h: -5.2,
          market_cap: 420000000,
          liquidity: 8500000,
          risk_level: 'HIGH',
          confidence: 0.58,
          recommended_size: 300
        },
        {
          symbol: 'PEPE',
          address: '0x3456...7890',
          price: 0.0000085,
          volume_24h: 95000000,
          price_change_24h: 8.7,
          market_cap: 1200000000,
          liquidity: 25000000,
          risk_level: 'LOW',
          confidence: 0.85,
          recommended_size: 1000
        },
        {
          symbol: 'MYRO',
          address: '0x4567...8901',
          price: 0.00028,
          volume_24h: 15000000,
          price_change_24h: 22.3,
          market_cap: 280000000,
          liquidity: 4200000,
          risk_level: 'HIGH',
          confidence: 0.65,
          recommended_size: 400
        },
        {
          symbol: 'SAMO',
          address: '0x5678...9012',
          price: 0.00042,
          volume_24h: 8500000,
          price_change_24h: -12.1,
          market_cap: 180000000,
          liquidity: 2800000,
          risk_level: 'EXTREME',
          confidence: 0.42,
          recommended_size: 150
        },
      ])
      setLoading(false)
    } catch (error) {
      console.error('Failed to fetch memecoins:', error)
      setLoading(false)
    }
  }

  const analyzeMeme = async () => {
    if (!searchSymbol.trim()) return
    
    setAnalyzing(true)
    setAnalysisResult(null)
    
    try {
      const response = await axios.post('http://localhost:8081/execute/predict', {
        symbol: searchSymbol,
        type: 'meme'
      })
      setAnalysisResult(response.data)
    } catch (error: any) {
      setAnalysisResult({ error: error.message })
    } finally {
      setAnalyzing(false)
    }
  }

  const getRiskColor = (risk: string) => {
    if (risk === 'LOW') return '#00ff88'
    if (risk === 'MEDIUM') return '#ffaa00'
    if (risk === 'HIGH') return '#ff5500'
    return '#ff0000'
  }

  const getConfidenceColor = (confidence: number) => {
    if (confidence >= 0.8) return '#00ff88'
    if (confidence >= 0.6) return '#ffaa00'
    return '#ff5500'
  }

  const formatNumber = (num: number, prefix = '') => {
    if (num >= 1000000000) return `${prefix}${(num / 1000000000).toFixed(2)}B`
    if (num >= 1000000) return `${prefix}${(num / 1000000).toFixed(2)}M`
    if (num >= 1000) return `${prefix}${(num / 1000).toFixed(2)}K`
    return `${prefix}${num.toFixed(2)}`
  }

  return (
    <div className="meme-analyzer-container">
      <div className="section-header">
        <h2 className="glow">🎪 Memecoin Analyzer</h2>
        <p>AI-powered safety checks and risk assessment for meme tokens</p>
      </div>

      {/* Analysis Input */}
      <div className="card analysis-card">
        <div className="card-header">
          <h3>🔍 Analyze Memecoin</h3>
        </div>
        <div className="analysis-input-group">
          <input
            type="text"
            value={searchSymbol}
            onChange={(e) => setSearchSymbol(e.target.value.toUpperCase())}
            placeholder="Enter token symbol (e.g., BONK)"
            className="input-field glow-input"
            onKeyPress={(e) => e.key === 'Enter' && analyzeMeme()}
          />
          <button 
            onClick={analyzeMeme}
            disabled={analyzing || !searchSymbol.trim()}
            className="btn primary glow-btn"
          >
            {analyzing ? '⏳ Analyzing...' : '🔬 Analyze'}
          </button>
        </div>

        {analysisResult && (
          <div className="analysis-result fade-in">
            <pre className="result-output">
              {JSON.stringify(analysisResult, null, 2)}
            </pre>
          </div>
        )}
      </div>

      {/* Top Memecoins */}
      {loading ? (
        <div className="loading-spinner">⏳ Loading memecoins...</div>
      ) : (
        <>
          <div className="section-title">
            <h3>🔥 Trending Memecoins</h3>
            <button onClick={fetchTopMemecoins} className="btn secondary small">
              🔄 Refresh
            </button>
          </div>

          <div className="meme-tokens-grid">
            {tokens.map(token => (
              <div key={token.symbol} className="meme-token-card hover-lift">
                <div className="token-header">
                  <div className="token-symbol">
                    <span className="symbol-name">{token.symbol}</span>
                    <span 
                      className="risk-badge"
                      style={{ background: getRiskColor(token.risk_level) }}
                    >
                      {token.risk_level} RISK
                    </span>
                  </div>
                  <div 
                    className={`price-change ${token.price_change_24h >= 0 ? 'positive' : 'negative'}`}
                  >
                    {token.price_change_24h >= 0 ? '▲' : '▼'} {Math.abs(token.price_change_24h).toFixed(2)}%
                  </div>
                </div>

                <div className="token-metrics">
                  <div className="metric">
                    <span className="metric-label">Price</span>
                    <span className="metric-value">${token.price.toFixed(8)}</span>
                  </div>
                  <div className="metric">
                    <span className="metric-label">Market Cap</span>
                    <span className="metric-value">{formatNumber(token.market_cap, '$')}</span>
                  </div>
                  <div className="metric">
                    <span className="metric-label">24h Volume</span>
                    <span className="metric-value">{formatNumber(token.volume_24h, '$')}</span>
                  </div>
                  <div className="metric">
                    <span className="metric-label">Liquidity</span>
                    <span className="metric-value">{formatNumber(token.liquidity, '$')}</span>
                  </div>
                </div>

                <div className="token-analysis">
                  <div className="confidence-section">
                    <span className="confidence-label">AI Confidence</span>
                    <div className="confidence-bar-container">
                      <div 
                        className="confidence-bar"
                        style={{ 
                          width: `${token.confidence * 100}%`,
                          background: getConfidenceColor(token.confidence)
                        }}
                      ></div>
                      <span className="confidence-value">{(token.confidence * 100).toFixed(0)}%</span>
                    </div>
                  </div>

                  <div className="position-sizing">
                    <span className="sizing-label">Recommended Size</span>
                    <span className="sizing-value">${token.recommended_size}</span>
                  </div>
                </div>

                <div className="token-footer">
                  <button 
                    className="btn primary small"
                    onClick={() => {
                      setSearchSymbol(token.symbol)
                      analyzeMeme()
                    }}
                  >
                    🔬 Deep Analysis
                  </button>
                  <button className="btn secondary small">📊 Chart</button>
                </div>

                <div className="token-address">
                  {token.address}
                </div>
              </div>
            ))}
          </div>

          {/* Safety Guidelines */}
          <div className="card safety-card">
            <div className="card-header">
              <h3>⚠️ Memecoin Safety Guidelines</h3>
            </div>
            <div className="safety-content">
              <div className="safety-rule">
                <span className="rule-icon">🛡️</span>
                <div className="rule-text">
                  <h4>Risk Assessment</h4>
                  <p>Our AI analyzes liquidity, volume, holder distribution, and social sentiment to assess risk levels.</p>
                </div>
              </div>
              <div className="safety-rule">
                <span className="rule-icon">💰</span>
                <div className="rule-text">
                  <h4>Position Sizing</h4>
                  <p>Recommended position sizes are calculated based on risk level and market conditions. Never exceed recommendations.</p>
                </div>
              </div>
              <div className="safety-rule">
                <span className="rule-icon">📊</span>
                <div className="rule-text">
                  <h4>Confidence Scores</h4>
                  <p>Confidence scores reflect the AI's certainty in the analysis. Lower scores indicate higher uncertainty.</p>
                </div>
              </div>
              <div className="safety-rule">
                <span className="rule-icon">⚡</span>
                <div className="rule-text">
                  <h4>Fast Moving Markets</h4>
                  <p>Memecoins are highly volatile. Always use stop losses and never invest more than you can afford to lose.</p>
                </div>
              </div>
            </div>
          </div>
        </>
      )}
    </div>
  )
}
