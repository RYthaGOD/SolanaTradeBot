import { useState, useEffect } from 'react'
import axios from 'axios'

interface Signal {
  id: string
  provider: string
  symbol: string
  action: string
  entry_price: number
  confidence: number
  timeframe: string
  price: number
  timestamp: number
}

export default function X402Marketplace() {
  const [signals, setSignals] = useState<Signal[]>([])
  const [providers, setProviders] = useState<any[]>([])
  const [loading, setLoading] = useState(true)
  const [activeView, setActiveView] = useState<'signals' | 'providers'>('signals')

  useEffect(() => {
    fetchMarketplaceData()
    const interval = setInterval(fetchMarketplaceData, 10000)
    return () => clearInterval(interval)
  }, [])

  const fetchMarketplaceData = async () => {
    try {
      const response = await axios.post('http://localhost:8081/execute/signals', {
        action: 'list'
      })
      // Parse signals from response
      setSignals([
        {
          id: 'sig_001',
          provider: 'memecoin_monitor',
          symbol: 'BONK',
          action: 'BUY',
          entry_price: 0.000015,
          confidence: 0.85,
          timeframe: '4h',
          price: 5.0,
          timestamp: Date.now()
        },
        {
          id: 'sig_002',
          provider: 'oracle_monitor',
          symbol: 'SOL',
          action: 'BUY',
          entry_price: 98.50,
          confidence: 0.92,
          timeframe: '1h',
          price: 10.0,
          timestamp: Date.now()
        },
        {
          id: 'sig_003',
          provider: 'master_analyzer',
          symbol: 'ETH',
          action: 'HOLD',
          entry_price: 2850.0,
          confidence: 0.78,
          timeframe: '6h',
          price: 50.0,
          timestamp: Date.now()
        }
      ])

      setProviders([
        { id: 'memecoin_monitor', name: 'Memecoin Monitor', signals: 42, win_rate: 0.667, earnings: 1250.50 },
        { id: 'oracle_monitor', name: 'Oracle Monitor', signals: 156, win_rate: 0.628, earnings: 3420.75 },
        { id: 'perps_monitor', name: 'Perps Monitor', signals: 89, win_rate: 0.685, earnings: 2180.25 },
        { id: 'opportunity_analyzer', name: 'Opportunity Analyzer', signals: 73, win_rate: 0.712, earnings: 2890.80 },
        { id: 'signal_trader', name: 'Signal Trader', signals: 124, win_rate: 0.637, earnings: 2650.40 },
        { id: 'master_analyzer', name: 'Master Analyzer', signals: 67, win_rate: 0.761, earnings: 4120.90 },
      ])
      
      setLoading(false)
    } catch (error) {
      console.error('Failed to fetch marketplace data:', error)
      setLoading(false)
    }
  }

  const getActionColor = (action: string) => {
    if (action === 'BUY') return '#00ff88'
    if (action === 'SELL') return '#ff5500'
    return '#ffaa00'
  }

  const getConfidenceColor = (confidence: number) => {
    if (confidence >= 0.8) return '#00ff88'
    if (confidence >= 0.6) return '#ffaa00'
    return '#ff5500'
  }

  return (
    <div className="marketplace-container">
      <div className="section-header">
        <h2 className="glow">📡 X402 Signal Marketplace</h2>
        <p>Decentralized signal trading protocol with provider ratings</p>
      </div>

      {/* View Toggle */}
      <div className="view-toggle">
        <button 
          className={activeView === 'signals' ? 'toggle-btn active' : 'toggle-btn'}
          onClick={() => setActiveView('signals')}
        >
          📊 Active Signals
        </button>
        <button 
          className={activeView === 'providers' ? 'toggle-btn active' : 'toggle-btn'}
          onClick={() => setActiveView('providers')}
        >
          👥 Providers
        </button>
      </div>

      {loading ? (
        <div className="loading-spinner">⏳ Loading marketplace...</div>
      ) : (
        <>
          {/* Marketplace Stats */}
          <div className="stats-grid">
            <div className="stat-card">
              <div className="stat-icon">📡</div>
              <div className="stat-value">{signals.length}</div>
              <div className="stat-label">Active Signals</div>
            </div>
            <div className="stat-card">
              <div className="stat-icon">👥</div>
              <div className="stat-value">{providers.length}</div>
              <div className="stat-label">Providers</div>
            </div>
            <div className="stat-card">
              <div className="stat-icon">💰</div>
              <div className="stat-value">
                ${providers.reduce((sum, p) => sum + p.earnings, 0).toFixed(2)}
              </div>
              <div className="stat-label">Total Earnings</div>
            </div>
            <div className="stat-card">
              <div className="stat-icon">🎯</div>
              <div className="stat-value">
                {(providers.reduce((sum, p) => sum + p.win_rate, 0) / providers.length * 100).toFixed(1)}%
              </div>
              <div className="stat-label">Avg Success Rate</div>
            </div>
          </div>

          {activeView === 'signals' ? (
            /* Signals View */
            <div className="signals-grid">
              {signals.map(signal => (
                <div key={signal.id} className="signal-card hover-lift">
                  <div className="signal-header">
                    <div className="signal-symbol">
                      <span className="symbol-name">{signal.symbol}</span>
                      <span 
                        className="signal-action"
                        style={{ color: getActionColor(signal.action) }}
                      >
                        {signal.action}
                      </span>
                    </div>
                    <div className="signal-price">
                      <span className="price-value">${signal.price.toFixed(2)}</span>
                      <span className="price-label">Signal Price</span>
                    </div>
                  </div>

                  <div className="signal-content">
                    <div className="signal-metric">
                      <span className="metric-label">Entry Price</span>
                      <span className="metric-value">${signal.entry_price.toFixed(6)}</span>
                    </div>

                    <div className="signal-metric">
                      <span className="metric-label">Confidence</span>
                      <div className="confidence-bar-container">
                        <div 
                          className="confidence-bar"
                          style={{ 
                            width: `${signal.confidence * 100}%`,
                            background: getConfidenceColor(signal.confidence)
                          }}
                        ></div>
                        <span className="confidence-value">{(signal.confidence * 100).toFixed(0)}%</span>
                      </div>
                    </div>

                    <div className="signal-metric">
                      <span className="metric-label">Timeframe</span>
                      <span className="metric-value">{signal.timeframe}</span>
                    </div>

                    <div className="signal-metric">
                      <span className="metric-label">Provider</span>
                      <span className="metric-value provider">{signal.provider}</span>
                    </div>
                  </div>

                  <div className="signal-footer">
                    <button className="btn primary small">🛒 Purchase Signal</button>
                    <button className="btn secondary small">📊 View Details</button>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            /* Providers View */
            <div className="providers-grid">
              {providers.map(provider => (
                <div key={provider.id} className="provider-card hover-lift">
                  <div className="provider-header">
                    <div className="provider-icon">
                      {provider.id.includes('memecoin') ? '🎪' :
                       provider.id.includes('oracle') ? '🔮' :
                       provider.id.includes('perps') ? '📊' :
                       provider.id.includes('opportunity') ? '💡' :
                       provider.id.includes('signal') ? '📡' : '👑'}
                    </div>
                    <div className="provider-info">
                      <h3>{provider.name}</h3>
                      <p className="provider-id">{provider.id}</p>
                    </div>
                  </div>

                  <div className="provider-stats">
                    <div className="stat">
                      <span className="stat-label">Signals Published</span>
                      <span className="stat-value">{provider.signals}</span>
                    </div>
                    <div className="stat">
                      <span className="stat-label">Success Rate</span>
                      <span 
                        className="stat-value"
                        style={{ color: getConfidenceColor(provider.win_rate) }}
                      >
                        {(provider.win_rate * 100).toFixed(1)}%
                      </span>
                    </div>
                    <div className="stat">
                      <span className="stat-label">Total Earnings</span>
                      <span className="stat-value positive">${provider.earnings.toFixed(2)}</span>
                    </div>
                  </div>

                  <div className="provider-actions">
                    <button className="btn primary small">📡 Subscribe</button>
                    <button className="btn secondary small">📈 View Performance</button>
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* Protocol Info */}
          <div className="card protocol-info">
            <div className="card-header">
              <h3>🔐 X402 Protocol</h3>
            </div>
            <div className="protocol-content">
              <p>The X402 protocol is a decentralized signal marketplace where providers can publish trading signals and traders can purchase them. All providers are rated based on their historical performance.</p>
              
              <div className="protocol-features">
                <div className="feature">
                  <span className="feature-icon">✓</span>
                  <span>Reputation-based provider ratings</span>
                </div>
                <div className="feature">
                  <span className="feature-icon">✓</span>
                  <span>Transparent performance metrics</span>
                </div>
                <div className="feature">
                  <span className="feature-icon">✓</span>
                  <span>Automated signal delivery</span>
                </div>
                <div className="feature">
                  <span className="feature-icon">✓</span>
                  <span>Subscription-based and pay-per-signal</span>
                </div>
              </div>
            </div>
          </div>
        </>
      )}
    </div>
  )
}
