import { useEffect, useRef, useCallback, useState } from 'react';
import { useNavigate } from 'react-router-dom';

const ONBOARDING_KEY = 'db-sim-onboarding-complete';

function scrollTo(id: string) {
  document.getElementById(id)?.scrollIntoView({ behavior: 'smooth' });
}

function useLaunchSimulator() {
  const navigate = useNavigate();
  return useCallback(() => {
    try { localStorage.removeItem(ONBOARDING_KEY); } catch { /* */ }
    navigate('/app');
  }, [navigate]);
}

// ── Reveal on scroll ───────────────────────────────────────────────────────
function Reveal({ children, className = '', delay = 0 }: { children: React.ReactNode; className?: string; delay?: number }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const obs = new IntersectionObserver(
      ([e]) => { if (e.isIntersecting) { el.classList.add('revealed'); obs.unobserve(el); } },
      { threshold: 0.12 },
    );
    obs.observe(el);
    return () => obs.disconnect();
  }, []);
  return (
    <div ref={ref} className={`reveal-target ${className}`} style={{ transitionDelay: `${delay}ms` }}>
      {children}
    </div>
  );
}

// ── Animated counter ───────────────────────────────────────────────────────
function Counter({ end, suffix = '' }: { end: number; suffix?: string }) {
  const [val, setVal] = useState(0);
  const ref = useRef<HTMLSpanElement>(null);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const obs = new IntersectionObserver(([e]) => {
      if (e.isIntersecting) {
        let start = 0;
        const step = Math.max(1, Math.floor(end / 30));
        const iv = setInterval(() => {
          start += step;
          if (start >= end) { start = end; clearInterval(iv); }
          setVal(start);
        }, 30);
        obs.unobserve(el);
      }
    }, { threshold: 0.5 });
    obs.observe(el);
    return () => obs.disconnect();
  }, [end]);
  return <span ref={ref}>{val}{suffix}</span>;
}

// ── Hero Canvas — animated product preview ─────────────────────────────────
function HeroCanvas() {
  return (
    <div className="hero-canvas-wrap">
      <svg viewBox="0 0 720 340" fill="none" className="hero-canvas">
        {/* Dot grid */}
        <defs>
          <pattern id="dotgrid" x="0" y="0" width="20" height="20" patternUnits="userSpaceOnUse">
            <circle cx="1" cy="1" r="0.8" fill="rgba(255,255,255,0.06)" />
          </pattern>
          <filter id="glow">
            <feGaussianBlur stdDeviation="3" result="blur" />
            <feMerge><feMergeNode in="blur" /><feMergeNode in="SourceGraphic" /></feMerge>
          </filter>
        </defs>
        <rect width="720" height="340" fill="url(#dotgrid)" />

        {/* ── Connection paths (draw in) ── */}
        <path d="M168 75 L260 75" className="conn-path" style={{ animationDelay: '0.8s' }} stroke="#3B82F6" />
        <path d="M168 75 L220 75 Q240 75 240 95 L240 185 Q240 205 260 205" className="conn-path" style={{ animationDelay: '1s' }} stroke="#14B8A6" />
        <path d="M430 75 L520 75" className="conn-path" style={{ animationDelay: '1.4s' }} stroke="#8B5CF6" />
        <path d="M430 205 L480 205 Q500 205 500 185 L500 95 Q500 75 520 75" className="conn-path" style={{ animationDelay: '1.6s' }} stroke="#8B5CF6" />
        <path d="M430 205 L520 245" className="conn-path" style={{ animationDelay: '1.8s' }} stroke="#F59E0B" />

        {/* ── Data particles ── */}
        <circle r="3" fill="#3B82F6" filter="url(#glow)" className="particle">
          <animateMotion dur="2.5s" repeatCount="indefinite" begin="1.5s" path="M168 75 L260 75" />
        </circle>
        <circle r="3" fill="#14B8A6" filter="url(#glow)" className="particle">
          <animateMotion dur="3s" repeatCount="indefinite" begin="2s" path="M168 75 L220 75 Q240 75 240 95 L240 185 Q240 205 260 205" />
        </circle>
        <circle r="3" fill="#8B5CF6" filter="url(#glow)" className="particle">
          <animateMotion dur="2.5s" repeatCount="indefinite" begin="2.5s" path="M430 75 L520 75" />
        </circle>
        <circle r="3" fill="#F59E0B" filter="url(#glow)" className="particle">
          <animateMotion dur="2s" repeatCount="indefinite" begin="3s" path="M430 205 L520 245" />
        </circle>

        {/* ── Block: Schema ── */}
        <g className="block-enter" style={{ animationDelay: '0.1s' }}>
          <rect x="28" y="48" width="140" height="54" rx="8" fill="#0D1320" stroke="#1E293B" strokeWidth="1.5" />
          <rect x="28" y="48" width="140" height="4" rx="2" fill="#6366F1" />
          <text x="98" y="72" textAnchor="middle" className="block-label">Schema</text>
          <text x="98" y="88" textAnchor="middle" className="block-sublabel">users — 4 cols</text>
        </g>

        {/* ── Block: Heap Storage ── */}
        <g className="block-enter" style={{ animationDelay: '0.25s' }}>
          <rect x="260" y="48" width="170" height="54" rx="8" fill="#0D1320" stroke="#1E293B" strokeWidth="1.5" />
          <rect x="260" y="48" width="170" height="4" rx="2" fill="#8B5CF6" />
          <text x="345" y="72" textAnchor="middle" className="block-label">Heap Storage</text>
          <text x="345" y="88" textAnchor="middle" className="block-sublabel">8 KB pages, 90% fill</text>
        </g>

        {/* ── Block: LRU Buffer ── */}
        <g className="block-enter" style={{ animationDelay: '0.4s' }}>
          <rect x="260" y="178" width="170" height="54" rx="8" fill="#0D1320" stroke="#1E293B" strokeWidth="1.5" />
          <rect x="260" y="178" width="170" height="4" rx="2" fill="#14B8A6" />
          <text x="345" y="202" textAnchor="middle" className="block-label">LRU Buffer</text>
          <text x="345" y="218" textAnchor="middle" className="block-sublabel">128 MB — 94% hit rate</text>
          {/* Animated fill bar */}
          <rect x="278" y="225" width="134" height="3" rx="1.5" fill="#1E293B" />
          <rect x="278" y="225" width="0" height="3" rx="1.5" fill="#14B8A6" className="fill-bar" />
        </g>

        {/* ── Block: B-Tree Index ── */}
        <g className="block-enter" style={{ animationDelay: '0.55s' }}>
          <rect x="520" y="48" width="170" height="54" rx="8" fill="#0D1320" stroke="#1E293B" strokeWidth="1.5" />
          <rect x="520" y="48" width="170" height="4" rx="2" fill="#3B82F6" />
          <text x="605" y="72" textAnchor="middle" className="block-label">B-Tree Index</text>
          <text x="605" y="88" textAnchor="middle" className="block-sublabel">id — fanout 128</text>
        </g>

        {/* ── Block: Seq Scan ── */}
        <g className="block-enter" style={{ animationDelay: '0.7s' }}>
          <rect x="520" y="218" width="170" height="54" rx="8" fill="#0D1320" stroke="#1E293B" strokeWidth="1.5" />
          <rect x="520" y="218" width="170" height="4" rx="2" fill="#F59E0B" />
          <text x="605" y="242" textAnchor="middle" className="block-label">Sequential Scan</text>
          <text x="605" y="258" textAnchor="middle" className="block-sublabel">32 pages prefetch</text>
        </g>

        {/* ── Live metric overlay ── */}
        <g className="block-enter" style={{ animationDelay: '2.2s' }}>
          <rect x="548" y="134" width="126" height="68" rx="8" fill="#0D1320" stroke="#22D3EE" strokeWidth="1" opacity="0.9" />
          <text x="558" y="152" className="metric-label">throughput</text>
          <text x="558" y="172" className="metric-value">12,430 ops/s</text>
          <text x="558" y="192" className="metric-label-green">avg latency 0.8ms</text>
        </g>
      </svg>
    </div>
  );
}

// ── Typing query animation ─────────────────────────────────────────────────
function TypingQuery() {
  const lines = [
    { prompt: '$', text: 'SELECT * FROM users WHERE id = 42;', color: '#22D3EE' },
    { prompt: ' ', text: '→ B-Tree lookup: 3 page reads', color: '#3B82F6' },
    { prompt: ' ', text: '→ Buffer hit: page found in cache', color: '#14B8A6' },
    { prompt: ' ', text: '→ Result: 1 row in 0.8ms', color: '#10B981' },
  ];
  return (
    <div className="font-mono text-[13px] leading-6 p-5">
      {lines.map((l, i) => (
        <div key={i} className="typing-line" style={{ animationDelay: `${i * 0.8}s` }}>
          <span className="text-gray-600">{l.prompt} </span>
          <span style={{ color: l.color }}>{l.text}</span>
        </div>
      ))}
      <span className="terminal-cursor">_</span>
    </div>
  );
}

// ── Mini bar chart for metrics card ────────────────────────────────────────
function MiniChart() {
  const bars = [35, 58, 42, 72, 65, 88, 54, 90, 68, 80, 95, 70];
  return (
    <div className="flex items-end gap-[3px] h-16 px-5 pb-1">
      {bars.map((h, i) => (
        <div
          key={i}
          className="mini-bar flex-1 rounded-sm"
          style={{
            height: `${h}%`,
            background: h > 80 ? '#10B981' : h > 60 ? '#3B82F6' : '#1E293B',
            animationDelay: `${i * 0.06}s`,
          }}
        />
      ))}
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════
export function LandingPage() {
  const launch = useLaunchSimulator();

  return (
    <div className="landing-root">
      {/* ── Nav ──────────────────────────────────────────────────── */}
      <nav className="landing-nav">
        <div className="nav-logo">
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
            <rect width="24" height="24" rx="6" fill="#3B82F6" />
            <path d="M7 8h10M7 12h7M7 16h10" stroke="#fff" strokeWidth="2" strokeLinecap="round" />
          </svg>
          <span>DB Simulator</span>
        </div>
        <div className="nav-links">
          <button onClick={() => scrollTo('features')}>Features</button>
          <button onClick={() => scrollTo('how-it-works')}>How It Works</button>
          <button onClick={launch} className="nav-cta">Open App</button>
        </div>
      </nav>

      {/* ── Hero ─────────────────────────────────────────────────── */}
      <section className="hero">
        <div className="hero-content">
          <p className="hero-tag">interactive database playground</p>
          <h1>
            Build databases<br />
            from scratch.<br />
            <span className="hero-accent">Break them on purpose.</span>
          </h1>
          <p className="hero-sub">
            Wire together B-trees, buffer pools, WAL, and LSM trees on a visual canvas.
            Run queries. Watch data flow. Understand what's actually happening under the hood.
          </p>
          <div className="hero-actions">
            <button onClick={launch} className="cta-primary">
              Start building
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M3 8h10M9 4l4 4-4 4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"/></svg>
            </button>
            <button onClick={() => scrollTo('demo')} className="cta-ghost">See how it works</button>
          </div>
          <div className="hero-stats">
            <div><strong>18</strong> block types</div>
            <div className="stat-dot" />
            <div><strong>6</strong> categories</div>
            <div className="stat-dot" />
            <div>Rust + WASM engine</div>
            <div className="stat-dot" />
            <div>Zero install</div>
          </div>
        </div>
        <HeroCanvas />
      </section>

      {/* ── Demo ─────────────────────────────────────────────────── */}
      <section id="demo" className="demo-section">
        <Reveal>
          <div className="demo-window">
            <div className="window-chrome">
              <div className="traffic-lights">
                <span /><span /><span />
              </div>
              <div className="window-url">db-simulator.app</div>
              <div />
            </div>
            <div className="demo-placeholder">
              <div className="demo-play-wrap">
                <button className="demo-play">
                  <svg width="28" height="28" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z" /></svg>
                </button>
              </div>
              <p>Demo video coming soon</p>
            </div>
          </div>
        </Reveal>
      </section>

      {/* ── Bento Features ───────────────────────────────────────── */}
      <section id="features" className="features-section">
        <Reveal>
          <div className="section-header">
            <p className="section-tag">capabilities</p>
            <h2>Not just diagrams.<br />A real simulation engine.</h2>
          </div>
        </Reveal>

        <div className="bento">
          {/* Big card — visual design with mini canvas */}
          <Reveal className="bento-card bento-wide">
            <div className="bento-card-inner">
              <div className="bento-text">
                <h3>Visual block design</h3>
                <p>Drag storage engines, indexes, buffers, and scan operators onto an infinite canvas.
                   Connect outputs to inputs. Each block is a real database component with configurable parameters.</p>
              </div>
              <div className="bento-viz bento-mini-canvas">
                <svg viewBox="0 0 280 120" fill="none">
                  <rect x="10" y="20" width="76" height="36" rx="6" fill="#1A1F35" stroke="#3B82F6" strokeWidth="1" />
                  <text x="48" y="42" textAnchor="middle" fill="#3B82F6" fontSize="10" fontFamily="monospace">Heap</text>
                  <rect x="110" y="8" width="76" height="36" rx="6" fill="#1A1F35" stroke="#8B5CF6" strokeWidth="1" />
                  <text x="148" y="30" textAnchor="middle" fill="#8B5CF6" fontSize="10" fontFamily="monospace">B-Tree</text>
                  <rect x="110" y="64" width="76" height="36" rx="6" fill="#1A1F35" stroke="#14B8A6" strokeWidth="1" />
                  <text x="148" y="86" textAnchor="middle" fill="#14B8A6" fontSize="10" fontFamily="monospace">Buffer</text>
                  <path d="M86 32 L110 26" stroke="#3B82F6" strokeWidth="1" strokeDasharray="3 2" className="animated-dash" />
                  <path d="M86 44 L110 76" stroke="#14B8A6" strokeWidth="1" strokeDasharray="3 2" className="animated-dash" />
                  <rect x="210" y="36" width="60" height="36" rx="6" fill="#1A1F35" stroke="#F59E0B" strokeWidth="1" />
                  <text x="240" y="58" textAnchor="middle" fill="#F59E0B" fontSize="10" fontFamily="monospace">Scan</text>
                  <path d="M186 26 L210 48" stroke="#8B5CF6" strokeWidth="1" strokeDasharray="3 2" className="animated-dash" />
                  <path d="M186 82 L210 60" stroke="#14B8A6" strokeWidth="1" strokeDasharray="3 2" className="animated-dash" />
                </svg>
              </div>
            </div>
          </Reveal>

          {/* Query execution card with terminal */}
          <Reveal className="bento-card bento-tall" delay={100}>
            <div className="bento-card-inner">
              <div className="bento-text">
                <h3>Watch queries execute</h3>
                <p>Trace every page read, index lookup, and cache hit as queries flow through your design.</p>
              </div>
              <div className="bento-viz bento-terminal">
                <TypingQuery />
              </div>
            </div>
          </Reveal>

          {/* Live metrics card */}
          <Reveal className="bento-card" delay={200}>
            <div className="bento-card-inner">
              <div className="bento-text">
                <h3>Live metrics</h3>
                <p>Throughput, latency percentiles, cache hit rates, IOPS — all updating in real time.</p>
              </div>
              <div className="bento-viz">
                <MiniChart />
                <div className="metric-row">
                  <span className="metric-pill metric-pill-green"><Counter end={94} suffix="%" /> hit rate</span>
                  <span className="metric-pill metric-pill-blue"><Counter end={12430} /> ops/s</span>
                </div>
              </div>
            </div>
          </Reveal>

          {/* Compare card */}
          <Reveal className="bento-card" delay={300}>
            <div className="bento-card-inner">
              <div className="bento-text">
                <h3>Compare designs</h3>
                <p>Build two architectures. Run the same workload. See which one wins and understand why.</p>
              </div>
              <div className="bento-viz bento-compare">
                <div className="compare-bar">
                  <div className="compare-fill" style={{ width: '78%', background: '#3B82F6' }} />
                  <span>Design A — 78%</span>
                </div>
                <div className="compare-bar">
                  <div className="compare-fill" style={{ width: '94%', background: '#10B981' }} />
                  <span>Design B — 94%</span>
                </div>
              </div>
            </div>
          </Reveal>

          {/* Learning challenges */}
          <Reveal className="bento-card" delay={400}>
            <div className="bento-card-inner">
              <div className="bento-text">
                <h3>Learning challenges</h3>
                <p>Guided experiments that teach indexing, caching, write-ahead logging, and more — by doing.</p>
              </div>
              <div className="bento-viz bento-challenges">
                <div className="challenge-item done"><span className="check">&#10003;</span> Add a B-tree to speed up lookups</div>
                <div className="challenge-item done"><span className="check">&#10003;</span> Size the buffer pool for 90%+ hit rate</div>
                <div className="challenge-item active"><span className="pulse-dot" /> Enable WAL for crash recovery</div>
              </div>
            </div>
          </Reveal>

          {/* Templates */}
          <Reveal className="bento-card" delay={500}>
            <div className="bento-card-inner">
              <div className="bento-text">
                <h3>Start from templates</h3>
                <p>OLTP, write-heavy logging, analytics, concurrent MVCC — pick one and start tweaking.</p>
              </div>
              <div className="bento-viz bento-templates">
                <div className="tpl-chip" style={{ borderColor: '#3B82F6' }}>OLTP Balanced</div>
                <div className="tpl-chip" style={{ borderColor: '#10B981' }}>Write-Heavy</div>
                <div className="tpl-chip" style={{ borderColor: '#8B5CF6' }}>Analytics</div>
                <div className="tpl-chip" style={{ borderColor: '#6366F1' }}>MVCC</div>
              </div>
            </div>
          </Reveal>
        </div>
      </section>

      {/* ── How It Works ─────────────────────────────────────────── */}
      <section id="how-it-works" className="how-section">
        <Reveal>
          <div className="section-header">
            <p className="section-tag">workflow</p>
            <h2>Three steps. Zero boilerplate.</h2>
          </div>
        </Reveal>
        <div className="how-steps">
          <Reveal className="how-step" delay={0}>
            <div className="step-num">01</div>
            <h3>Design</h3>
            <p>Drag blocks from the palette. Connect them. Choose your storage engine, index strategy, and buffer size.</p>
          </Reveal>
          <div className="step-arrow">
            <svg width="40" height="24" viewBox="0 0 40 24" fill="none">
              <path d="M0 12h36M28 4l8 8-8 8" stroke="#1E293B" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          </div>
          <Reveal className="how-step" delay={150}>
            <div className="step-num">02</div>
            <h3>Configure</h3>
            <p>Define a workload — mix of reads, writes, scans. Set concurrency, distribution, total operations.</p>
          </Reveal>
          <div className="step-arrow">
            <svg width="40" height="24" viewBox="0 0 40 24" fill="none">
              <path d="M0 12h36M28 4l8 8-8 8" stroke="#1E293B" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          </div>
          <Reveal className="how-step" delay={300}>
            <div className="step-num">03</div>
            <h3>Simulate</h3>
            <p>Hit play. Watch data flow through every component. Read the metrics. Learn what matters.</p>
          </Reveal>
        </div>
      </section>

      {/* ── Tech strip ───────────────────────────────────────────── */}
      <section className="tech-section">
        <Reveal>
          <div className="tech-inner">
            <p className="tech-label">Powered by</p>
            <div className="tech-pills">
              <span>Rust</span>
              <span className="tech-arrow">&rarr;</span>
              <span>WebAssembly</span>
              <span className="tech-arrow">&rarr;</span>
              <span>Your Browser</span>
            </div>
            <p className="tech-sub">The simulation engine is a Rust crate compiled to WASM. No server. No latency. Everything runs locally at near-native speed.</p>
          </div>
        </Reveal>
      </section>

      {/* ── CTA ──────────────────────────────────────────────────── */}
      <section className="cta-section">
        <Reveal>
          <h2>Your database.<br />Your rules.</h2>
          <p>Open the simulator and start experimenting. No account needed.</p>
          <button onClick={launch} className="cta-primary cta-large">
            Open DB Simulator
            <svg width="18" height="18" viewBox="0 0 16 16" fill="none"><path d="M3 8h10M9 4l4 4-4 4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"/></svg>
          </button>
        </Reveal>
      </section>

      {/* ── Footer ───────────────────────────────────────────────── */}
      <footer className="landing-footer">
        <span>Built with Rust, WASM, React, and a love for databases</span>
      </footer>
    </div>
  );
}
