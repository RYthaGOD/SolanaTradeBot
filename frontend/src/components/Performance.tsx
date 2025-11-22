import { useState, useEffect } from 'react'
import axios from 'axios'
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts'

interface PerformanceMetrics {
  total_return: number
  current_capital: number
  max_drawdown: number
  sharpe_ratio: number
  win_rate: number
  daily_pnl: number
  total_pnl: number
  trade_count: number
}

export default function Performance() {
  const [metrics, setMetrics] = useState<PerformanceMetrics | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')

  useEffect(() => {
    fetchPerformance()
    const interval = setInterval(fetchPerformance, 5000)
    return () => clearInterval(interval)
  }, [])

  const fetchPerformance = async () => {
    try {
      const response = await axios.get('http://localhost:8080/performance')
      if (response.data.success) {
        setMetrics(response.data.data)
      }
      setLoading(false)
      setError('')
    } catch (err) {
      setError('Failed to fetch performance data')
      setLoading(false)
    }
  }

  if (loading) {
    return <div className="loading">Loading performance metrics...</div>
  }

  if (error) {
    return <div className="error">{error}</div>
  }

  const chartData = [
    {
      name: 'Returns',
      value: metrics?.total_return || 0
    },
    {
      name: 'Win Rate',
      value: metrics?.win_rate || 0
    },
    {
      name: 'Sharpe Ratio',
      value: (metrics?.sharpe_ratio || 0) * 20
    }
  ]

  return (
    <div>
      <h2>Performance Analytics</h2>

      <div className="stats-grid">
        <div className="stat-card">
          <div className="stat-label">Total Return</div>
          <div className="stat-value">{metrics?.total_return.toFixed(2) || '0.00'}%</div>
        </div>
        <div className="stat-card">
          <div className="stat-label">Sharpe Ratio</div>
          <div className="stat-value">{metrics?.sharpe_ratio.toFixed(2) || '0.00'}</div>
        </div>
        <div className="stat-card">
          <div className="stat-label">Win Rate</div>
          <div className="stat-value">{metrics?.win_rate.toFixed(1) || '0.0'}%</div>
        </div>
        <div className="stat-card">
          <div className="stat-label">Max Drawdown</div>
          <div className="stat-value">{metrics?.max_drawdown.toFixed(2) || '0.00'}%</div>
        </div>
      </div>

      <div className="card">
        <h3>Performance Metrics</h3>
        <ResponsiveContainer width="100%" height={300}>
          <BarChart data={chartData}>
            <CartesianGrid strokeDasharray="3 3" />
            <XAxis dataKey="name" />
            <YAxis />
            <Tooltip />
            <Legend />
            <Bar dataKey="value" fill="#667eea" />
          </BarChart>
        </ResponsiveContainer>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(300px, 1fr))', gap: '20px' }}>
        <div className="card">
          <h3>Trading Statistics</h3>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '15px' }}>
            <div>
              <p style={{ color: '#666', marginBottom: '5px' }}>Total Trades</p>
              <p style={{ fontSize: '1.5rem', fontWeight: 'bold' }}>{metrics?.trade_count || 0}</p>
            </div>
            <div>
              <p style={{ color: '#666', marginBottom: '5px' }}>Total P&L</p>
              <p style={{ fontSize: '1.5rem', fontWeight: 'bold', color: metrics && metrics.total_pnl >= 0 ? '#2e7d32' : '#c62828' }}>
                ${metrics?.total_pnl.toFixed(2) || '0.00'}
              </p>
            </div>
            <div>
              <p style={{ color: '#666', marginBottom: '5px' }}>Daily P&L</p>
              <p style={{ fontSize: '1.5rem', fontWeight: 'bold', color: metrics && metrics.daily_pnl >= 0 ? '#2e7d32' : '#c62828' }}>
                ${metrics?.daily_pnl.toFixed(2) || '0.00'}
              </p>
            </div>
          </div>
        </div>

        <div className="card">
          <h3>Risk Metrics</h3>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '15px' }}>
            <div>
              <p style={{ color: '#666', marginBottom: '5px' }}>Current Capital</p>
              <p style={{ fontSize: '1.5rem', fontWeight: 'bold' }}>${metrics?.current_capital.toFixed(2) || '0.00'}</p>
            </div>
            <div>
              <p style={{ color: '#666', marginBottom: '5px' }}>Max Drawdown</p>
              <p style={{ fontSize: '1.5rem', fontWeight: 'bold', color: '#c62828' }}>
                {metrics?.max_drawdown.toFixed(2) || '0.00'}%
              </p>
            </div>
            <div>
              <p style={{ color: '#666', marginBottom: '5px' }}>Risk-Adjusted Return</p>
              <p style={{ fontSize: '1.5rem', fontWeight: 'bold', color: '#2e7d32' }}>
                {metrics?.sharpe_ratio.toFixed(2) || '0.00'}
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
