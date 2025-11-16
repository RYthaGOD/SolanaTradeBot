import { useEffect, useState } from 'react'
import { httpJson } from '../utils/http'

interface ApiResponse<T> {
  success: boolean
  data: T
  message: string
}

interface AgentPerformance {
  total_trades: number
  successful_trades: number
  failed_trades: number
  total_profit: number
  total_loss: number
  avg_reward: number
  win_rate: number
  sharpe_ratio: number
  max_drawdown: number
  learning_rate: number
}

interface AgentSnapshot {
  agent_id: string
  provider_type: string
  epsilon: number
  performance: AgentPerformance
}

type PerformanceMap = Record<string, AgentPerformance>

const formatPercent = (value: number) => `${(value * 100).toFixed(1)}%`

export default function RLDashboard() {
  const [performance, setPerformance] = useState<PerformanceMap>({})
  const [snapshots, setSnapshots] = useState<AgentSnapshot[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')

  useEffect(() => {
    const loadData = async () => {
      setLoading(true)
      setError('')
      try {
        const [perfRes, snapshotRes] = await Promise.all([
          httpJson<ApiResponse<PerformanceMap>>('http://localhost:8080/rl/performance'),
          httpJson<ApiResponse<AgentSnapshot[]>>('http://localhost:8080/rl/providers')
        ])
        setPerformance(perfRes.data)
        setSnapshots(snapshotRes.data)
      } catch (err) {
        console.error('Failed to load RL data', err)
        setError('Unable to load RL telemetry. Confirm the backend service is running.')
      } finally {
        setLoading(false)
      }
    }

    loadData()
    const interval = setInterval(loadData, 20_000)
    return () => clearInterval(interval)
  }, [])

  const agentCount = Object.keys(performance).length
  const aggregateWinRate =
    agentCount === 0
      ? 0
      : Object.values(performance).reduce((sum, perf) => sum + perf.win_rate, 0) / agentCount

  return (
    <div className="card">
      <div className="card-header" style={{ marginBottom: '16px' }}>
        <div>
          <h3>RL Telemetry</h3>
          <p style={{ color: 'var(--muted)' }}>Epsilon decay, reward curves, and win rates across autonomous agents</p>
        </div>
      </div>

      {error && <div className="error" style={{ marginBottom: '16px' }}>{error}</div>}
      {loading && <div className="loading">Syncing reinforcement learning metrics…</div>}

      {!loading && (
        <>
          <div className="rl-grid">
            <div className="rl-card">
              <span>Active Agents</span>
              <strong>{agentCount}</strong>
            </div>
            <div className="rl-card">
              <span>Avg Win Rate</span>
              <strong>{formatPercent(aggregateWinRate)}</strong>
            </div>
            <div className="rl-card">
              <span>Avg Sharpe Ratio</span>
              <strong>
                {agentCount === 0
                  ? '0.00'
                  : (
                      Object.values(performance).reduce((sum, perf) => sum + perf.sharpe_ratio, 0) /
                      agentCount
                    ).toFixed(2)}
              </strong>
            </div>
            <div className="rl-card">
              <span>Exploration (ε)</span>
              <strong>
                {snapshots.length === 0
                  ? '0.00'
                  : (
                      snapshots.reduce((sum, snap) => sum + snap.epsilon, 0) / snapshots.length
                    ).toFixed(3)}
              </strong>
            </div>
          </div>

          <div className="rl-table-wrapper">
            <table className="table">
              <thead>
                <tr>
                  <th>Agent</th>
                  <th>Provider</th>
                  <th>Trades</th>
                  <th>Win Rate</th>
                  <th>Sharpe</th>
                  <th>Avg Reward</th>
                  <th>Epsilon</th>
                </tr>
              </thead>
              <tbody>
                {snapshots.map((snapshot) => (
                  <tr key={snapshot.agent_id}>
                    <td>{snapshot.agent_id}</td>
                    <td>{snapshot.provider_type}</td>
                    <td>{snapshot.performance.total_trades}</td>
                    <td>{formatPercent(snapshot.performance.win_rate)}</td>
                    <td>{snapshot.performance.sharpe_ratio.toFixed(2)}</td>
                    <td>{snapshot.performance.avg_reward.toFixed(3)}</td>
                    <td>{snapshot.epsilon.toFixed(3)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
    </div>
  )
}
