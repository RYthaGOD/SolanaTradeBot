import { useState, useEffect } from 'react'
import { httpJson } from '../utils/http'
import MobulaInsights from './MobulaInsights'

interface PerformanceData {
  total_return: number
  current_capital: number
  max_drawdown: number
  sharpe_ratio: number
  win_rate: number
  daily_pnl: number
  total_pnl: number
  trade_count: number
}

interface MarketData {
  symbol: string
  price: string
  change: string
  volume: string
}

export default function Dashboard() {
  const [performance, setPerformance] = useState<PerformanceData | null>(null)
  const [marketData, setMarketData] = useState<MarketData[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')

  useEffect(() => {
    fetchData()
    const interval = setInterval(fetchData, 5000)
    return () => clearInterval(interval)
  }, [])

    const fetchData = async () => {
      try {
        const [perfRes, marketRes] = await Promise.all([
          httpJson<{ success: boolean; data: PerformanceData }>('http://localhost:8080/performance'),
          httpJson<{ success: boolean; data: MarketData[] }>('http://localhost:8080/market-data')
        ])

        if (perfRes.success) {
          setPerformance(perfRes.data)
        }
        if (marketRes.success) {
          setMarketData(marketRes.data)
        }
        setLoading(false)
        setError('')
      } catch {
        setError('Failed to fetch data. Make sure the backend server is running.')
        setLoading(false)
      }
    }

  if (loading) {
    return <div className="loading">Loading dashboard...</div>
  }

  if (error) {
    return <div className="error">{error}</div>
  }

  return (
    <div>
      <h2>Trading Dashboard</h2>

      <div className="stats-grid">
        <div className="stat-card">
          <div className="stat-label">Portfolio Value</div>
          <div className="stat-value">${performance?.current_capital.toFixed(2) || '0.00'}</div>
        </div>
        <div className="stat-card">
          <div className="stat-label">Total Return</div>
          <div className="stat-value">{performance?.total_return.toFixed(2) || '0.00'}%</div>
        </div>
        <div className="stat-card">
          <div className="stat-label">Total P&L</div>
          <div className="stat-value">${performance?.total_pnl.toFixed(2) || '0.00'}</div>
        </div>
        <div className="stat-card">
          <div className="stat-label">Win Rate</div>
          <div className="stat-value">{performance?.win_rate.toFixed(1) || '0.0'}%</div>
        </div>
      </div>

      <div className="card">
        <h3>Market Overview</h3>
        <table className="table">
          <thead>
            <tr>
              <th>Symbol</th>
              <th>Price</th>
              <th>24h Change</th>
              <th>Volume</th>
            </tr>
          </thead>
          <tbody>
            {marketData.map((item, index) => (
              <tr key={index}>
                <td><strong>{item.symbol}</strong></td>
                <td>${item.price}</td>
                <td style={{ color: parseFloat(item.change) >= 0 ? '#2e7d32' : '#c62828' }}>
                  {parseFloat(item.change) >= 0 ? '+' : ''}{item.change}%
                </td>
                <td>${parseFloat(item.volume).toLocaleString()}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

        <div className="card">
          <h3>Quick Stats</h3>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '20px' }}>
            <div>
              <p style={{ color: '#666', marginBottom: '8px' }}>Sharpe Ratio</p>
              <p style={{ fontSize: '1.5rem', fontWeight: 'bold' }}>{performance?.sharpe_ratio.toFixed(2) || '0.00'}</p>
            </div>
            <div>
              <p style={{ color: '#666', marginBottom: '8px' }}>Max Drawdown</p>
              <p style={{ fontSize: '1.5rem', fontWeight: 'bold', color: '#c62828' }}>
                {performance?.max_drawdown.toFixed(2) || '0.00'}%
              </p>
            </div>
            <div>
              <p style={{ color: '#666', marginBottom: '8px' }}>Total Trades</p>
              <p style={{ fontSize: '1.5rem', fontWeight: 'bold' }}>{performance?.trade_count || 0}</p>
            </div>
            <div>
              <p style={{ color: '#666', marginBottom: '8px' }}>Daily P&L</p>
              <p style={{ fontSize: '1.5rem', fontWeight: 'bold', color: performance && performance.daily_pnl >= 0 ? '#2e7d32' : '#c62828' }}>
                ${performance?.daily_pnl.toFixed(2) || '0.00'}
              </p>
            </div>
          </div>
        </div>

        <MobulaInsights />
    </div>
  )
}
