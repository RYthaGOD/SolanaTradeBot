import { useState, useEffect } from 'react'
import { httpJson } from '../utils/http'

interface TradingSignal {
  symbol: string
  action: string
  confidence: string
  price: string
  size: string
}

export default function TradingView() {
  const [signals, setSignals] = useState<TradingSignal[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')

  useEffect(() => {
    fetchSignals()
    const interval = setInterval(fetchSignals, 3000)
    return () => clearInterval(interval)
  }, [])

    const fetchSignals = async () => {
      try {
        const response = await httpJson<{ success: boolean; data: TradingSignal[] }>('http://localhost:8080/signals')
        if (response.success) {
          setSignals(response.data)
        }
        setLoading(false)
        setError('')
      } catch {
        setError('Failed to fetch trading signals')
        setLoading(false)
      }
    }

  if (loading) {
    return <div className="loading">Loading trading signals...</div>
  }

  if (error) {
    return <div className="error">{error}</div>
  }

  return (
    <div>
      <h2>Trading Signals</h2>
      <p style={{ color: '#666', marginBottom: '30px' }}>
        Real-time AI-generated trading signals based on market analysis
      </p>

      <div className="card">
        <h3>Recent Signals</h3>
        {signals.length === 0 ? (
          <p style={{ textAlign: 'center', padding: '40px', color: '#999' }}>
            No trading signals generated yet. Waiting for market data...
          </p>
        ) : (
          <table className="table">
            <thead>
              <tr>
                <th>Symbol</th>
                <th>Action</th>
                <th>Confidence</th>
                <th>Price</th>
                <th>Size</th>
              </tr>
            </thead>
            <tbody>
              {signals.map((signal, index) => (
                <tr key={index}>
                  <td><strong>{signal.symbol}</strong></td>
                  <td>
                    <span className={`badge ${signal.action.toLowerCase()}`}>
                      {signal.action}
                    </span>
                  </td>
                  <td>{(parseFloat(signal.confidence) * 100).toFixed(0)}%</td>
                  <td>${signal.price}</td>
                  <td>{signal.size}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      <div className="card">
        <h3>Signal Generation Strategy</h3>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(250px, 1fr))', gap: '20px' }}>
          <div style={{ padding: '20px', background: '#f5f5f5', borderRadius: '8px' }}>
            <h4 style={{ marginBottom: '10px', color: '#667eea' }}>📊 Moving Averages</h4>
            <p style={{ color: '#666', fontSize: '0.9rem' }}>
              SMA-10 and SMA-20 crossover detection for trend identification
            </p>
          </div>
          <div style={{ padding: '20px', background: '#f5f5f5', borderRadius: '8px' }}>
            <h4 style={{ marginBottom: '10px', color: '#667eea' }}>🤖 ML Confidence</h4>
            <p style={{ color: '#666', fontSize: '0.9rem' }}>
              Machine learning model assigns confidence scores to each signal
            </p>
          </div>
          <div style={{ padding: '20px', background: '#f5f5f5', borderRadius: '8px' }}>
            <h4 style={{ marginBottom: '10px', color: '#667eea' }}>🛡️ Risk Management</h4>
            <p style={{ color: '#666', fontSize: '0.9rem' }}>
              Dynamic position sizing based on Kelly criterion and drawdown limits
            </p>
          </div>
        </div>
      </div>
    </div>
  )
}
