import { useState, useEffect } from 'react'
import axios from 'axios'

interface AtomicFunction {
  name: string
  description: string
  icon: string
  color: string
}

export default function AIOrchestrator() {
  const [context, setContext] = useState('')
  const [result, setResult] = useState<any>(null)
  const [loading, setLoading] = useState(false)
  const [functions, setFunctions] = useState<string[]>([])
  const [selectedFunction, setSelectedFunction] = useState('')
  const [params, setParams] = useState<Record<string, string>>({})

  const atomicFunctions: AtomicFunction[] = [
    { name: 'trade', description: 'Execute trade + DB + Risk + X402 Signal', icon: '💰', color: '#00ff88' },
    { name: 'portfolio', description: 'Holdings + Value + ROI calculation', icon: '💼', color: '#00d4ff' },
    { name: 'risk', description: 'Metrics + Drawdown + Trade records', icon: '🛡️', color: '#ff5500' },
    { name: 'database', description: 'Trades + Stats + Recent history', icon: '💾', color: '#aa00ff' },
    { name: 'wallet', description: 'Balance + Address + Management', icon: '👛', color: '#ffaa00' },
    { name: 'fees', description: 'Estimate + Stats + Network status', icon: '💸', color: '#00aaff' },
    { name: 'predict', description: 'ML + RL + Meme analysis', icon: '🔮', color: '#ff00ff' },
    { name: 'validate', description: 'Wallet + Amount + Symbol checks', icon: '✅', color: '#00ff00' },
    { name: 'system', description: 'Complete health + RL + X402 status', icon: '⚙️', color: '#888888' },
    { name: 'ai', description: 'DeepSeek AI analysis + risk assessment', icon: '🧠', color: '#ff00aa' },
    { name: 'circuit', description: 'Circuit breaker state management', icon: '⚡', color: '#ffff00' },
    { name: 'oracle', description: 'Switchboard price aggregation', icon: '🔮', color: '#00ffff' },
    { name: 'dex', description: 'DEX Screener token pairs', icon: '📊', color: '#ff8800' },
    { name: 'router', description: 'Jupiter routing + pair support', icon: '🔄', color: '#8800ff' },
    { name: 'retry', description: 'Retry with exponential backoff', icon: '🔁', color: '#ff0088' },
    { name: 'stream', description: 'WebSocket market updates', icon: '📡', color: '#00ff88' },
  ]

  useEffect(() => {
    fetchFunctions()
  }, [])

  const fetchFunctions = async () => {
    try {
      const response = await axios.get('http://localhost:8081/functions')
      setFunctions(response.data.functions || [])
    } catch (error) {
      console.error('Failed to fetch functions:', error)
    }
  }

  const handleOrchestrate = async () => {
    if (!context.trim()) return
    
    setLoading(true)
    setResult(null)
    
    try {
      const response = await axios.post('http://localhost:8081/orchestrate', {
        context: context,
        parameters: {}
      })
      setResult(response.data)
    } catch (error: any) {
      setResult({ error: error.message })
    } finally {
      setLoading(false)
    }
  }

  const handleExecute = async () => {
    if (!selectedFunction) return
    
    setLoading(true)
    setResult(null)
    
    try {
      const response = await axios.post(`http://localhost:8081/execute/${selectedFunction}`, params)
      setResult(response.data)
    } catch (error: any) {
      setResult({ error: error.message })
    } finally {
      setLoading(false)
    }
  }

  const addParam = (key: string, value: string) => {
    setParams(prev => ({ ...prev, [key]: value }))
  }

  const removeParam = (key: string) => {
    const newParams = { ...params }
    delete newParams[key]
    setParams(newParams)
  }

  return (
    <div className="orchestrator-container">
      <div className="section-header">
        <h2 className="glow">🤖 AI Orchestrator</h2>
        <p>DeepSeek-powered intelligent routing across 16 atomic operations</p>
      </div>

      <div className="orchestrator-grid">
        {/* AI Context Mode */}
        <div className="card large glow-border">
          <div className="card-header">
            <h3>🧠 AI Context Mode</h3>
            <span className="badge pulse">DeepSeek Intelligence</span>
          </div>
          
          <div className="input-group">
            <label>Describe what you want to do:</label>
            <textarea
              value={context}
              onChange={(e) => setContext(e.target.value)}
              placeholder="e.g., 'Execute a trade for SOL with 10 units at $100' or 'Check my portfolio performance' or 'Analyze BONK memecoin'"
              rows={4}
              className="input-field glow-input"
            />
          </div>

          <button 
            onClick={handleOrchestrate} 
            disabled={loading || !context.trim()}
            className="btn primary large glow-btn"
          >
            {loading ? '⏳ Processing...' : '🚀 Let AI Decide & Execute'}
          </button>
        </div>

        {/* Direct Execution Mode */}
        <div className="card large">
          <div className="card-header">
            <h3>⚡ Direct Execution Mode</h3>
            <span className="badge">Bypass AI - Direct Control</span>
          </div>
          
          <div className="input-group">
            <label>Select Atomic Function:</label>
            <select 
              value={selectedFunction} 
              onChange={(e) => setSelectedFunction(e.target.value)}
              className="input-field"
            >
              <option value="">-- Choose Function --</option>
              {functions.map(func => (
                <option key={func} value={func}>{func}</option>
              ))}
            </select>
          </div>

          {selectedFunction && (
            <div className="params-section">
              <h4>Parameters</h4>
              {Object.keys(params).map(key => (
                <div key={key} className="param-row">
                  <input type="text" value={key} disabled className="param-key" />
                  <input 
                    type="text" 
                    value={params[key]} 
                    onChange={(e) => addParam(key, e.target.value)}
                    className="param-value"
                  />
                  <button onClick={() => removeParam(key)} className="btn-remove">×</button>
                </div>
              ))}
              
              <button 
                onClick={() => addParam(`param${Object.keys(params).length + 1}`, '')}
                className="btn secondary small"
              >
                + Add Parameter
              </button>
            </div>
          )}

          <button 
            onClick={handleExecute} 
            disabled={loading || !selectedFunction}
            className="btn primary large"
          >
            {loading ? '⏳ Executing...' : '▶️ Execute Function'}
          </button>
        </div>
      </div>

      {/* Available Atomic Functions Grid */}
      <div className="functions-grid">
        <h3 className="section-title">📦 16 Atomic Operations</h3>
        <div className="function-cards">
          {atomicFunctions.map(func => (
            <div 
              key={func.name} 
              className="function-card hover-lift"
              style={{ '--func-color': func.color } as any}
              onClick={() => setSelectedFunction(func.name)}
            >
              <div className="func-icon">{func.icon}</div>
              <div className="func-name">{func.name}</div>
              <div className="func-desc">{func.description}</div>
            </div>
          ))}
        </div>
      </div>

      {/* Result Display */}
      {result && (
        <div className="card result-card fade-in">
          <div className="card-header">
            <h3>📊 Execution Result</h3>
          </div>
          <pre className="result-output">
            {JSON.stringify(result, null, 2)}
          </pre>
        </div>
      )}
    </div>
  )
}
