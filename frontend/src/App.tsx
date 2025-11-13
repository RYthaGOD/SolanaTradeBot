import { useState, useEffect } from 'react'
import Dashboard from './components/Dashboard'
import TradingView from './components/TradingView'
import Portfolio from './components/Portfolio'
import Performance from './components/Performance'
import axios from 'axios'

function App() {
  const [activeTab, setActiveTab] = useState('dashboard')
  const [isConnected, setIsConnected] = useState(false)

  useEffect(() => {
    const checkConnection = async () => {
      try {
        const response = await axios.get('http://localhost:8080/health', { timeout: 2000 })
        setIsConnected(response.data.success)
      } catch {
        setIsConnected(false)
      }
    }
    
    checkConnection()
    const interval = setInterval(checkConnection, 10000)
    return () => clearInterval(interval)
  }, [])

  return (
    <div className="app">
      <header className="header">
        <h1>⚡ SolanaTradeBot</h1>
        <p className="subtitle">Next-Gen AI Trading Platform</p>
        <div className={`connection-status ${isConnected ? '' : 'offline'}`}>
          <div className={`status-dot ${isConnected ? '' : 'offline'}`}></div>
          {isConnected ? 'Backend Connected' : 'Backend Offline'}
        </div>
      </header>

      <nav className="nav-tabs">
        <button className={activeTab === 'dashboard' ? 'tab active' : 'tab'} onClick={() => setActiveTab('dashboard')}>
          📊 Dashboard
        </button>
        <button className={activeTab === 'trading' ? 'tab active' : 'tab'} onClick={() => setActiveTab('trading')}>
          🎯 Signals
        </button>
        <button className={activeTab === 'portfolio' ? 'tab active' : 'tab'} onClick={() => setActiveTab('portfolio')}>
          💼 Portfolio
        </button>
        <button className={activeTab === 'performance' ? 'tab active' : 'tab'} onClick={() => setActiveTab('performance')}>
          📈 Performance
        </button>
      </nav>

      <main className="main-content">
        {activeTab === 'dashboard' && <Dashboard />}
        {activeTab === 'trading' && <TradingView />}
        {activeTab === 'portfolio' && <Portfolio />}
        {activeTab === 'performance' && <Performance />}
      </main>

      <footer className="footer">
        <p>🚀 Powered by Switchboard Oracle, DEX Screener, PumpFun & X402 Protocol | ⚡ 6 AI Agents | 🧠 DeepSeek LLM</p>
      </footer>
    </div>
  )
}

export default App
