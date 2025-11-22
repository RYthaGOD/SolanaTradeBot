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
      // Fetch active signals from API
      const signalsResponse = await axios.get('http://localhost:8080/signals/marketplace/active')
      if (signalsResponse.data.success && Array.isArray(signalsResponse.data.data)) {
        const apiSignals = signalsResponse.data.data.map((sig: any) => ({
          id: sig.id || sig.signal_id || '',
          provider: sig.provider || sig.provider_id || '',
          symbol: sig.symbol || '',
          action: sig.action || 'BUY',
          entry_price: sig.entry_price || sig.price || 0,
          confidence: sig.confidence || 0.5,
          timeframe: sig.timeframe || '1h',
          price: sig.price || sig.signal_price || 0,
          timestamp: sig.timestamp || Date.now()
        }))
        setSignals(apiSignals)
      } else {
        // Fallback to empty array if no signals
        setSignals([])
      }

      // Fetch marketplace stats to get provider info
      try {
        const statsResponse = await axios.get('http://localhost:8080/signals/marketplace/stats')
        if (statsResponse.data.success && statsResponse.data.data) {
          const stats = statsResponse.data.data
          // Try to extract provider info from stats
          if (stats.providers && Array.isArray(stats.providers)) {
            setProviders(stats.providers)
          } else {
            // Fallback: create providers from signal data
            const providerMap = new Map<string, any>()
            apiSignals.forEach((sig: Signal) => {
              if (!providerMap.has(sig.provider)) {
                providerMap.set(sig.provider, {
                  id: sig.provider,
                  name: sig.provider.replace(/_/g, ' ').replace(/\b\w/g, l => l.toUpperCase()),
                  signals: 0,
                  win_rate: 0.65,
                  earnings: 0
                })
              }
              const provider = providerMap.get(sig.provider)!
              provider.signals++
            })
            setProviders(Array.from(providerMap.values()))
          }
        } else {
          // Fallback: create providers from signal data
          const providerMap = new Map<string, any>()
          apiSignals.forEach((sig: Signal) => {
            if (!providerMap.has(sig.provider)) {
              providerMap.set(sig.provider, {
                id: sig.provider,
                name: sig.provider.replace(/_/g, ' ').replace(/\b\w/g, l => l.toUpperCase()),
                signals: 0,
                win_rate: 0.65,
                earnings: 0
              })
            }
            const provider = providerMap.get(sig.provider)!
            provider.signals++
          })
          setProviders(Array.from(providerMap.values()))
        }
      } catch (err) {
        console.error('Failed to fetch provider stats:', err)
        // Fallback: create providers from signal data
        const providerMap = new Map<string, any>()
        apiSignals.forEach((sig: Signal) => {
          if (!providerMap.has(sig.provider)) {
            providerMap.set(sig.provider, {
              id: sig.provider,
              name: sig.provider.replace(/_/g, ' ').replace(/\b\w/g, l => l.toUpperCase()),
              signals: 0,
              win_rate: 0.65,
              earnings: 0
            })
          }
          const provider = providerMap.get(sig.provider)!
          provider.signals++
        })
        setProviders(Array.from(providerMap.values()))
      }
      
      setLoading(false)
    } catch (error) {
      console.error('Failed to fetch marketplace data:', error)
      // Set empty arrays on error
      setSignals([])
      setProviders([])
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
