import { useState, FormEvent } from 'react'
import { httpJson } from '../utils/http'

interface ApiResponse<T> {
  success: boolean
  data: T
  message: string
}

interface ChainBreakdown {
  chain: string
  balance_usd: number
  percentage: number
  token_count: number
}

interface ActivityEntry {
  timestamp: number
  action: string
  symbol: string
  amount: number
  usd_value: number
}

interface WalletOverview {
  address: string
  total_value_usd: number
  native_balance_usd: number
  token_count: number
  nft_count: number
  chains: ChainBreakdown[]
  recent_activity: ActivityEntry[]
}

const formatUsd = (value?: number) => {
  if (value === undefined || Number.isNaN(value)) return '$0.00'
  if (value >= 1_000_000) return `$${(value / 1_000_000).toFixed(2)}M`
  if (value >= 1_000) return `$${(value / 1_000).toFixed(2)}K`
  return `$${value.toFixed(2)}`
}

const formatDate = (timestamp: number) => {
  return new Date(timestamp * 1000).toLocaleString()
}

export default function MoralisWallet() {
  const [address, setAddress] = useState('FaucetAddressDemo1111111111111111111111111111111111')
  const [overview, setOverview] = useState<WalletOverview | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  const fetchInsights = async (event?: FormEvent) => {
    event?.preventDefault()
    if (!address.trim()) {
      setError('Please enter a wallet address')
      return
    }
    setLoading(true)
    setError('')
    try {
      const response = await httpJson<ApiResponse<WalletOverview>>(
        `http://localhost:8080/wallet/insights/${address.trim()}`
      )
      setOverview(response.data)
    } catch (err) {
      console.error('Failed to load wallet insights', err)
      setError('Unable to fetch wallet data. Confirm the backend is running and MORALIS_API_KEY is configured.')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="card">
      <div className="card-header" style={{ marginBottom: '16px' }}>
        <div>
          <h3>Moralis Wallet Insights</h3>
          <p style={{ color: 'var(--muted)' }}>Cross-chain view of holdings, allocation, and recent actions</p>
        </div>
        <form onSubmit={fetchInsights} style={{ display: 'flex', gap: '10px', flexWrap: 'wrap' }}>
          <input
            type="text"
            value={address}
            onChange={(e) => setAddress(e.target.value)}
            className="input-field"
            placeholder="Enter Solana wallet address"
            style={{ minWidth: '260px' }}
          />
          <button className="btn primary" type="submit" disabled={loading}>
            {loading ? 'Loading…' : 'Load Wallet'}
          </button>
        </form>
      </div>

      {error && <div className="error" style={{ marginBottom: '16px' }}>{error}</div>}

      {overview && (
        <div className="wallet-grid">
          <div className="wallet-card">
            <span>Total Net Worth</span>
            <strong>{formatUsd(overview.total_value_usd)}</strong>
          </div>
          <div className="wallet-card">
            <span>Native Balance</span>
            <strong>{formatUsd(overview.native_balance_usd)}</strong>
          </div>
          <div className="wallet-card">
            <span>Tokens</span>
            <strong>{overview.token_count}</strong>
          </div>
          <div className="wallet-card">
            <span>NFTs</span>
            <strong>{overview.nft_count}</strong>
          </div>
        </div>
      )}

      {overview && (
        <div className="wallet-layout">
          <div className="wallet-panel">
            <h4>Chain Allocation</h4>
            <p className="section-subtitle">Distribution of USD value across supported chains</p>
            <div className="allocation-list">
              {overview.chains.map((chain) => (
                <div key={chain.chain} className="allocation-row">
                  <div>
                    <strong>{chain.chain}</strong>
                    <p>{chain.token_count} tokens</p>
                  </div>
                  <div style={{ textAlign: 'right' }}>
                    <strong>{formatUsd(chain.balance_usd)}</strong>
                    <p>{chain.percentage.toFixed(1)}%</p>
                  </div>
                </div>
              ))}
            </div>
          </div>

          <div className="wallet-panel">
            <h4>Recent Activity</h4>
            <p className="section-subtitle">Most recent swaps, stakes, and NFT purchases</p>
            <ul className="activity-list">
              {overview.recent_activity.map((activity, idx) => (
                <li key={`${activity.symbol}-${idx}`}>
                  <div>
                    <strong>{activity.action}</strong>
                    <p>{activity.symbol} • {formatDate(activity.timestamp)}</p>
                  </div>
                  <div style={{ textAlign: 'right' }}>
                    <strong>{activity.amount.toFixed(2)}</strong>
                    <p>{formatUsd(activity.usd_value)}</p>
                  </div>
                </li>
              ))}
            </ul>
          </div>
        </div>
      )}
    </div>
  )
}
