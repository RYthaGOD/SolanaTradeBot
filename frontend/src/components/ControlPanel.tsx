import React, { useState } from 'react';
import axios from 'axios';

const API_V2_BASE = 'http://localhost:8081';

interface Operation {
  name: string;
  endpoint: string;
  description: string;
  category: string;
  params?: { [key: string]: string };
}

const OPERATIONS: Operation[] = [
  // Core Operations
  { name: 'Execute Trade', endpoint: 'trade', description: 'Execute trade + DB + Risk + X402 + RL', category: 'Core', params: { symbol: 'SOL', action: 'buy', size: '1', price: '100' } },
  { name: 'Portfolio', endpoint: 'portfolio', description: 'Get holdings + Calculate value + ROI', category: 'Core' },
  { name: 'Risk Analysis', endpoint: 'risk', description: 'Metrics + Drawdown + Record trades', category: 'Core' },
  { name: 'Database', endpoint: 'database', description: 'Get trades + Stats + Recent', category: 'Core', params: { action: 'stats' } },
  { name: 'Wallet', endpoint: 'wallet', description: 'Balance + Address + Management + PDA', category: 'Core', params: { action: 'info' } },
  { name: 'Fees', endpoint: 'fees', description: 'Estimate + Stats + Network', category: 'Core', params: { priority: 'normal' } },
  { name: 'Predict', endpoint: 'predict', description: 'ML + RL + Meme Analysis', category: 'Core', params: { symbol: 'SOL' } },
  { name: 'Validate', endpoint: 'validate', description: 'Wallet + Amount + Symbol', category: 'Core', params: { type: 'wallet', value: 'test' } },
  { name: 'System Status', endpoint: 'system', description: 'Health + RL + X402', category: 'Core' },
  
  // Advanced Infrastructure
  { name: 'AI Analysis', endpoint: 'ai', description: 'DeepSeek AI analysis + risk', category: 'Infrastructure', params: { action: 'analyze', symbol: 'SOL' } },
  { name: 'Circuit Breaker', endpoint: 'circuit', description: 'Circuit breaker state', category: 'Infrastructure', params: { action: 'status' } },
  { name: 'Oracle', endpoint: 'oracle', description: 'Switchboard aggregation', category: 'Infrastructure', params: { symbol: 'SOL' } },
  { name: 'DEX Screener', endpoint: 'dex', description: 'Token pairs + details', category: 'Infrastructure', params: { chain: 'solana', query: 'SOL' } },
  { name: 'Jupiter Router', endpoint: 'router', description: 'Routing + pair support', category: 'Infrastructure', params: { from: 'SOL', to: 'USDC' } },
  { name: 'Retry', endpoint: 'retry', description: 'Exponential backoff', category: 'Infrastructure', params: { operation: 'test' } },
  { name: 'Stream', endpoint: 'stream', description: 'WebSocket broadcast', category: 'Infrastructure', params: { type: 'market', symbol: 'SOL' } },
  
  // Specialized Features
  { name: 'RL Coordinator', endpoint: 'rl', description: 'RL agent performance', category: 'Specialized', params: { action: 'status' } },
  { name: 'Meme Analyzer', endpoint: 'meme', description: 'Meme safety checks', category: 'Specialized', params: { symbol: 'BONK' } },
  { name: 'X402 Signals', endpoint: 'signals', description: 'Signal marketplace', category: 'Specialized', params: { action: 'list' } },
];

const ControlPanel: React.FC = () => {
  const [activeOp, setActiveOp] = useState<string | null>(null);
  const [results, setResults] = useState<{ [key: string]: any }>({});
  const [loading, setLoading] = useState<{ [key: string]: boolean }>({});
  const [errors, setErrors] = useState<{ [key: string]: string }>({});
  const [category, setCategory] = useState<string>('All');

  const executeOperation = async (op: Operation) => {
    setActiveOp(op.endpoint);
    setLoading({ ...loading, [op.endpoint]: true });
    setErrors({ ...errors, [op.endpoint]: '' });

    try {
      const response = await axios.post(`${API_V2_BASE}/execute/${op.endpoint}`, op.params || {}, {
        timeout: 10000,
      });
      setResults({ ...results, [op.endpoint]: response.data });
    } catch (error: any) {
      const errorMsg = error.response?.data?.error || error.message || 'Unknown error';
      setErrors({ ...errors, [op.endpoint]: errorMsg });
    } finally {
      setLoading({ ...loading, [op.endpoint]: false });
    }
  };

  const categories = ['All', 'Core', 'Infrastructure', 'Specialized'];
  const filteredOps = category === 'All' 
    ? OPERATIONS 
    : OPERATIONS.filter(op => op.category === category);

  return (
    <div className="control-panel">
      <div className="control-header">
        <h2>Manual Control Panel</h2>
        <p>Direct access to all 19 atomic operations</p>
      </div>

      <div className="category-filter">
        {categories.map(cat => (
          <button
            key={cat}
            className={`filter-btn ${category === cat ? 'active' : ''}`}
            onClick={() => setCategory(cat)}
          >
            {cat}
            {cat === 'All' && <span className="badge">19</span>}
            {cat === 'Core' && <span className="badge">9</span>}
            {cat === 'Infrastructure' && <span className="badge">7</span>}
            {cat === 'Specialized' && <span className="badge">3</span>}
          </button>
        ))}
      </div>

      <div className="operations-grid">
        {filteredOps.map(op => (
          <div key={op.endpoint} className={`operation-card ${activeOp === op.endpoint ? 'active' : ''}`}>
            <div className="op-header">
              <h3>{op.name}</h3>
              <span className={`category-badge ${op.category.toLowerCase()}`}>
                {op.category}
              </span>
            </div>
            <p className="op-description">{op.description}</p>
            
            <button
              className="execute-btn"
              onClick={() => executeOperation(op)}
              disabled={loading[op.endpoint]}
            >
              {loading[op.endpoint] ? (
                <><span className="spinner"></span> Executing...</>
              ) : (
                <>⚡ Execute</>
              )}
            </button>

            {results[op.endpoint] && (
              <div className="result success">
                <strong>✓ Success:</strong>
                <pre>{JSON.stringify(results[op.endpoint], null, 2)}</pre>
              </div>
            )}

            {errors[op.endpoint] && (
              <div className="result error">
                <strong>✗ Error:</strong>
                <p>{errors[op.endpoint]}</p>
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
};

export default ControlPanel;
