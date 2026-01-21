# Modular DB Builder: PRD

**Last Updated**: 2026-01-20
**Status**: Concept → Prototype
**Owner**: [TBD]

---

## TL;DR

We're building a **visual database construction kit** that lets database researchers and engineers design custom database architectures by composing reusable blocks — without writing code.

**The bet**: Visual composition + instant feedback will unlock 10x faster experimentation for database design.

---

## Part 1: Why This, Why Now?

### The Problem (in one sentence)

Designing a new database architecture today requires months of coding before you know if your design choices actually work for your use case.

### The Current State (what sucks today)

If you want to explore database design alternatives, you must:

1. **Read papers** → Guess which approach might work
2. **Write thousands of lines of code** → Implement storage engine, indexes, query planner
3. **Build test harnesses** → Generate workloads, collect metrics
4. **Wait weeks** → Only then see if your design is good or bad
5. **Start over** → If you want to try a different approach

**Result**: People don't experiment. They copy existing designs (PostgreSQL, MySQL) even when those aren't optimal for their use case.

### Why This Matters

Every company with >1M users eventually hits database limits:
- Shopify re-architected for multi-tenancy
- Discord moved from MongoDB to Cassandra for messages
- Uber built their own storage engine (Docstore)
- Cloudflare built their own analytics DB (ClickHouse-inspired)

But they only tried ONE alternative because experimentation is too expensive.

**What if you could try 10 alternatives in a weekend?**

### Why Now?

Three things converged:

1. **Visual programming tools matured** — Unreal Blueprint, Node-RED, Figma plugins prove visual composition works for complex systems
2. **WebAssembly is fast enough** — We can run real simulations in the browser (RocksDB compiled to WASM exists)
3. **Database research is exploding** — New storage engines (Bw-Tree, FASTER), indexes (Learned Indexes), and compaction strategies published yearly, but no one tries them

### The Opportunity

**Who wins**: Teams that can experiment faster

**The insight**: Database design is composition (storage + indexes + concurrency), not monolithic code. Make composition visual and instant.

---

## Part 2: Who Is This For?

### Primary User: The Database Architect

**Profile**:
- Senior engineer or researcher
- Designing a new system or replatforming existing one
- Knows database internals (understands B-trees, MVCC, LSM trees)
- Constrained by time and budget to try alternatives

**Jobs to be Done**:
1. **Evaluate tradeoffs** — "Should I use LSM tree or B-tree for my workload?"
2. **Experiment fast** — "Can I test 5 designs in a day?"
3. **Make data-driven decisions** — "Show me which design has lower p99 latency"
4. **Learn by doing** — "I want to understand why MongoDB uses WiredTiger"

**Success = They try 3+ designs before committing to implementation**

### Secondary Users (Post-MVP)

- **Database educators** — Teaching students how databases work
- **Database vendors** — Prototyping new features (e.g., CockroachDB testing new compaction)
- **Open source contributors** — Experimenting with PostgreSQL extensions

---

## Part 3: What Success Looks Like

### North Star Metric

**Time from idea to validated design decision**

- **Today**: 2-4 weeks (for one alternative)
- **Target**: 2-4 hours (for 5 alternatives)

### Key Metrics (MVP)

| Metric | Target | Why It Matters |
|--------|--------|----------------|
| Designs created per user | 3+ | Proves experimentation is happening |
| Time to first working design | <30 min | Onboarding friction |
| Design comparisons run | 2+ per user | Proves decision-making utility |
| Blocks used per design | 5-8 | Sweet spot (not too simple, not too complex) |
| Custom blocks created | 1+ by 10% of users | Extensibility validation |

### What "Good" Looks Like (6 months)

**User story**:
> "I was designing a time-series database for IoT data. In 3 hours, I tested LSM vs B-tree, tried 4 compaction strategies, and discovered that columnar storage with bitmap indexes was 10x faster for my queries. I shipped that design to prod and it works."

**Anti-success story** (what we don't want):
> "Cool tool, played with it for an hour, but didn't learn anything I couldn't have Googled."

---

## Part 4: The Solution (High Level)

### Core Concept

**"Lego for Databases"** — Snap together storage engines, indexes, concurrency protocols, and buffers to build custom database architectures.

### The Three-Sentence Pitch

1. Drag blocks (LSM tree, B-tree index, MVCC) onto a canvas and connect them
2. Run your workload against the design (inserts, queries, transactions)
3. See metrics instantly (throughput, latency, bottlenecks) and iterate

### The Key Insight

Database architecture is **composition of independent components**:
- Storage (how data is stored)
- Indexing (how data is found)
- Concurrency (how conflicts are resolved)
- Buffering (what stays in memory)
- Query execution (how queries run)

**Today**: These are tangled in code (hard to swap)
**Our approach**: Make them visual blocks (easy to swap)

---

## Part 5: What We're Building (MVP)

### The One Thing That Matters Most

**Users can compare two database designs side-by-side and see which performs better for their workload.**

Everything else is in service of this.

### MVP Scope (8 weeks)

#### What's In

**Block Library** (15 blocks):
- 3 Storage Engines: Heap File, Clustered B-tree, LSM Tree
- 3 Indexes: B-tree, Hash, Skip List
- 2 Buffers: LRU, Clock
- 2 Concurrency: 2PL, MVCC
- 5 Query Ops: Sequential Scan, Index Scan, Nested Loop Join, Hash Join, Sort

**Canvas**:
- Drag blocks onto canvas
- Connect blocks (with type checking)
- Configure block parameters

**Simulation**:
- Define simple workload (inserts, selects, updates)
- Run workload against design
- Collect metrics (throughput, latency, IO count, cache hits)

**Comparison**:
- Run two designs side-by-side
- Show diff in metrics (Design A: 1000 QPS, Design B: 500 QPS)

#### What's Out (Post-MVP)

- Custom block creation (SDK)
- Real-time animation of data flow
- Distributed blocks (sharding, replication)
- Advanced query planning
- SQL syntax support
- Export to production code

### Why This Scope?

**Forcing function**: Can a user answer "LSM or B-tree for my use case?" in 30 minutes?

If yes → MVP is sufficient
If no → We missed something critical

---

## Part 6: How It Works (User Flow)

### Happy Path (30-minute session)

```
1. Choose a template (OLTP, OLAP, Time-Series)
   ↓
2. Customize blocks
   - Swap LSM tree for B-tree
   - Add covering index
   - Adjust buffer size
   ↓
3. Define workload
   - 70% reads, 30% writes
   - 100 concurrent users
   - Zipfian distribution (hot keys)
   ↓
4. Run simulation
   - Execute 10K queries
   - Collect metrics
   ↓
5. Compare alternative
   - Create second design (variant)
   - Run same workload
   ↓
6. Make decision
   - Design A: 1200 QPS, p99 = 45ms
   - Design B: 800 QPS, p99 = 20ms
   - Choose based on requirements
```

**Time**: 30 minutes (vs 2 weeks today)

---

## Part 7: Key Design Decisions

### Decision 1: Visual vs Code-Based

**Choice**: Visual (blocks + canvas)

**Why**:
- ✅ Faster to iterate (drag-drop vs write-compile-run)
- ✅ Lower friction (no setup, runs in browser)
- ✅ Forces clarity (explicit connections = explicit dependencies)
- ❌ Limited expressiveness (can't do everything)

**Tradeoff**: Power users may want code. We accept this.

**Mitigation**: Post-MVP, allow custom blocks with code.

---

### Decision 2: Accuracy vs Speed

**Choice**: Simplified simulation (80% accurate, 10x faster)

**Why**:
- ✅ Fast feedback loop (run 100 simulations in a day)
- ✅ Good enough for design decisions (relative perf matters, not absolute)
- ❌ Not production-ready predictions

**Example**: We'll simulate B-tree lookups as O(log n) page reads, not byte-level page layout.

**Tradeoff**: Not a replacement for real benchmarks.

**Mitigation**: Clear disclaimer + "validate in prod" step.

---

### Decision 3: Block Granularity

**Choice**: Medium granularity (not too atomic, not too coarse)

**Why**:
- ✅ Easy to understand (one block = one concept)
- ✅ Flexible enough (can compose different designs)
- ❌ Not perfect for every use case

**Example**:
- ✅ Good: "B-tree Index" block
- ❌ Too atomic: "B-tree Node" block
- ❌ Too coarse: "PostgreSQL Storage" block

**Tradeoff**: Advanced users may want finer control.

**Mitigation**: Block parameters for tuning (fanout, fill factor).

---

### Decision 4: Where to Start (Domain)

**Choice**: OLTP first (write-heavy and read-heavy scenarios)

**Why**:
- ✅ Most common use case (every company has OLTP workloads)
- ✅ Well-understood (B-tree vs LSM is clear tradeoff)
- ✅ Easier to validate (known benchmarks like YCSB)

**Out of scope (for now)**:
- ❌ OLAP (columnar storage, vectorized execution)
- ❌ Distributed (sharding, replication, consensus)
- ❌ Streaming (event sourcing, real-time)

**Tradeoff**: Won't help with analytics use cases initially.

**Mitigation**: Add OLAP blocks in Phase 2.

---

## Part 8: Risks & Mitigations

### Risk 1: Users don't understand blocks

**Risk**: Blocks are too abstract or poorly documented

**Likelihood**: High 🔴
**Impact**: High (no adoption)

**Mitigation**:
- Pre-built templates (OLTP, OLAP) so users start from working designs
- Inline help text for every block (what it does, when to use it)
- Tutorial mode (guided walkthrough)

**Validation**: User testing with 5 database engineers (can they build a design in 30 min?)

---

### Risk 2: Simulation is too slow

**Risk**: Running 10K queries takes 5 minutes (users lose patience)

**Likelihood**: Medium 🟡
**Impact**: High (no iteration)

**Mitigation**:
- Optimize hot paths (B-tree lookup, page allocation)
- Use Web Workers (parallelize simulation)
- Sample data (simulate 10K records, extrapolate to 1M)

**Validation**: Benchmark target is <5 seconds for 10K queries

---

### Risk 3: Results don't match real databases

**Risk**: Simulator says "Design A is faster" but in prod, Design B wins

**Likelihood**: Medium 🟡
**Impact**: High (trust lost)

**Mitigation**:
- Validate against known benchmarks (YCSB, TPC-C)
- Clear disclaimers ("simulation, not production")
- Show confidence intervals ("±20% margin")

**Validation**: Run YCSB workload, compare to published PostgreSQL/MySQL/RocksDB numbers

---

### Risk 4: Not differentiated enough

**Risk**: Users think "this is just a toy" (can't do real work)

**Likelihood**: Low 🟢
**Impact**: Medium (slow adoption)

**Mitigation**:
- Focus on "speed to decision" (not "replace production DB")
- Case studies from real users (e.g., "we chose LSM because of this tool")
- Integration with real tools (export to RocksDB config)

**Validation**: At least 1 user makes a prod decision based on simulator

---

## Part 9: Non-Goals (What We're NOT Doing)

### Explicitly Out of Scope

1. **Generate production code** — We won't output a working database implementation
   - Why: Too complex, too many edge cases
   - Alternative: Export design as documentation + config

2. **Replace real benchmarking** — We're not TPC-C/YCSB
   - Why: Simulation ≠ reality
   - Alternative: We're the "rough draft" before benchmarks

3. **Support every database feature** — No triggers, views, stored procedures
   - Why: Focusing on core architecture, not application features
   - Alternative: Add incrementally if users ask

4. **Be a database course** — We won't teach basic SQL or relational theory
   - Why: Assumes user already knows database internals
   - Alternative: Link to external resources (CMU course, Database Internals book)

5. **Distributed systems (MVP)** — No sharding, replication, consensus
   - Why: Single-node is 80% of decisions
   - Alternative: Phase 2 feature

---

## Part 10: Open Questions

### Questions We Need to Answer (Before MVP)

1. **Block interface design**
   - Q: How do blocks communicate (push vs pull)?
   - Impact: Affects performance and complexity
   - Need: Prototype 2-3 approaches, benchmark

2. **Workload definition UX**
   - Q: SQL syntax or custom DSL?
   - Impact: Usability vs flexibility
   - Need: User testing with 3 database engineers

3. **Metrics that matter**
   - Q: Which metrics do users actually care about?
   - Impact: Dashboard design
   - Need: Interview 5 database architects

4. **Validation strategy**
   - Q: How do we prove results are "accurate enough"?
   - Impact: Trust and adoption
   - Need: Run YCSB, compare to published benchmarks

---

## Part 11: Success Criteria (Go/No-Go)

### After 8 Weeks (MVP Done)

**Go Signal** (continue to next phase):
- ✅ 10+ database engineers tested it
- ✅ At least 3 created ≥2 designs and compared them
- ✅ At least 1 said "this changed my mind about a design choice"
- ✅ Simulation results within 30% of known benchmarks (YCSB)
- ✅ Median time to first design: <30 minutes

**No-Go Signal** (pivot or kill):
- ❌ Users don't understand blocks (need >1 hour of hand-holding)
- ❌ Simulation is too slow (>30 seconds for 10K queries)
- ❌ Results are wildly inaccurate (>50% off from benchmarks)
- ❌ Users say "cool demo, but won't use it for real work"

---

## Part 12: Roadmap

### Phase 1: MVP (Week 1-8)

**Goal**: Validate core concept (visual composition + simulation)

**Deliverables**:
- Block library (15 blocks)
- Visual canvas (drag, drop, connect)
- Basic simulation engine
- Metrics dashboard
- Side-by-side comparison

**Success**: 1 user makes a design decision based on the tool

---

### Phase 2: Depth (Week 9-16)

**Goal**: Make it useful for more use cases

**Deliverables**:
- 20 more blocks (OLAP, compression, partitioning)
- Advanced workload patterns (YCSB, TPC-C)
- Block SDK (users can create custom blocks)
- Export designs (JSON, config files)

**Success**: 10 users actively using it weekly

---

### Phase 3: Community (Week 17-24)

**Goal**: Enable ecosystem

**Deliverables**:
- Block marketplace (share custom blocks)
- Template library (OLTP, OLAP, IoT, etc.)
- Documentation generator (auto-docs from blocks)
- Integration with real tools (RocksDB config export)

**Success**: 50+ custom blocks created by users

---

## Part 13: Why This Will Work

### The Thesis

**Most database design decisions are made with incomplete information because experimentation is too expensive.**

If we make experimentation 10x faster, people will:
1. Try more alternatives
2. Make better decisions
3. Ship faster

### The Moat (Why can't someone else do this?)

1. **Domain expertise** — Requires deep understanding of database internals (not just UI)
2. **Block library** — Hard to build high-quality, composable blocks (takes years)
3. **Network effects** — As users create custom blocks, library grows (shared value)

### The Wedge (How do we get initial users?)

1. **Target database researchers first** — They're experimenting anyway (papers require prototypes)
2. **Publish case studies** — "How we chose LSM over B-tree in 2 hours"
3. **Partner with educators** — Use in CMU/Stanford database courses

---

## Part 14: How We'll Build This

### Team (Ideal)

- 1 Product Engineer (full-stack, TypeScript, React)
- 1 Database Systems Expert (implement blocks, ensure correctness)
- 1 Designer (UX for canvas, visual design)

### Tech Stack (Locked)

**Frontend**:
- React + TypeScript (UI)
- React Flow (visual canvas)
- Recharts (metrics visualization)
- Tailwind CSS (styling)

**Simulation**:
- TypeScript (core engine, runs in browser)
- Web Workers (parallelization)
- IndexedDB (local persistence)

**Why browser-based?**
- Zero friction (no install)
- Shareable (send link, not code)
- Fast iteration (deploy instantly)

### Build Strategy

**Week 1-2**: Canvas + 3 blocks (Heap Storage, B-tree Index, Sequential Scan)
**Week 3-4**: Workload runner + metrics
**Week 5-6**: 10 more blocks + comparison UI
**Week 7-8**: Polish + user testing

---

## Part 15: The Hard Parts

### What We're Worried About

1. **Complexity explosion** — As we add more blocks, wiring becomes spaghetti
   - Mitigation: Force modular interfaces, constrain connections

2. **Performance ceiling** — Browser can't handle large-scale simulations
   - Mitigation: Sample data, coarse-grained simulation

3. **Adoption friction** — Users don't know where to start
   - Mitigation: Templates + guided tutorials

4. **Accuracy vs simplicity** — Too simple = not useful, too complex = too slow
   - Mitigation: Make accuracy tunable (fast mode vs accurate mode)

---

## Part 16: What Would Make This 10x Better?

### If We Had Infinite Resources

1. **AI Co-Pilot** — "I have a write-heavy workload, suggest a design"
2. **Real Database Export** — Generate actual RocksDB/PostgreSQL code
3. **Cloud Integration** — Deploy design to AWS/GCP, run real benchmarks
4. **Collaborative Editing** — Multiple users design together (Figma-like)
5. **Version Control** — Git for database designs (track changes, branch, merge)

### If We Had Zero Resources

**The scrappiest MVP**:
- 3 blocks (Heap, B-tree, Sequential Scan)
- Text-based config (no visual canvas)
- CLI tool (not web app)
- One workload (YCSB Workload A)

**Would this work?** Maybe. But misses the core insight (visual = fast).

---

## Part 17: Why You Should Care

### For Database Architects

**Today**: You're copying PostgreSQL's design because you don't have time to explore alternatives.

**Tomorrow**: You try 5 designs in an afternoon and ship the optimal one.

### For Database Researchers

**Today**: You spend 3 months implementing a paper's idea to see if it's practical.

**Tomorrow**: You implement it as a block in 1 week and test it against 10 workloads.

### For Educators

**Today**: Students read about B-trees vs LSM trees but never see them in action.

**Tomorrow**: Students build both in 30 minutes and understand the tradeoffs viscerally.

---

## Part 18: The Riskiest Assumption

**"People will trust a simulator's results enough to make real decisions."**

If this is false, nothing else matters.

### How We'll Validate

1. Run YCSB workload, compare to published PostgreSQL results
2. Interview 5 database architects: "Would you trust this?"
3. Publish methodology (how we simulate, what we simplify)
4. Add confidence intervals to all metrics

**Success = At least 1 user says: "I chose Design B because the simulator showed it was 2x faster."**

---

## Part 19: What We're Optimizing For

### The North Star

**Speed to validated design decision.**

Not:
- ❌ Number of features
- ❌ Accuracy of simulation
- ❌ Complexity of designs supported

But:
- ✅ Time from "I have a use case" to "I know which design is best"

### The Core Loop

```
Design → Simulate → Compare → Decide
   ↑                             ↓
   └─────────────────────────────┘
        (10x faster than today)
```

If we make this loop fast, we win.

---

## Part 20: TL;DR (The Cliff Notes)

**Problem**: Database design requires months of coding before you know if it works.

**Solution**: Visual database construction kit — snap together blocks, run workloads, see results instantly.

**Why now**: Visual tools work (Figma, Unreal), WebAssembly is fast, database research is exploding.

**Who**: Database architects designing new systems or replatforming.

**Success**: Users try 3+ designs before committing to one (vs 1 today).

**MVP**: 15 blocks, visual canvas, workload runner, metrics, side-by-side comparison. 8 weeks.

**Riskiest assumption**: People will trust the simulator's results.

**Validation**: At least 1 user makes a real design decision based on the tool.

**Moat**: Domain expertise + block library + network effects.

**Why this matters**: Every company with scale hits database limits. This makes experimentation 10x faster.

---

**End of PRD**

---

## Appendix A: Block Details

### Storage Engine Blocks

#### 1. Heap File Storage
- **What it does**: Stores records in insertion order (unordered)
- **When to use**: Simple, no clustering needed
- **Tradeoffs**: Fast inserts, slow scans without index
- **Params**: page_size, fill_factor
- **Metrics**: pages_written, fragmentation
- **References**: PostgreSQL heap files

#### 2. Clustered B-tree Storage
- **What it does**: Stores records sorted by primary key
- **When to use**: Frequent range scans on primary key
- **Tradeoffs**: Slower inserts (reordering), fast scans
- **Params**: fanout, fill_factor
- **Metrics**: page_splits, tree_height
- **References**: MySQL InnoDB

#### 3. LSM Tree Storage
- **What it does**: Write-optimized log-structured storage
- **When to use**: High insert/update rate
- **Tradeoffs**: Fast writes, slower reads (multiple levels)
- **Params**: memtable_size, compaction_strategy
- **Metrics**: write_amplification, compactions
- **References**: RocksDB, LevelDB

### Index Blocks

#### 4. B-tree Index
- **What it does**: Balanced tree index (supports range queries)
- **When to use**: Range scans, sorted access
- **Tradeoffs**: Slower inserts (rebalancing), fast lookups
- **Params**: fanout, unique, covering_columns
- **Metrics**: lookup_time, page_splits
- **References**: PostgreSQL default index

#### 5. Hash Index
- **What it does**: Hash table for equality lookups
- **When to use**: Only equality predicates (WHERE id = 123)
- **Tradeoffs**: Fast equality, no range scans
- **Params**: bucket_count, load_factor
- **Metrics**: collisions, lookup_time
- **References**: PostgreSQL hash index

#### 6. Skip List Index
- **What it does**: Probabilistic balanced structure
- **When to use**: Concurrent inserts, simpler than B-tree
- **Tradeoffs**: Simpler code, slightly slower than B-tree
- **Params**: max_level, probability
- **Metrics**: lookup_time, avg_level
- **References**: Redis sorted sets, LevelDB

### Buffer Management Blocks

#### 7. LRU Buffer Pool
- **What it does**: Least recently used eviction
- **When to use**: General-purpose caching
- **Tradeoffs**: Simple, but struggles with scans
- **Params**: size, eviction_batch
- **Metrics**: hit_ratio, evictions
- **References**: Classic caching

#### 8. Clock Buffer Pool
- **What it does**: Second-chance algorithm (approximates LRU)
- **When to use**: Better than LRU for sequential scans
- **Tradeoffs**: More complex, handles scans better
- **Params**: size, hand_speed
- **Metrics**: hit_ratio, evictions
- **References**: PostgreSQL buffer pool

### Concurrency Blocks

#### 9. Two-Phase Locking (2PL)
- **What it does**: Pessimistic locking (acquire all locks before release)
- **When to use**: Strong consistency, low contention
- **Tradeoffs**: Simple, but can deadlock
- **Params**: lock_granularity (row/page/table), timeout
- **Metrics**: lock_waits, deadlocks
- **References**: Traditional RDBMS

#### 10. MVCC
- **What it does**: Multi-version concurrency (readers don't block writers)
- **When to use**: High read concurrency
- **Tradeoffs**: More complex, requires garbage collection
- **Params**: isolation_level, gc_interval
- **Metrics**: active_versions, gc_runs
- **References**: PostgreSQL, MySQL InnoDB

### Query Execution Blocks

#### 11. Sequential Scan
- **What it does**: Read all records in order
- **When to use**: No index, or reading >10% of table
- **Tradeoffs**: Simple, but slow for large tables
- **Params**: prefetch_size
- **Metrics**: pages_read, time
- **References**: Universal

#### 12. Index Scan
- **What it does**: Use index to find specific records
- **When to use**: Selective queries (reading <10% of table)
- **Tradeoffs**: Fast for selective, overhead for non-selective
- **Params**: index_name
- **Metrics**: index_lookups, pages_read
- **References**: Universal

#### 13. Nested Loop Join
- **What it does**: For each row in outer table, scan inner table
- **When to use**: Small tables, or when inner has index
- **Tradeoffs**: Simple, but O(N*M) without index
- **Params**: None
- **Metrics**: rows_scanned, time
- **References**: Universal

#### 14. Hash Join
- **What it does**: Build hash table of one side, probe with other
- **When to use**: Equality joins, large tables
- **Tradeoffs**: Memory-intensive, fast for equality
- **Params**: hash_table_size
- **Metrics**: hash_collisions, time
- **References**: PostgreSQL, MySQL

#### 15. Sort Operator
- **What it does**: Sort records (for ORDER BY, merge join)
- **When to use**: Explicit sort or merge join prep
- **Tradeoffs**: Memory-intensive, may spill to disk
- **Params**: memory_limit, sort_method
- **Metrics**: rows_sorted, spill_to_disk
- **References**: Universal

---

## Appendix B: Example Use Cases

### Use Case 1: E-commerce Product Catalog

**Requirements**:
- 10M products
- 1000 QPS reads (product lookups)
- 10 QPS writes (price updates)
- Query pattern: SELECT * FROM products WHERE id = ?

**Design A: Heap + B-tree Index**
```
Schema → Heap File Storage → B-tree Index (on id) → Index Scan → Results
```
- Throughput: 1200 QPS
- p99 latency: 8ms
- Cache hit: 85%

**Design B: Clustered B-tree**
```
Schema → Clustered B-tree Storage → Index Scan → Results
```
- Throughput: 1500 QPS
- p99 latency: 5ms
- Cache hit: 92%

**Decision**: Design B wins (fewer page reads, better locality)

---

### Use Case 2: Time-Series IoT Data

**Requirements**:
- 100K sensors
- 50K inserts/sec
- 10 reads/sec (recent data only)
- Query pattern: INSERT INTO sensor_data (...) VALUES (...)

**Design A: Heap + B-tree Index**
```
Schema → Heap File Storage → B-tree Index → Results
```
- Throughput: 15K inserts/sec (bottleneck: B-tree rebalancing)
- Write amplification: 8x

**Design B: LSM Tree**
```
Schema → LSM Tree Storage → Skip List Index (in-memory) → Results
```
- Throughput: 45K inserts/sec
- Write amplification: 3x (with tiered compaction)

**Decision**: Design B wins (write-optimized)

---

### Use Case 3: Analytics Workload

**Requirements**:
- 1B records
- Complex aggregations (SUM, AVG, GROUP BY)
- Query pattern: SELECT country, AVG(revenue) FROM orders GROUP BY country

**Design A: Heap + B-tree**
```
Schema → Heap File Storage → Sequential Scan → Results
```
- Query time: 45 seconds (must read all rows)

**Design B: Columnar Storage + Bitmap Index**
```
Schema → Columnar Storage → Bitmap Index (on country) → Results
```
- Query time: 2 seconds (only read 'country' and 'revenue' columns)

**Decision**: Design B wins (50x faster for analytics)

---

**End of Document**
