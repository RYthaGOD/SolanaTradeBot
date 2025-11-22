import { useState, useEffect } from 'react'
import axios from 'axios'

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

interface DashboardProps {
  systemStats?: any
}

export default function Dashboard({ systemStats }: DashboardProps) {
  const [performance, setPerformance] = useState<PerformanceData | null>(null)
  const [marketData, setMarketData] = useState<MarketData[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [systemHealth, setSystemHealth] = useState<any>(null)
  const [tradingEnabled, setTradingEnabled] = useState<boolean>(true)
  const [toggling, setToggling] = useState<boolean>(false)

  useEffect(() => {
    fetchData()
    fetchTradingState()
    const interval = setInterval(fetchData, 5000)
    const tradingStateInterval = setInterval(fetchTradingState, 10000)
    return () => {
      clearInterval(interval)
      clearInterval(tradingStateInterval)
    }
  }, [])

  const fetchTradingState = async () => {
    try {
      const response = await axios.get('http://localhost:8080/trading-state')
      if (response.data.success) {
        setTradingEnabled(response.data.data.enabled)
      }
    } catch (err) {
      console.error('Failed to fetch trading state:', err)
    }
  }

  const toggleTrading = async () => {
    setToggling(true)
    try {
      const newState = !tradingEnabled
      const response = await axios.post('http://localhost:8080/trading-toggle', {
        enabled: newState
      })
      if (response.data.success) {
        setTradingEnabled(newState)
      }
    } catch (err) {
      console.error('Failed to toggle trading:', err)
      setError('Failed to toggle trading state')
    } finally {
      setToggling(false)
    }
  }

  const fetchData = async () => {
    try {
      const [perfRes, marketRes] = await Promise.all([
        axios.get('http://localhost:8080/performance'),
        axios.get('http://localhost:8080/market-data')
      ])

      // Fetch system health from AI Orchestrator
      try {
        const systemRes = await axios.post('http://localhost:8081/execute/system', {})
        setSystemHealth(systemRes.data)
      } catch {}

      if (perfRes.data.success) {
        setPerformance(perfRes.data.data)
      }
      if (marketRes.data.success) {
        setMarketData(marketRes.data.data)
      }
      setLoading(false)
      setError('')
    } catch (err) {
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
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '20px' }}>
        <h2 style={{ margin: 0 }}>Trading Dashboard</h2>
        <button
          onClick={toggleTrading}
          disabled={toggling}
          style={{
            padding: '12px 24px',
            fontSize: '16px',
            fontWeight: 'bold',
            borderRadius: '8px',
            border: 'none',
            cursor: toggling ? 'not-allowed' : 'pointer',
            backgroundColor: tradingEnabled ? '#c62828' : '#2e7d32',
            color: 'white',
            transition: 'all 0.3s ease',
            boxShadow: '0 2px 8px rgba(0,0,0,0.2)',
            opacity: toggling ? 0.6 : 1
          }}
          onMouseEnter={(e) => {
            if (!toggling) {
              e.currentTarget.style.transform = 'scale(1.05)'
            }
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.transform = 'scale(1)'
          }}
        >
          {toggling ? '⏳ Processing...' : tradingEnabled ? '🛑 Stop Trading' : '▶️ Start Trading'}
        </button>
      </div>

      <div style={{
        padding: '12px 16px',
        marginBottom: '20px',
        borderRadius: '8px',
        backgroundColor: tradingEnabled ? '#e8f5e9' : '#ffebee',
        border: `2px solid ${tradingEnabled ? '#2e7d32' : '#c62828'}`,
        color: tradingEnabled ? '#1b5e20' : '#b71c1c',
        fontWeight: 'bold',
        display: 'flex',
        alignItems: 'center',
        gap: '10px'
      }}>
        <span style={{ fontSize: '20px' }}>{tradingEnabled ? '🟢' : '🔴'}</span>
        <span>Trading is {tradingEnabled ? 'ENABLED' : 'DISABLED'}</span>
      </div>

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
    </div>
  )
}
