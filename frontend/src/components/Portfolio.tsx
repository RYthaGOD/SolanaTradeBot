import { useState, useEffect } from 'react'
import { httpJson } from '../utils/http'
import { PieChart, Pie, Cell, ResponsiveContainer, Legend, Tooltip } from 'recharts'

interface PortfolioData {
  positions: Record<string, number>
  total_value: number
  cash: number
  daily_pnl: number
  total_pnl: number
}

const COLORS = ['#667eea', '#764ba2', '#f093fb', '#4facfe', '#43e97b']

export default function Portfolio() {
  const [portfolio, setPortfolio] = useState<PortfolioData | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')

  useEffect(() => {
    fetchPortfolio()
    const interval = setInterval(fetchPortfolio, 5000)
    return () => clearInterval(interval)
  }, [])

    const fetchPortfolio = async () => {
      try {
        const response = await httpJson<{ success: boolean; data: PortfolioData }>('http://localhost:8080/portfolio')
        if (response.success) {
          setPortfolio(response.data)
        }
        setLoading(false)
        setError('')
      } catch {
        setError('Failed to fetch portfolio data')
        setLoading(false)
      }
    }

  if (loading) {
    return <div className="loading">Loading portfolio...</div>
  }

  if (error) {
    return <div className="error">{error}</div>
  }

  const positions = portfolio?.positions || {}
  const chartData = Object.entries(positions).map(([symbol, size]) => ({
    name: symbol,
    value: typeof size === 'number' ? size : 0
  }))

  return (
    <div>
      <h2>Portfolio Overview</h2>

      <div className="stats-grid">
        <div className="stat-card">
          <div className="stat-label">Total Value</div>
          <div className="stat-value">${portfolio?.total_value?.toFixed(2) || '0.00'}</div>
        </div>
        <div className="stat-card">
          <div className="stat-label">Cash Balance</div>
          <div className="stat-value">${portfolio?.cash?.toFixed(2) || '0.00'}</div>
        </div>
        <div className="stat-card">
          <div className="stat-label">Today's P&L</div>
          <div className="stat-value" style={{ color: portfolio && portfolio.daily_pnl >= 0 ? '#4caf50' : '#f44336' }}>
            ${portfolio?.daily_pnl?.toFixed(2) || '0.00'}
          </div>
        </div>
        <div className="stat-card">
          <div className="stat-label">Total P&L</div>
          <div className="stat-value" style={{ color: portfolio && portfolio.total_pnl >= 0 ? '#4caf50' : '#f44336' }}>
            ${portfolio?.total_pnl?.toFixed(2) || '0.00'}
          </div>
        </div>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(400px, 1fr))', gap: '20px' }}>
        <div className="card">
          <h3>Holdings</h3>
          {Object.keys(positions).length === 0 ? (
            <p style={{ textAlign: 'center', padding: '40px', color: '#999' }}>
              No positions currently held
            </p>
          ) : (
            <table className="table">
              <thead>
                <tr>
                  <th>Asset</th>
                  <th>Size</th>
                  <th>Type</th>
                </tr>
              </thead>
              <tbody>
                {Object.entries(positions).map(([symbol, size]) => (
                  <tr key={symbol}>
                    <td><strong>{symbol}</strong></td>
                    <td>{typeof size === 'number' ? size.toFixed(4) : size}</td>
                    <td>{symbol === 'CASH' ? 'Fiat' : 'Crypto'}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>

        {chartData.length > 0 && (
          <div className="card">
            <h3>Portfolio Allocation</h3>
            <ResponsiveContainer width="100%" height={300}>
              <PieChart>
                <Pie
                  data={chartData}
                  cx="50%"
                  cy="50%"
                  labelLine={false}
                  label={({ name, percent }) => `${name}: ${(percent * 100).toFixed(0)}%`}
                  outerRadius={80}
                  fill="#8884d8"
                  dataKey="value"
                >
                  {chartData.map((_, index) => (
                    <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
                  ))}
                </Pie>
                <Tooltip />
                <Legend />
              </PieChart>
            </ResponsiveContainer>
          </div>
        )}
      </div>
    </div>
  )
}
