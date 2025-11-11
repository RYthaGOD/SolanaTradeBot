import { useState, useEffect } from 'react'
import Dashboard from './components/Dashboard'
import TradingView from './components/TradingView'
import Portfolio from './components/Portfolio'
import Performance from './components/Performance'

function App() {
  const [activeTab, setActiveTab] = useState('dashboard')

  return (
    <div className="app">
      <header className="header">
        <h1>🔥 AgentBurn Solana Trader</h1>
        <p className="subtitle">AI-Powered Trading System</p>
      </header>

      <nav className="nav-tabs">
        <button
          className={activeTab === 'dashboard' ? 'tab active' : 'tab'}
          onClick={() => setActiveTab('dashboard')}
        >
          Dashboard
        </button>
        <button
          className={activeTab === 'trading' ? 'tab active' : 'tab'}
          onClick={() => setActiveTab('trading')}
        >
          Trading Signals
        </button>
        <button
          className={activeTab === 'portfolio' ? 'tab active' : 'tab'}
          onClick={() => setActiveTab('portfolio')}
        >
          Portfolio
        </button>
        <button
          className={activeTab === 'performance' ? 'tab active' : 'tab'}
          onClick={() => setActiveTab('performance')}
        >
          Performance
        </button>
      </nav>

      <main className="main-content">
        {activeTab === 'dashboard' && <Dashboard />}
        {activeTab === 'trading' && <TradingView />}
        {activeTab === 'portfolio' && <Portfolio />}
        {activeTab === 'performance' && <Performance />}
      </main>

      <footer className="footer">
        <p>⚠️ Simulated trading environment - No real funds at risk</p>
      </footer>
    </div>
  )
}

export default App
