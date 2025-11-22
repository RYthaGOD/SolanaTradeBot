import { useState, useEffect } from 'react'
import axios from 'axios'

interface AgentPerformance {
  agent_id: string
  total_trades: number
  successful_trades: number
  failed_trades: number
  win_rate: number
  avg_reward: number
  sharpe_ratio: number
  max_drawdown: number
  learning_rate: number
}

export default function RLAgents() {
  const [agents, setAgents] = useState<AgentPerformance[]>([])
  const [loading, setLoading] = useState(true)
  const [selectedAgent, setSelectedAgent] = useState<string | null>(null)

  useEffect(() => {
    fetchAgents()
    const interval = setInterval(fetchAgents, 5000)
    return () => clearInterval(interval)
  }, [])

  const fetchAgents = async () => {
    try {
      // Fetch RL agent learning metrics from API (learning from past trades only)
      const response = await axios.get('http://localhost:8080/rl/agents')
      
      if (response.data.success && Array.isArray(response.data.data)) {
        // Map API response to component format
        const agentsData: AgentPerformance[] = response.data.data.map((agent: any) => ({
          agent_id: agent.agent_id || '',
          total_trades: agent.total_trades || 0,
          successful_trades: agent.successful_trades || 0,
          failed_trades: agent.failed_trades || 0,
          win_rate: agent.win_rate || 0,
          avg_reward: agent.avg_reward || 0,
          sharpe_ratio: agent.sharpe_ratio || 0,
          max_drawdown: agent.max_drawdown || 0,
          learning_rate: agent.learning_rate || 0.01,
        }))
        
        setAgents(agentsData)
        setLoading(false)
      } else {
        // No agents registered yet - show empty state
        setAgents([])
        setLoading(false)
      }
    } catch (error: any) {
      console.error('Failed to fetch RL agents:', error)
      // If endpoint doesn't exist or coordinator not available, show empty state
      setAgents([])
      setLoading(false)
    }
  }

  const getStatusColor = (winRate: number) => {
    if (winRate >= 0.7) return '#00ff88'
    if (winRate >= 0.6) return '#ffaa00'
    return '#ff5500'
  }

  const getAgentName = (agentId: string) => {
    return agentId.replace(/_agent$/, '').replace(/_/g, ' ').split(' ')
      .map(word => word.charAt(0).toUpperCase() + word.slice(1))
      .join(' ')
  }

  const getAgentIcon = (agentId: string) => {
    if (agentId.includes('memecoin')) return '🎪'
    if (agentId.includes('oracle')) return '🔮'
    if (agentId.includes('perps')) return '📊'
    if (agentId.includes('opportunity')) return '💡'
    if (agentId.includes('signal')) return '📡'
    if (agentId.includes('master')) return '👑'
    return '🤖'
  }

  return (
    <div className="rl-agents-container">
      <div className="section-header">
        <h2 className="glow">🧠 Reinforcement Learning Agents</h2>
        <p>Self-improving agents learning from every trade outcome</p>
      </div>

      {loading ? (
        <div className="loading-spinner">⏳ Loading agent performance...</div>
      ) : (
        <>
          {/* Summary Stats */}
          <div className="stats-grid">
            <div className="stat-card">
              <div className="stat-icon">🤖</div>
              <div className="stat-value">{agents.length}</div>
              <div className="stat-label">Active Agents</div>
            </div>
            <div className="stat-card">
              <div className="stat-icon">📈</div>
              <div className="stat-value">
                {agents.reduce((sum, a) => sum + a.total_trades, 0)}
              </div>
              <div className="stat-label">Total Trades</div>
            </div>
            <div className="stat-card">
              <div className="stat-icon">🎯</div>
              <div className="stat-value">
                {(agents.reduce((sum, a) => sum + a.win_rate, 0) / agents.length * 100).toFixed(1)}%
              </div>
              <div className="stat-label">Avg Win Rate</div>
            </div>
            <div className="stat-card">
              <div className="stat-icon">⚡</div>
              <div className="stat-value">
                {(agents.reduce((sum, a) => sum + a.avg_reward, 0) / agents.length).toFixed(3)}
              </div>
              <div className="stat-label">Avg Reward</div>
            </div>
          </div>

          {/* Agent Cards */}
          <div className="agents-grid">
            {agents.map(agent => (
              <div 
                key={agent.agent_id}
                className={`agent-card hover-lift ${selectedAgent === agent.agent_id ? 'selected' : ''}`}
                onClick={() => setSelectedAgent(selectedAgent === agent.agent_id ? null : agent.agent_id)}
              >
                <div className="agent-header">
                  <div className="agent-icon">{getAgentIcon(agent.agent_id)}</div>
                  <div className="agent-info">
                    <h3>{getAgentName(agent.agent_id)}</h3>
                    <p className="agent-id">{agent.agent_id}</p>
                  </div>
                  <div 
                    className="agent-status"
                    style={{ background: getStatusColor(agent.win_rate) }}
                  ></div>
                </div>

                <div className="agent-metrics">
                  <div className="metric-row">
                    <span className="metric-label">Win Rate</span>
                    <div className="metric-bar-container">
                      <div 
                        className="metric-bar"
                        style={{ 
                          width: `${agent.win_rate * 100}%`,
                          background: getStatusColor(agent.win_rate)
                        }}
                      ></div>
                      <span className="metric-value">{(agent.win_rate * 100).toFixed(1)}%</span>
                    </div>
                  </div>

                  <div className="metric-row">
                    <span className="metric-label">Trades</span>
                    <span className="metric-value">
                      {agent.total_trades} ({agent.successful_trades}W/{agent.failed_trades}L)
                    </span>
                  </div>

                  <div className="metric-row">
                    <span className="metric-label">Avg Reward</span>
                    <span className={`metric-value ${agent.avg_reward > 0 ? 'positive' : 'negative'}`}>
                      {agent.avg_reward > 0 ? '+' : ''}{agent.avg_reward.toFixed(3)}
                    </span>
                  </div>

                  <div className="metric-row">
                    <span className="metric-label">Sharpe Ratio</span>
                    <span className="metric-value">{agent.sharpe_ratio.toFixed(2)}</span>
                  </div>

                  <div className="metric-row">
                    <span className="metric-label">Max Drawdown</span>
                    <span className="metric-value negative">
                      {(agent.max_drawdown * 100).toFixed(1)}%
                    </span>
                  </div>

                  <div className="metric-row">
                    <span className="metric-label">Learning Rate</span>
                    <span className="metric-value adaptive">
                      {agent.learning_rate.toFixed(4)} <span className="pulse">●</span>
                    </span>
                  </div>
                </div>

                {selectedAgent === agent.agent_id && (
                  <div className="agent-details fade-in">
                    <h4>Learning Progress</h4>
                    <p>This agent is continuously learning from trade outcomes. The learning rate adapts based on performance:</p>
                    <ul>
                      <li>✓ Win rate {agent.win_rate >= 0.6 ? '>' : '<'} 60%: Learning rate adjusting</li>
                      <li>✓ Experience replay: Optimizing from past trades</li>
                      <li>✓ Q-learning with epsilon-greedy exploration</li>
                    </ul>
                  </div>
                )}
              </div>
            ))}
          </div>

          {/* Learning Insights */}
          <div className="card insights-card">
            <div className="card-header">
              <h3>📚 Learning Insights</h3>
            </div>
            <div className="insights-content">
              <div className="insight">
                <span className="insight-icon">🎯</span>
                <div>
                  <h4>Adaptive Learning Rates</h4>
                  <p>Agents automatically adjust learning rates based on performance. High-performing agents decrease exploration, struggling agents increase it.</p>
                </div>
              </div>
              <div className="insight">
                <span className="insight-icon">🔄</span>
                <div>
                  <h4>Experience Replay</h4>
                  <p>Each agent maintains a replay buffer of past experiences, learning from both successes and failures to improve future decisions.</p>
                </div>
              </div>
              <div className="insight">
                <span className="insight-icon">🌐</span>
                <div>
                  <h4>Centralized Coordination</h4>
                  <p>All agents are connected to a learning coordinator that shares knowledge and coordinates strategies across the system.</p>
                </div>
              </div>
            </div>
          </div>
        </>
      )}
    </div>
  )
}
