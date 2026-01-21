# 8-Week Roadmap: Modular DB Builder (MVP)

**Last Updated**: 2026-01-20
**Duration**: 8 weeks (56 days)
**Team**: 1-2 engineers + 1 designer (or full-stack + design skills)
**Goal**: Ship a working MVP where users can compare two database designs and make data-driven decisions

---

## Roadmap Overview

```
Week 1-2: Foundation (Canvas + 3 Blocks)
Week 3-4: Execution (Workload + Metrics)
Week 5-6: Breadth (12 More Blocks + Comparison)
Week 7-8: Polish (UX + User Testing)
```

---

## Week 1-2: Foundation
**Goal**: Users can place blocks on a canvas and connect them

### Week 1: Canvas Infrastructure

#### Monday - Wednesday
**Task**: Set up project + visual canvas
**Deliverables**:
- [ ] React + TypeScript project scaffolding
- [ ] React Flow integration
- [ ] Basic canvas (drag, drop, pan, zoom)
- [ ] Block palette (sidebar with 3 placeholder blocks)

**Technical Details**:
- Use Vite or Next.js for fast dev setup
- React Flow for node-based editor
- Tailwind CSS for styling
- TypeScript strict mode

**Acceptance Criteria**:
- User can drag a block from palette onto canvas
- User can pan and zoom canvas
- Blocks snap to grid (optional but nice)

#### Thursday - Friday
**Task**: Define block system architecture
**Deliverables**:
- [ ] TypeScript interfaces for Block, Port, Connection
- [ ] Block definition schema (inputs, outputs, parameters)
- [ ] Connection validation logic (type checking)

**Code Structure**:
```typescript
// src/types/block.ts
interface Block {
  id: string;
  type: string;
  name: string;
  category: BlockCategory;
  inputs: Port[];
  outputs: Port[];
  parameters: Parameter[];
}

interface Port {
  name: string;
  type: PortType;
  dataType: DataType;
}

// src/types/connection.ts
interface Connection {
  id: string;
  source: { blockId: string; portName: string };
  target: { blockId: string; portName: string };
}
```

**Acceptance Criteria**:
- Block definitions are type-safe
- Can validate if two ports can connect
- Error messages for invalid connections

---

### Week 2: First Working Blocks

#### Monday - Tuesday
**Task**: Implement 3 core blocks
**Deliverables**:
- [ ] Block 1: Schema Definition (input: none, output: table metadata)
- [ ] Block 2: Heap File Storage (input: records, output: stored records)
- [ ] Block 3: Sequential Scan (input: scan request, output: records)

**Implementation**:
```typescript
// src/blocks/storage/HeapFileStorage.ts
class HeapFileStorage implements BlockImplementation {
  private pages: Page[] = [];

  execute(inputs: BlockInputs): BlockOutputs {
    // Simple in-memory heap file simulation
    const records = inputs.records;
    const pages = this.storeRecords(records);
    return { stored_records: records, pages };
  }
}
```

**Acceptance Criteria**:
- User can connect Schema → Heap Storage → Sequential Scan
- Data flows through blocks (mock data for now)
- Console log shows data flowing

#### Wednesday - Friday
**Task**: Parameter configuration UI
**Deliverables**:
- [ ] Parameter panel (right sidebar)
- [ ] Input fields for each parameter type (number, boolean, string)
- [ ] Real-time parameter updates

**UI Components**:
- NumberInput (with min/max validation)
- Checkbox (for booleans)
- Select dropdown (for enums)

**Acceptance Criteria**:
- Click a block → parameter panel shows
- Change parameter → block state updates
- Validation errors show inline

---

## Week 3-4: Execution Engine
**Goal**: Run workloads and collect metrics

### Week 3: Workload Runner

#### Monday - Tuesday
**Task**: Workload definition system
**Deliverables**:
- [ ] Workload schema (queries, operations, distribution)
- [ ] Workload editor UI (simple form-based)
- [ ] Support INSERT and SELECT operations

**Workload Format**:
```typescript
interface Workload {
  name: string;
  operations: Operation[];
  concurrency: number;
  distribution: 'uniform' | 'zipfian';
}

interface Operation {
  type: 'INSERT' | 'SELECT';
  template: string;
  weight: number; // percentage
}
```

**UI**:
- Modal or bottom panel for workload definition
- Add/remove operations
- Slider for read/write ratio

**Acceptance Criteria**:
- User can define 2-3 operations
- User can set distribution (uniform vs zipfian)
- Workload saved in design state

#### Wednesday - Friday
**Task**: Execution engine (first version)
**Deliverables**:
- [ ] Compile canvas into executable graph
- [ ] Topological sort (execution order)
- [ ] Execute 10K operations sequentially
- [ ] Show progress indicator

**Execution Flow**:
```
1. Validate canvas (all inputs connected)
2. Build execution graph
3. Initialize blocks (allocate buffers, trees)
4. For each operation:
   - Send to entry block (Schema)
   - Data flows through connections
   - Blocks process and emit outputs
5. Collect metrics from each block
```

**Acceptance Criteria**:
- Run button executes workload
- Progress bar shows completion
- Takes <10 seconds for 10K operations
- No crashes or errors

---

### Week 4: Metrics & Visualization

#### Monday - Tuesday
**Task**: Metrics collection system
**Deliverables**:
- [ ] Metrics collector (centralized)
- [ ] Block-level metrics (each block reports)
- [ ] System-level aggregation

**Metrics to Collect**:
- Heap Storage: pages_written, pages_read
- Sequential Scan: records_scanned, time_taken
- System: total_time, throughput (ops/sec)

**Implementation**:
```typescript
class MetricsCollector {
  private metrics: Map<string, Metric> = new Map();

  recordCounter(name: string, value: number): void {
    // Increment counter
  }

  recordHistogram(name: string, value: number): void {
    // Track distribution
  }

  getMetrics(): MetricsSummary {
    // Aggregate and return
  }
}
```

**Acceptance Criteria**:
- Each block emits metrics during execution
- Metrics persisted after execution completes
- Can query metrics programmatically

#### Wednesday - Friday
**Task**: Metrics dashboard UI
**Deliverables**:
- [ ] Metrics panel (bottom or right)
- [ ] Show key metrics (throughput, latency, IO)
- [ ] Simple bar charts (Recharts library)

**Dashboard Layout**:
```
┌─────────────────────────────────────┐
│ Execution Results                   │
├─────────────────────────────────────┤
│ Throughput: 1,234 ops/sec           │
│ Total Time: 8.1s                    │
│ Pages Written: 542                  │
│ Pages Read: 1,234                   │
├─────────────────────────────────────┤
│ Block Breakdown:                    │
│  ├─ Heap Storage: 4.2s (52%)        │
│  ├─ Sequential Scan: 3.1s (38%)     │
│  └─ Other: 0.8s (10%)               │
└─────────────────────────────────────┘
```

**Acceptance Criteria**:
- Metrics render after execution
- Charts are readable and accurate
- User can understand bottlenecks

---

## Week 5-6: Breadth (More Blocks)
**Goal**: Add 12 more blocks to enable real comparisons

### Week 5: Core Building Blocks

#### Monday - Tuesday
**Task**: Implement 4 storage/index blocks
**Deliverables**:
- [ ] Clustered B-tree Storage
- [ ] LSM Tree Storage
- [ ] B-tree Index
- [ ] Hash Index

**Focus**: Get the core algorithms right
- B-tree: Insert, lookup, page splits
- LSM tree: Memtable, SSTable, compaction
- Hash index: Bucket allocation, collision handling

**Acceptance Criteria**:
- Each block passes unit tests
- Can swap Heap Storage for Clustered B-tree
- Metrics show performance differences

#### Wednesday - Thursday
**Task**: Implement 4 buffer/concurrency blocks
**Deliverables**:
- [ ] LRU Buffer Pool
- [ ] Clock Buffer Pool
- [ ] Two-Phase Locking
- [ ] MVCC

**Focus**: Simulate behavior, not perfect accuracy
- LRU: Simple eviction on cache miss
- 2PL: Acquire locks, detect deadlocks
- MVCC: Version chains, snapshot reads

**Acceptance Criteria**:
- Buffer blocks reduce IO when cache hit
- Concurrency blocks serialize conflicting transactions

#### Friday
**Task**: Implement 4 query execution blocks
**Deliverables**:
- [ ] Index Scan
- [ ] Nested Loop Join
- [ ] Hash Join
- [ ] Sort Operator

**Focus**: Get join algorithms right (most educational)

**Acceptance Criteria**:
- Can build designs with joins
- Hash join faster than nested loop for large tables

---

### Week 6: Comparison Feature

#### Monday - Tuesday
**Task**: Multi-design support
**Deliverables**:
- [ ] Create multiple designs (tabs or workspace selector)
- [ ] Copy/duplicate design
- [ ] Switch between designs

**UI**:
- Tabs at top: "Design A" | "Design B" | "+ New"
- Duplicate button creates copy
- Each design has independent state

**Acceptance Criteria**:
- User can create 2+ designs
- Designs are independent (changing one doesn't affect others)
- Can delete designs

#### Wednesday - Thursday
**Task**: Side-by-side comparison UI
**Deliverables**:
- [ ] Compare mode (split screen)
- [ ] Run workload on both designs simultaneously
- [ ] Show metrics diff

**Comparison View**:
```
┌──────────────────┬──────────────────┐
│   Design A       │   Design B       │
│ (Heap + B-tree)  │ (LSM Tree)       │
├──────────────────┼──────────────────┤
│ 1,200 ops/sec    │ 1,800 ops/sec    │
│ 8.3s total       │ 5.5s total       │
│ 542 pages write  │ 234 pages write  │
├──────────────────┼──────────────────┤
│                  │ ✓ 50% faster     │
│                  │ ✓ 57% less IO    │
└──────────────────┴──────────────────┘
```

**Acceptance Criteria**:
- User clicks "Compare" → enters split screen
- Both designs execute same workload
- Diff highlights winner

#### Friday
**Task**: Template library
**Deliverables**:
- [ ] 3 pre-built templates (OLTP, Write-Heavy, Read-Heavy)
- [ ] "New from Template" flow
- [ ] Template descriptions

**Templates**:
1. **OLTP (Balanced)**: Heap + B-tree + LRU + 2PL
2. **Write-Heavy**: LSM Tree + Skip List + Clock + MVCC
3. **Read-Heavy**: Clustered B-tree + Covering Index + LRU + MVCC

**Acceptance Criteria**:
- User can start from template
- Templates are working designs (not empty)
- User can customize after starting from template

---

## Week 7-8: Polish & Testing
**Goal**: Make it usable and validate with real users

### Week 7: UX Polish

#### Monday - Tuesday
**Task**: Onboarding & help system
**Deliverables**:
- [ ] First-time user tutorial (guided walkthrough)
- [ ] Inline help for blocks (tooltips, descriptions)
- [ ] Example workloads (YCSB Workload A, B)

**Tutorial Flow**:
```
1. Welcome screen → "Let's build your first database"
2. Start from template (OLTP)
3. Guided: "Try changing Heap to LSM Tree"
4. Guided: "Now run the workload"
5. Guided: "See the metrics? LSM is faster for writes!"
6. Done → "Now try comparing two designs"
```

**Acceptance Criteria**:
- New user can complete tutorial in <10 minutes
- Tutorial is skippable
- Help text answers basic questions

#### Wednesday - Thursday
**Task**: Error handling & validation
**Deliverables**:
- [ ] Validate canvas before execution (all inputs connected)
- [ ] Error messages for invalid connections
- [ ] Handle execution errors gracefully

**Error Scenarios**:
- Missing connection: "Sequential Scan needs input from Storage"
- Type mismatch: "Cannot connect INDEX_LOOKUP to RECORD_STREAM"
- Execution error: "Workload failed at operation 1,234"

**Acceptance Criteria**:
- No silent failures
- Error messages are actionable
- User can fix errors based on message

#### Friday
**Task**: Performance optimization
**Deliverables**:
- [ ] Profile execution (identify bottlenecks)
- [ ] Optimize hot paths (B-tree lookup, page allocation)
- [ ] Target: <5 seconds for 10K operations

**Optimization Strategies**:
- Use Web Workers for heavy computation
- Batch operations where possible
- Pre-allocate data structures

**Acceptance Criteria**:
- 10K operations complete in <5 seconds
- UI remains responsive during execution

---

### Week 8: User Testing & Launch Prep

#### Monday - Tuesday
**Task**: Alpha testing with 5 users
**Deliverables**:
- [ ] Recruit 5 database engineers (friends, Twitter, LinkedIn)
- [ ] Give them task: "Design a database for write-heavy workload"
- [ ] Observe and take notes

**Questions to Answer**:
- Can they complete the task in <30 minutes?
- What blocks do they struggle with?
- What metrics do they care about?
- What features do they ask for?

**Acceptance Criteria**:
- At least 3 users complete the task
- Collect 10+ pieces of feedback
- Prioritize top 3 issues for fixes

#### Wednesday - Thursday
**Task**: Fixes based on feedback
**Deliverables**:
- [ ] Fix top 3 UX issues
- [ ] Add missing blocks (if critical)
- [ ] Improve documentation/help

**Expected Fixes** (based on common issues):
- Clarify block descriptions
- Add missing workload patterns
- Improve metrics visualization

**Acceptance Criteria**:
- Re-test with 1-2 users
- Issues are resolved

#### Friday
**Task**: Launch prep
**Deliverables**:
- [ ] Polish landing page / README
- [ ] Record demo video (3-5 min)
- [ ] Write launch post (Twitter, HN, Reddit)
- [ ] Deploy to Vercel/Netlify

**Launch Assets**:
- Demo video showing: Start from template → Swap blocks → Compare results
- Screenshot: Side-by-side comparison showing clear winner
- One-liner: "Design and compare custom database architectures in 30 minutes"

**Acceptance Criteria**:
- Live URL works
- Demo video is compelling
- Launch post is ready

---

## Deliverables Summary

### End of Week 2
✅ Canvas with 3 blocks (Schema, Heap Storage, Sequential Scan)
✅ Can connect blocks
✅ Parameter configuration works

### End of Week 4
✅ Workload runner (INSERT, SELECT)
✅ Execution engine (10K ops)
✅ Metrics dashboard

### End of Week 6
✅ 15 total blocks (storage, indexes, buffers, concurrency, query ops)
✅ Side-by-side comparison
✅ 3 templates (OLTP, Write-Heavy, Read-Heavy)

### End of Week 8
✅ Onboarding tutorial
✅ Error handling
✅ Alpha tested with 5 users
✅ Launched publicly

---

## Resource Requirements

### Engineering

**Full-Stack Engineer** (8 weeks full-time):
- Week 1-2: Canvas + block system
- Week 3-4: Execution engine + metrics
- Week 5-6: More blocks + comparison
- Week 7-8: Polish + testing

**OR**

**Two Engineers** (4 weeks each):
- Engineer A: Frontend (canvas, UI, metrics visualization)
- Engineer B: Backend (blocks, execution engine, algorithms)

### Design

**Designer** (2-3 weeks part-time):
- Week 1: Information architecture + wireframes
- Week 3: Metrics dashboard design
- Week 6: Comparison UI design
- Week 7: Onboarding flow design

**OR**

**Full-Stack Engineer with design skills** (no separate designer needed)

---

## Tech Stack (Finalized)

### Frontend
- **React 18** + **TypeScript 5**
- **React Flow** (canvas)
- **Recharts** (metrics visualization)
- **Tailwind CSS** (styling)
- **Zustand** (state management, simpler than Redux)
- **Vite** (build tool, fast HMR)

### Simulation Engine
- **TypeScript** (all blocks implemented in TS)
- **Web Workers** (optional, for performance)

### Storage
- **IndexedDB** (persist designs locally)
- **JSON export/import** (share designs)

### Deployment
- **Vercel** or **Netlify** (static site hosting)
- **GitHub Pages** (alternative if open source)

---

## Dependencies & Risks

### Critical Path

```
Week 1-2 (Canvas + Blocks)
    ↓
Week 3-4 (Execution + Metrics)
    ↓
Week 5-6 (More Blocks + Comparison)
    ↓
Week 7-8 (Polish + Testing)
```

**Any delay in Week 1-4 cascades to entire project.**

### Top Risks

| Risk | Mitigation |
|------|------------|
| **Execution too slow** (>10s for 10K ops) | Use Web Workers, optimize hot paths, sample data |
| **Block algorithms are wrong** | Validate against known benchmarks (YCSB), simplify if needed |
| **Users don't understand blocks** | Pre-built templates, inline help, tutorial |
| **Scope creep** (too many blocks) | Lock scope at 15 blocks, say no to everything else |

---

## Success Metrics (8-Week Mark)

### Quantitative
- [ ] 5+ alpha testers used the tool
- [ ] 3+ created ≥2 designs and compared them
- [ ] 1+ said "this influenced my design decision"
- [ ] Median time to first working design: <30 min
- [ ] Execution time: <5 seconds for 10K operations

### Qualitative
- [ ] Users understand what blocks do
- [ ] Users can find the blocks they need
- [ ] Metrics are clear and actionable
- [ ] Comparison UI clearly shows winner

### Launch
- [ ] Public URL live
- [ ] Demo video published
- [ ] Launch post on HN/Twitter/Reddit
- [ ] 10+ upvotes or retweets (initial signal)

---

## Post-MVP (Week 9+)

If MVP succeeds, prioritize:

1. **Custom block SDK** — Let users create blocks
2. **More blocks** — OLAP (columnar, vectorized), distribution (sharding)
3. **Advanced workloads** — YCSB, TPC-C benchmarks
4. **Real DB export** — Generate RocksDB config, PostgreSQL schema
5. **Collaboration** — Multi-user editing (Figma-like)

---

## Daily Standups (Suggested)

**Every morning, answer**:
1. What did I ship yesterday?
2. What will I ship today?
3. What's blocking me?

**Every Friday, answer**:
1. Did we hit this week's goal?
2. What do we cut/defer if we're behind?
3. What's the plan for next week?

---

## The One Thing That Cannot Slip

**By end of Week 4**, we must have:
- Working execution engine
- Metrics dashboard
- 3 blocks that demonstrate clear performance difference

**Why**: This is the core value prop. If this doesn't work, nothing else matters.

---

**End of Roadmap**
