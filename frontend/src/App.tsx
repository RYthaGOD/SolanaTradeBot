import { useState, useEffect } from 'react'
import Dashboard from './components/Dashboard'
import TradingView from './components/TradingView'
import Portfolio from './components/Portfolio'
import Performance from './components/Performance'
import AIOrchestrator from './components/AIOrchestrator'
import RLAgents from './components/RLAgents'
import X402Marketplace from './components/X402Marketplace'
import MemeAnalyzer from './components/MemeAnalyzer'
import axios from 'axios'

function App() {
  const [activeTab, setActiveTab] = useState('dashboard')
  const [isConnected, setIsConnected] = useState(false)
  const [apiV2Connected, setApiV2Connected] = useState(false)
  const [systemStats, setSystemStats] = useState<any>(null)

  useEffect(() => {
    const checkConnection = async () => {
      try {
        // Check legacy API
        const response = await axios.get('http://localhost:8080/health', { timeout: 2000 })
        setIsConnected(response.data.success)
        
        // Check new AI Orchestrator API
        const v2Response = await axios.get('http://localhost:8081/health', { timeout: 2000 })
        setApiV2Connected(v2Response.status === 200)
      } catch {
        setIsConnected(false)
        setApiV2Connected(false)
      }
    }
    
    const fetchSystemStats = async () => {
      try {
        const response = await axios.post('http://localhost:8081/execute/system', {})
        setSystemStats(response.data)
      } catch (error) {
        console.log('System stats not available')
      }
    }
    
    checkConnection()
    fetchSystemStats()
    const interval = setInterval(() => {
      checkConnection()
      fetchSystemStats()
    }, 10000)
    return () => clearInterval(interval)
  }, [])

  const tabs = [
    { id: 'dashboard', icon: '📊', name: 'Dashboard', color: '#00d4ff' },
    { id: 'ai', icon: '🤖', name: 'AI Orchestrator', color: '#ff00ff' },
    { id: 'rl', icon: '🧠', name: 'RL Agents', color: '#00ff88' },
    { id: 'trading', icon: '🎯', name: 'Signals', color: '#ffaa00' },
    { id: 'portfolio', icon: '💼', name: 'Portfolio', color: '#00aaff' },
    { id: 'performance', icon: '📈', name: 'Performance', color: '#ff5500' },
    { id: 'x402', icon: '📡', name: 'X402 Market', color: '#aa00ff' },
    { id: 'meme', icon: '🎪', name: 'Meme Coins', color: '#ff0099' },
  ]

  return (
    <div className="app">
      <header className="header">
        <div className="header-content">
          <div className="brand">
            <h1 className="glow">⚡ SolanaTradeBot</h1>
            <p className="subtitle pulse">AI-Powered Autonomous Trading Platform</p>
          </div>
          
          <div className="header-stats">
            <div className="stat-card mini">
              <div className="stat-icon">🧠</div>
              <div className="stat-info">
                <div className="stat-label">RL Agents</div>
                <div className="stat-value">{systemStats?.rl_agents || 0}</div>
              </div>
            </div>
            <div className="stat-card mini">
              <div className="stat-icon">📡</div>
              <div className="stat-info">
                <div className="stat-label">X402 Signals</div>
                <div className="stat-value">{systemStats?.signals || 0}</div>
              </div>
            </div>
          </div>
          
          <div className="connection-panel">
            <div className={`connection-status ${isConnected ? 'connected' : 'offline'}`}>
              <div className={`status-dot ${isConnected ? 'connected' : 'offline'}`}></div>
              <span>{isConnected ? 'API v1' : 'Offline'}</span>
            </div>
            <div className={`connection-status ${apiV2Connected ? 'connected' : 'offline'}`}>
              <div className={`status-dot ${apiV2Connected ? 'connected' : 'offline'}`}></div>
              <span>{apiV2Connected ? 'AI API v2' : 'Offline'}</span>
            </div>
          </div>
        </div>
      </header>

      <nav className="nav-tabs modern">
        {tabs.map(tab => (
          <button 
            key={tab.id}
            className={activeTab === tab.id ? 'tab active' : 'tab'} 
            onClick={() => setActiveTab(tab.id)}
            style={{
              '--tab-color': tab.color
            } as any}
          >
            <span className="tab-icon">{tab.icon}</span>
            <span className="tab-name">{tab.name}</span>
            {activeTab === tab.id && <div className="tab-indicator"></div>}
          </button>
        ))}
      </nav>

      <main className="main-content modern">
        <div className="content-wrapper fade-in">
          {activeTab === 'dashboard' && <Dashboard systemStats={systemStats} />}
          {activeTab === 'ai' && <AIOrchestrator />}
          {activeTab === 'rl' && <RLAgents />}
          {activeTab === 'trading' && <TradingView />}
          {activeTab === 'portfolio' && <Portfolio />}
          {activeTab === 'performance' && <Performance />}
          {activeTab === 'x402' && <X402Marketplace />}
          {activeTab === 'meme' && <MemeAnalyzer />}
        </div>
      </main>

      <footer className="footer modern">
        <div className="footer-content">
          <div className="footer-section">
            <span className="footer-label">Integrations:</span>
            <span className="tech-badge">Switchboard Oracle</span>
            <span className="tech-badge">Jupiter DEX</span>
            <span className="tech-badge">DEX Screener</span>
            <span className="tech-badge">PumpFun</span>
          </div>
          <div className="footer-section">
            <span className="footer-label">AI:</span>
            <span className="tech-badge glow">DeepSeek LLM</span>
            <span className="tech-badge glow">6 Specialized Agents</span>
            <span className="tech-badge glow">RL Learning</span>
          </div>
          <div className="footer-section">
            <span className="footer-label">Protocol:</span>
            <span className="tech-badge pulse">X402 Signal Marketplace</span>
            <span className="tech-badge pulse">16 Atomic Operations</span>
          </div>
        </div>
      </footer>
    </div>
  )
}

export default App
