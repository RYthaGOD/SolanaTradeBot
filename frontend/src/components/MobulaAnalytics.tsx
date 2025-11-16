import { useEffect, useState, FormEvent } from 'react'
import { httpJson } from '../utils/http'

interface ApiResponse<T> {
  success: boolean
  data: T
  message: string
}

interface TokenInfo {
  symbol: string
  name: string
}

interface MobulaPair {
  pair_address: string
  url: string
  base_token: TokenInfo
  price_usd?: string
  volume: {
    h24: number
  }
  liquidity: {
    usd?: number
  }
  price_change: {
    m5: number
    h1: number
    h6: number
    h24: number
  }
}

interface TradingOpportunity {
  pair_address: string
  token_symbol: string
  token_name: string
  price_usd: number
  liquidity_usd: number
  opportunity_score: number
  price_change_5m: number
  price_change_1h: number
  price_change_6h: number
  price_change_24h: number
  signals: string[]
}

const formatUsd = (value?: number | string) => {
  if (!value) return '$0.00'
  const num = typeof value === 'string' ? parseFloat(value) : value
  if (Number.isNaN(num)) return '$0.00'
  if (num >= 1_000_000) return `$${(num / 1_000_000).toFixed(1)}M`
  if (num >= 1_000) return `$${(num / 1_000).toFixed(1)}K`
  return `$${num.toFixed(2)}`
}

const percent = (value?: number) => {
  if (value === undefined || Number.isNaN(value)) return '0.0%'
  const sign = value > 0 ? '+' : ''
  return `${sign}${value.toFixed(1)}%`
}

export default function MobulaAnalytics() {
  const [trending, setTrending] = useState<MobulaPair[]>([])
  const [opportunities, setOpportunities] = useState<TradingOpportunity[]>([])
  const [searchResults, setSearchResults] = useState<MobulaPair[]>([])
  const [searchQuery, setSearchQuery] = useState('')
  const [loadingSearch, setLoadingSearch] = useState(false)
  const [error, setError] = useState('')

  useEffect(() => {
    fetchTrending()
    fetchOpportunities()
  }, [])

  const fetchTrending = async () => {
    try {
      const response = await httpJson<ApiResponse<MobulaPair[]>>('http://localhost:8080/dex/trending')
      setTrending(response.data.slice(0, 6))
    } catch (err) {
      console.error('Failed to load trending tokens', err)
      setError('Unable to load trending tokens')
    }
  }

  const fetchOpportunities = async () => {
    try {
      const response = await httpJson<ApiResponse<TradingOpportunity[]>>('http://localhost:8080/dex/opportunities')
      setOpportunities(response.data.slice(0, 5))
    } catch (err) {
      console.error('Failed to load Mobula opportunities', err)
      setError('Unable to load Mobula opportunities')
    }
  }

  const handleSearch = async (event: FormEvent) => {
    event.preventDefault()
    if (!searchQuery.trim()) return
    setLoadingSearch(true)
    setError('')
    try {
      const response = await httpJson<ApiResponse<MobulaPair[]>>(
        `http://localhost:8080/dex/search/${encodeURIComponent(searchQuery.trim())}`
      )
      setSearchResults(response.data.slice(0, 6))
    } catch (err) {
      console.error('Mobula search failed', err)
      setError('Search failed. Try another token or check API health.')
    } finally {
      setLoadingSearch(false)
    }
  }

  return (
    <div className="card mobula-card">
      <div className="card-header" style={{ marginBottom: '20px' }}>
        <div>
          <h3>Mobula Intelligence</h3>
          <p style={{ color: 'var(--muted)', marginTop: '4px' }}>
            Unified Solana token discovery from the Mobula GMGN-compatible data API
          </p>
        </div>
        <form onSubmit={handleSearch} style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search token symbol or address"
            className="input-field"
            style={{ minWidth: '240px' }}
          />
          <button className="btn primary small" type="submit" disabled={loadingSearch}>
            {loadingSearch ? 'Searching...' : 'Search'}
          </button>
        </form>
      </div>

      {error && <div className="error" style={{ marginBottom: '20px' }}>{error}</div>}

      {searchResults.length > 0 && (
        <div style={{ marginBottom: '24px' }}>
          <h4 style={{ marginBottom: '12px' }}>Search Results</h4>
          <div className="mobula-grid">
            {searchResults.map((pair) => (
              <div key={pair.pair_address} className="mobula-token-card">
                <div className="token-header">
                  <div>
                    <strong>{pair.base_token.symbol}</strong>
                    <p style={{ color: 'var(--muted)', fontSize: '0.85rem' }}>{pair.base_token.name}</p>
                  </div>
                  <span>{formatUsd(pair.price_usd)}</span>
                </div>
                <div className="token-metrics">
                  <div>
                    <span>24h Volume</span>
                    <strong>{formatUsd(pair.volume.h24)}</strong>
                  </div>
                  <div>
                    <span>Liquidity</span>
                    <strong>{formatUsd(pair.liquidity.usd)}</strong>
                  </div>
                  <div>
                    <span>1h Change</span>
                    <strong style={{ color: pair.price_change.h1 >= 0 ? '#48f7c1' : '#ff6b6b' }}>
                      {percent(pair.price_change.h1)}
                    </strong>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      <div className="mobula-sections">
        <div>
          <h4>Trending On-Chain</h4>
          <p className="section-subtitle">Mobula volume leaders filtered by deep liquidity</p>
          <div className="mobula-grid">
            {trending.map((pair) => (
              <div key={pair.pair_address} className="mobula-token-card">
                <div className="token-header">
                  <div>
                    <strong>{pair.base_token.symbol}</strong>
                    <p style={{ color: 'var(--muted)', fontSize: '0.85rem' }}>{pair.base_token.name}</p>
                  </div>
                  <span>{formatUsd(pair.price_usd)}</span>
                </div>
                <div className="token-metrics">
                  <div>
                    <span>24h Volume</span>
                    <strong>{formatUsd(pair.volume.h24)}</strong>
                  </div>
                  <div>
                    <span>Liquidity</span>
                    <strong>{formatUsd(pair.liquidity.usd)}</strong>
                  </div>
                  <div>
                    <span>24h Change</span>
                    <strong style={{ color: pair.price_change.h24 >= 0 ? '#48f7c1' : '#ff6b6b' }}>
                      {percent(pair.price_change.h24)}
                    </strong>
                  </div>
                </div>
                <a
                  href={pair.url}
                  target="_blank"
                  rel="noreferrer"
                  className="doc-card__cta"
                  style={{ fontSize: '0.85rem' }}
                >
                  View Pair ↗
                </a>
              </div>
            ))}
          </div>
        </div>

        <div>
          <h4>High-Conviction Opportunities</h4>
          <p className="section-subtitle">Composite Mobula scores across momentum, volume, and liquidity</p>
          <div className="opportunity-list">
            {opportunities.map((opp) => (
              <div key={opp.pair_address} className="opportunity-card">
                <div className="opportunity-header">
                  <div>
                    <strong>{opp.token_symbol}</strong>
                    <p style={{ color: 'var(--muted)', fontSize: '0.85rem' }}>{opp.token_name}</p>
                  </div>
                  <span className="score-badge">{opp.opportunity_score.toFixed(0)}</span>
                </div>
                <div className="token-metrics" style={{ marginTop: '10px' }}>
                  <div>
                    <span>Price</span>
                    <strong>{formatUsd(opp.price_usd)}</strong>
                  </div>
                  <div>
                    <span>Liquidity</span>
                    <strong>{formatUsd(opp.liquidity_usd)}</strong>
                  </div>
                  <div>
                    <span>Momentum</span>
                    <strong style={{ color: opp.price_change_1h >= 0 ? '#48f7c1' : '#ff6b6b' }}>
                      {percent(opp.price_change_1h)}
                    </strong>
                  </div>
                </div>
                {opp.signals.length > 0 && (
                  <ul className="signal-list">
                    {opp.signals.slice(0, 3).map((signal, index) => (
                      <li key={`${opp.pair_address}-${index}`}>{signal}</li>
                    ))}
                  </ul>
                )}
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  )
}
