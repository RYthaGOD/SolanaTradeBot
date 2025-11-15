import { useState, useEffect, lazy, Suspense } from 'react'
import { httpJson } from './utils/http'

const Dashboard = lazy(() => import('./components/Dashboard'))
const TradingView = lazy(() => import('./components/TradingView'))
const Portfolio = lazy(() => import('./components/Portfolio'))
const Performance = lazy(() => import('./components/Performance'))
const AIOrchestrator = lazy(() => import('./components/AIOrchestrator'))
const RLAgents = lazy(() => import('./components/RLAgents'))
const X402Marketplace = lazy(() => import('./components/X402Marketplace'))
const MemeAnalyzer = lazy(() => import('./components/MemeAnalyzer'))

function App() {
  const [activeTab, setActiveTab] = useState('dashboard')
  const [isConnected, setIsConnected] = useState(false)
  const [apiV2Connected, setApiV2Connected] = useState(false)
  const [systemStats, setSystemStats] = useState<any>(null)

  useEffect(() => {
    let isMounted = true

    const checkConnection = async () => {
      const [legacy, v2] = await Promise.allSettled([
        httpJson<{ success?: boolean }>('http://localhost:8080/health', { timeoutMs: 2000 }),
        httpJson<Record<string, unknown>>('http://localhost:8081/health', { timeoutMs: 2000 })
      ])

      if (!isMounted) return

      setIsConnected(legacy.status === 'fulfilled' ? Boolean(legacy.value?.success) : false)
      setApiV2Connected(v2.status === 'fulfilled')
    }
    
    const fetchSystemStats = async () => {
      try {
        const response = await httpJson<any>('http://localhost:8081/execute/system', {
          method: 'POST',
          data: {}
        })
        if (isMounted) {
          setSystemStats(response)
        }
      } catch {
        if (isMounted) {
          console.log('System stats not available')
        }
      }
    }
    
    checkConnection()
    fetchSystemStats()
    const interval = setInterval(() => {
      checkConnection()
      fetchSystemStats()
    }, 10000)
    return () => {
      isMounted = false
      clearInterval(interval)
    }
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
            <Suspense fallback={<div className="loading">Loading module...</div>}>
              {activeTab === 'dashboard' && <Dashboard />}
              {activeTab === 'ai' && <AIOrchestrator />}
              {activeTab === 'rl' && <RLAgents />}
              {activeTab === 'trading' && <TradingView />}
              {activeTab === 'portfolio' && <Portfolio />}
              {activeTab === 'performance' && <Performance />}
              {activeTab === 'x402' && <X402Marketplace />}
              {activeTab === 'meme' && <MemeAnalyzer />}
            </Suspense>
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
