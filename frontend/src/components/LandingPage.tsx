interface LandingPageProps {
  onEnter: () => void
}

interface DocLink {
  title: string
  file: string
  summary: string
}

const docLinks: DocLink[] = [
  { title: 'AI Learning Guide', file: 'AI_LEARNING_GUIDE.md', summary: 'Curriculum and heuristics for the DeepSeek + RL stack.' },
  { title: 'Algorithm Improvements', file: 'ALGORITHM_IMPROVEMENTS.md', summary: 'Roadmap of alpha enhancements and latency reducers.' },
  { title: 'Branch Merge Guide', file: 'BRANCH_MERGE_GUIDE.md', summary: 'Safe-delivery workflow for shipping releases.' },
  { title: 'Budget & Quant Features', file: 'BUDGET_AND_QUANT_FEATURES.md', summary: 'Budget guardrails and quant feature toggles.' },
  { title: 'Complete Implementation', file: 'COMPLETE_IMPLEMENTATION.md', summary: 'Full-stack overview of pipelines and APIs.' },
  { title: 'DEX Screener API Guide', file: 'DEXSCREENER_API_GUIDE.md', summary: 'Integration notes for real-time DEX Screener feeds.' },
  { title: 'Features', file: 'FEATURES.md', summary: 'Single source of truth for all shipped modules.' },
  { title: 'Futuristic UI Guide', file: 'FUTURISTIC_UI_GUIDE.md', summary: 'Design language and motion guidelines.' },
  { title: 'Historical Data Guide', file: 'HISTORICAL_DATA_GUIDE.md', summary: 'Pipelines for backfills and replay.' },
  { title: 'Implementation Summary', file: 'IMPLEMENTATION_SUMMARY.md', summary: 'Condensed hand-off for stakeholders.' },
  { title: 'Jito BAM Integration', file: 'JITO_BAM_INTEGRATION.md', summary: 'MEV/BAM connectivity and configuration.' },
  { title: 'Logic Verification', file: 'LOGIC_VERIFICATION.md', summary: 'Formal checks and invariants.' },
  { title: 'Merge Summary', file: 'MERGE_SUMMARY.md', summary: 'Latest release highlights and decisions.' },
  { title: 'Merge to Main', file: 'MERGE_TO_MAIN.md', summary: 'Checklist before cutting production builds.' },
  { title: 'Production Readiness', file: 'PRODUCTION_READINESS_REVIEW.md', summary: 'Deployment gate criteria and SLOs.' },
  { title: 'README', file: 'README.md', summary: 'Bootstrapping instructions for the entire stack.' },
  { title: 'Replit Deployment', file: 'replit.md', summary: 'Hosted playground notes and limitations.' },
  { title: 'Risk Integration', file: 'RISK_INTEGRATION.md', summary: 'Guardrails, limits, and protective circuits.' },
  { title: 'Specialized Providers', file: 'SPECIALIZED_PROVIDERS.md', summary: 'Catalog of external intelligence feeds.' },
  { title: 'Switchboard Oracle Guide', file: 'SWITCHBOARD_ORACLE_GUIDE.md', summary: 'Price oracle wiring and fallback logic.' },
  { title: 'Unified System', file: 'UNIFIED_SYSTEM.md', summary: 'Macro architecture for orchestration.' },
  { title: 'Wallet Integration', file: 'WALLET_INTEGRATION.md', summary: 'Secure signing, custody, and key rotation.' },
  { title: 'X402 Protocol', file: 'X402_PROTOCOL.md', summary: 'Signal marketplace specification.' }
]

export default function LandingPage({ onEnter }: LandingPageProps) {
  return (
    <div className="landing-page">
      <div className="landing-hero glass">
        <div>
          <p className="eyebrow">Solana Autonomous Stack</p>
          <h1>Black-Gold Control Center</h1>
          <p className="hero-copy">
            DeepSeek orchestration, RL agents, and X402 signal commerce all flow through this hub.
            Review the system blueprints, then launch the live trading cockpit when you are ready.
          </p>
          <div className="hero-actions">
            <button className="btn primary glow-btn" onClick={onEnter}>
              Enter Live Dashboard
            </button>
            <a className="btn ghost" href="/docs/README.md" target="_blank" rel="noreferrer">
              View Quickstart
            </a>
          </div>
        </div>
        <div className="hero-metrics">
          <div>
            <span className="metric-label">Latency Budget</span>
            <span className="metric-value">120ms</span>
            <span className="metric-footnote">Solana RPC median</span>
          </div>
          <div>
            <span className="metric-label">RL Agents</span>
            <span className="metric-value">6</span>
            <span className="metric-footnote">Self-improving specialists</span>
          </div>
          <div>
            <span className="metric-label">Atomic Ops</span>
            <span className="metric-value">16</span>
            <span className="metric-footnote">Orchestrator functions</span>
          </div>
        </div>
      </div>

      <section className="landing-section glass">
        <div className="section-header">
          <div>
            <p className="eyebrow">Docs Portal</p>
            <h2>Trace Every Subsystem</h2>
          </div>
          <p>All technical dossiers ship with the app bundle inside <code>/docs</code>.</p>
        </div>
        <div className="docs-grid">
          {docLinks.map(doc => (
            <a
              key={doc.file}
              href={`/docs/${doc.file}`}
              className="doc-card"
              target="_blank"
              rel="noreferrer"
            >
              <div className="doc-card__badge">PDF/Markdown</div>
              <h3>{doc.title}</h3>
              <p>{doc.summary}</p>
              <span className="doc-card__cta">Open ↗</span>
            </a>
          ))}
        </div>
      </section>

      <section className="landing-section glass">
        <div className="section-header">
          <div>
            <p className="eyebrow">Launch Checklist</p>
            <h2>What &ldquo;Live Ready&rdquo; Looks Like</h2>
          </div>
        </div>
        <div className="checklist">
          <div>
            <h4>Backend</h4>
            <ul>
              <li>API v1/v2 health endpoints expose readiness states.</li>
              <li>Risk, wallet, and signal services gated behind config toggles.</li>
              <li>Logging routed to structured sinks for post-trade analysis.</li>
            </ul>
          </div>
          <div>
            <h4>AI/RL Fabric</h4>
            <ul>
              <li>DeepSeek orchestrator aware of 16 atomic operations.</li>
              <li>RL agents streaming telemetry via system executor.</li>
              <li>Meme analysis &amp; X402 providers validated before execution.</li>
            </ul>
          </div>
          <div>
            <h4>Frontend</h4>
            <ul>
              <li>Black-gold UI optimized for low-latency monitoring.</li>
              <li>Lazy-loaded views ensure charts load on demand.</li>
              <li>Docs portal packaged for on-site reference.</li>
            </ul>
          </div>
        </div>
      </section>
    </div>
  )
}
