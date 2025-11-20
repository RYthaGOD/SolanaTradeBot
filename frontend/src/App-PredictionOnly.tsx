import { useState, useEffect } from 'react'
import { useWallet, useConnection } from '@solana/wallet-adapter-react'
import { LAMPORTS_PER_SOL } from '@solana/web3.js'
import PredictionMarkets from './components/PredictionMarkets'
import { WalletButton } from './components/WalletButton'
import axios from 'axios'

function App() {
  const [isConnected, setIsConnected] = useState(false)
  const [balance, setBalance] = useState<number | null>(null)
  
  // Solana wallet hooks
  const { publicKey, connected } = useWallet()
  const { connection } = useConnection()

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

  // Fetch wallet balance when connected
  useEffect(() => {
    if (!publicKey) {
      setBalance(null)
      return
    }

    const getBalance = async () => {
      try {
        const bal = await connection.getBalance(publicKey)
        setBalance(bal / LAMPORTS_PER_SOL)
      } catch (error) {
        console.error('Error fetching balance:', error)
      }
    }

    getBalance()
    const interval = setInterval(getBalance, 10000)
    return () => clearInterval(interval)
  }, [publicKey, connection])

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

            {connected && publicKey && (
              <div className="stat-card mini">
                <div className="stat-info">
                  <div className="stat-label">Wallet Balance</div>
                  <div className="stat-value">
                    {balance !== null ? `${balance.toFixed(4)} SOL` : 'Loading...'}
                  </div>
                </div>
              </div>
            )}

            <WalletButton />
          </div>
        </div>
      </header>

      <main className="main-content modern">
        <div className="content-wrapper fade-in">
          {connected ? (
            <PredictionMarkets />
          ) : (
            <div className="connect-prompt">
              <div className="prompt-icon">🔐</div>
              <h2>Connect Your Solana Wallet</h2>
              <p>Please connect a Solana wallet to start trading prediction markets</p>
              <div className="supported-wallets">
                <span className="wallet-badge">Phantom</span>
                <span className="wallet-badge">Solflare</span>
                <span className="wallet-badge">Backpack</span>
                <span className="wallet-badge">Trust Wallet</span>
                <span className="wallet-badge">Coinbase Wallet</span>
              </div>
              <div className="connect-button-wrapper">
                <WalletButton />
              </div>
            </div>
          )}
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
            <span className="tech-badge">Monaco Protocol</span>
          </div>
          {connected && publicKey && (
            <div className="footer-section">
              <span className="footer-label">Wallet:</span>
              <span className="tech-badge address">
                {publicKey.toBase58().slice(0, 4)}...{publicKey.toBase58().slice(-4)}
              </span>
            </div>
          )}
        </div>
      </footer>
    </div>
  )
}

export default App
