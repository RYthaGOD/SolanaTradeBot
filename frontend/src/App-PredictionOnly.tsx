import { useState, useEffect } from 'react'
import PredictionMarkets from './components/PredictionMarkets'
import axios from 'axios'

function App() {
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
        <div className="header-content">
          <div className="brand">
            <h1 className="glow">🔮 Prediction Markets Trader</h1>
            <p className="subtitle pulse">EV-Based Trading with Kelly Criterion</p>
          </div>
          
          <div className="header-stats">
            <div className="stat-card mini">
              <div className={`status-dot ${isConnected ? 'connected' : 'disconnected'}`}></div>
              <div className="stat-info">
                <div className="stat-label">Server Status</div>
                <div className="stat-value">{isConnected ? 'Connected' : 'Disconnected'}</div>
              </div>
            </div>
          </div>
        </div>
      </header>

      <main className="main-content modern">
        <div className="content-wrapper fade-in">
          <PredictionMarkets />
        </div>
      </main>

      <footer className="footer modern">
        <div className="footer-content">
          <div className="footer-section">
            <span className="footer-label">Strategy:</span>
            <span className="tech-badge glow">Expected Value Analysis</span>
            <span className="tech-badge glow">Kelly Criterion</span>
          </div>
          <div className="footer-section">
            <span className="footer-label">Markets:</span>
            <span className="tech-badge pulse">Crypto Predictions</span>
            <span className="tech-badge pulse">Binary Outcomes</span>
          </div>
          <div className="footer-section">
            <span className="footer-label">Platform:</span>
            <span className="tech-badge">Solana On-Chain</span>
            <span className="tech-badge">Polymarket-Style</span>
          </div>
        </div>
      </footer>
    </div>
  )
}

export default App
