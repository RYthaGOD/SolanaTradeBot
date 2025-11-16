import { useState, useEffect } from 'react'
import { httpJson } from '../utils/http'

interface MemeTradeSignal {
  token_address: string
  symbol: string
  name: string
  action: string
  confidence: number
  entry_price: number
  target_price: number
  stop_loss: number
  reasons: string[]
  timestamp: number
}

export default function MemeAnalyzer() {
  const [signals, setSignals] = useState<MemeTradeSignal[]>([])
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
      const response = await httpJson<{ success: boolean; data: MemeTradeSignal[] }>('http://localhost:8080/pumpfun/signals')
      setSignals(response.data)
    } catch (error) {
      console.error('Failed to fetch memecoins:', error)
    } finally {
      setLoading(false)
    }
  }

  const analyzeMeme = async () => {
    if (!searchSymbol.trim()) return

    setAnalyzing(true)
    setAnalysisResult(null)

    try {
      const response = await httpJson<any>('http://localhost:8081/execute/predict', {
        method: 'POST',
        data: {
          symbol: searchSymbol,
          type: 'meme'
        }
      })
      setAnalysisResult(response)
    } catch (error: any) {
      setAnalysisResult({ error: error.message })
    } finally {
      setAnalyzing(false)
    }
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
        <h3>🔥 Live PumpFun Signals</h3>
        <button onClick={fetchTopMemecoins} className="btn secondary small">
          🔄 Refresh
        </button>
      </div>

      <div className="meme-tokens-grid">
        {signals.map(signal => (
          <div key={signal.token_address} className="meme-token-card hover-lift">
            <div className="token-header">
              <div className="token-symbol">
                <span className="symbol-name">{signal.symbol}</span>
                <span
                  className="risk-badge"
                  style={{ background: signal.action === 'BUY' ? '#00ff88' : '#ffaa00' }}
                >
                  {signal.action}
                </span>
              </div>
              <div className="price-change positive">
                Confidence {(signal.confidence * 100).toFixed(0)}%
              </div>
            </div>

            <div className="token-metrics">
              <div className="metric">
                <span className="metric-label">Entry</span>
                <span className="metric-value">${signal.entry_price.toFixed(8)}</span>
              </div>
              <div className="metric">
                <span className="metric-label">Target</span>
                <span className="metric-value">${signal.target_price.toFixed(8)}</span>
              </div>
              <div className="metric">
                <span className="metric-label">Stop</span>
                <span className="metric-value">${signal.stop_loss.toFixed(8)}</span>
              </div>
            </div>

            {signal.reasons.length > 0 && (
              <ul className="signal-list">
                {signal.reasons.slice(0, 3).map((reason, idx) => (
                  <li key={`${signal.token_address}-${idx}`}>{reason}</li>
                ))}
              </ul>
            )}
            <div className="token-address">{signal.token_address}</div>
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
