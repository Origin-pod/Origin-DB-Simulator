# Wireframes: Modular DB Builder

**Last Updated**: 2026-01-20
**Status**: Concept Wireframes
**Fidelity**: Low-fidelity (structural layout, no visual design)

---

## Overview

These wireframes show the key screens and states for the Modular DB Builder MVP. Focus is on information architecture, user flows, and interaction patterns — not visual design.

**Design Principles Applied**:
- Clarity over aesthetics
- Progressive disclosure (show only what's needed)
- Clear visual hierarchy
- Obvious affordances (what's clickable is clear)
- Error prevention and recovery

---

## Screen 1: Main Canvas (Default State)

**Context**: User opens tool for first time or starts new design

```
┌────────────────────────────────────────────────────────────────────────┐
│  [🔷 DB Builder]    Untitled Design ⌄    [▶ Run]  [Compare]   [Share▾]│
├──────────┬──────────────────────────────────────────────────┬──────────┤
│          │                                                  │          │
│ BLOCKS   │                                                  │ (empty)  │
│          │                                                  │          │
│ [Search] │                                                  │          │
│          │         Drop blocks here to start                │          │
│ Storage ▾│              designing                           │          │
│  🟣 Heap │                                                  │          │
│  🟣 LSM  │                   [Templates ▾]                  │          │
│  🟣 B+   │                                                  │          │
│          │                                                  │          │
│ Indexes ▾│                                                  │          │
│  🔵 B-tr │                                                  │          │
│  🔵 Hash │                                                  │          │
│  🔵 Skip │                                                  │          │
│          │                                                  │          │
│ Buffers ▾│                                                  │          │
│  🟢 LRU  │                                                  │          │
│  🟢 Clck │                                                  │          │
│          │                                                  │          │
│ [Show]   │                                                  │          │
│ [15 blks]│                                                  │          │
│          │                                                  │          │
│          │                                                  │          │
│          │                                                  │          │
└──────────┴──────────────────────────────────────────────────┴──────────┘
```

**Elements**:

### Top Bar
- **Logo**: Brand identity, clickable → home
- **Design Name**: Editable inline (click to rename)
- **Dropdown**: Access templates, recent designs
- **Run Button**: Primary action (disabled until design is valid)
- **Compare Button**: Secondary action (disabled until 2+ designs exist)
- **Share Dropdown**: Export, generate link, save

### Left Sidebar (Block Library)
- **Search**: Filter blocks by name
- **Collapsible Categories**: Storage, Indexes, Buffers, Concurrency, Query Ops
- **Block Items**: Icon + Name (draggable)
- **Hover State**: Shows tooltip with description
- **Footer**: Toggle "Show All Blocks" (15 vs 50+ post-MVP)

### Center Canvas
- **Empty State**: Helpful message + call-to-action
- **Templates Button**: Quick start (OLTP, Write-Heavy, Read-Heavy)
- **Grid Background**: Subtle, helps with alignment
- **Zoom Controls**: (bottom-right, not shown) +/- and fit-to-screen

### Right Sidebar
- **Empty Initially**: Only appears when block is selected
- **Collapsible**: Can hide to maximize canvas space

**User Actions**:
1. Click "Templates" → Select template → Canvas populates
2. Drag block from library → Canvas
3. Search for block → Filtered list appears

**Design Rationale**:
- Empty state is welcoming, not intimidating
- Templates front-and-center (fastest path to working design)
- Block library uses familiar patterns (collapsible tree, icons)

---

## Screen 2: Canvas with Blocks (Active Design)

**Context**: User has placed blocks and is designing

```
┌────────────────────────────────────────────────────────────────────────┐
│  [🔷 DB Builder]    OLTP Database ⌄      [▶ Run]  [Compare]   [Share▾]│
├──────────┬──────────────────────────────────────────────────┬──────────┤
│          │                                                  │ B-tree   │
│ BLOCKS   │    ┌──────────┐                                 │ Index    │
│          │    │ Schema   │                                 │──────────│
│ [Search] │    │ Table:   │                                 │          │
│          │    │ users    │●──┐                             │ Columns: │
│ Storage ▾│    └──────────┘   │                             │ [x] id   │
│  🟣 Heap │                    │                             │ [ ] name │
│  🟣 LSM  │                    ↓                             │ [x] email│
│  🟣 B+   │    ┌──────────┐   ┌──────────┐                  │          │
│          │    │ Heap     │   │ B-tree   │                  │ Unique:  │
│ Indexes ▾│    │ Storage  │●─→│ Index    │◄─────(selected) │ [✓] Yes  │
│  🔵 B-tr │    │          │   │ (id)     │                  │          │
│  🔵 Hash │    └──────────┘   └──────────┘                  │ Fanout:  │
│  🔵 Skip │           │               │                      │ [128___] │
│          │           ↓               ↓                      │          │
│ Buffers ▾│    ┌──────────────────────────┐                 │ Covering:│
│  🟢 LRU  │    │     LRU Buffer Pool      │                 │ [+Add]   │
│  🟢 Clck │    │       (256 MB)           │                 │          │
│          │    └──────────────────────────┘                 │          │
│ [Show]   │                                                  │          │
│ [15 blks]│                                                  │ [Delete] │
│          │                                                  │          │
└──────────┴──────────────────────────────────────────────────┴──────────┘
```

**Elements**:

### Canvas (Active State)
- **Blocks**: Rounded rectangles with name and key info
- **Ports**: Small circles on edges (● = output, ◯ = input)
- **Connections**: Lines with arrows showing data flow direction
- **Selection**: Blue outline + shadow on selected block
- **Hover**: Subtle highlight + cursor change

### Block Anatomy
```
┌──────────────┐
│ Block Name   │  ← Title (bold)
│──────────────│
│ Key info     │  ← 1-2 lines of context
│ (e.g., size) │
│              │
│●            ○│  ← Ports (left = input, right = output)
└──────────────┘
```

### Connection Anatomy
```
Block A ●─────→○ Block B
        │
        └─ Arrow indicates direction
```

### Right Sidebar (Parameter Panel)
- **Block Name**: At top, indicates what's selected
- **Divider**: Separates sections
- **Parameters**: Grouped logically
  - Checkboxes for toggles
  - Number inputs for numeric values
  - Dropdowns for enums
- **Action Buttons**: Delete at bottom (destructive action)

**User Actions**:
1. Click block → Parameter panel appears
2. Drag from output port → Hover over input port → Release to connect
3. Click connection line → Press Delete to remove
4. Click canvas background → Deselect block

**Design Rationale**:
- Ports are obvious (circles on edges)
- Arrows show data flow (matches mental model)
- Parameters appear contextually (only when relevant)
- Destructive actions (Delete) are separated from creation

---

## Screen 3: Block Library (Expanded View)

**Context**: User needs to see all blocks or search for specific one

```
┌────────────────────────────────────────────────────────────────────────┐
│  [🔷 DB Builder]    OLTP Database ⌄      [▶ Run]  [Compare]   [Share▾]│
├──────────┬──────────────────────────────────────────────────┬──────────┤
│          │                                                  │          │
│ BLOCKS   │                                                  │ (empty)  │
│ [🔍 lsm_]│  ← Search active                                 │          │
│          │                                                  │          │
│ ┌────────────────┐                                          │          │
│ │ Search Results │                                          │          │
│ ├────────────────┤                                          │          │
│ │ 🟣 LSM Tree    │ ← Matching blocks                        │          │
│ │    Storage     │                                          │          │
│ │    Write-opt...│ ← Truncated description                  │          │
│ │                │                                          │          │
│ │ [No matches in │                                          │          │
│ │  other groups] │                                          │          │
│ └────────────────┘                                          │          │
│                                                             │          │
│ Storage ▾(expanded)                                         │          │
│  🟣 Heap File Storage                                       │          │
│     Simple, unordered                                       │          │
│  🟣 LSM Tree Storage        ← Highlighted from search       │          │
│     High write throughput                                   │          │
│  🟣 Clustered B+ Storage                                    │          │
│     Primary key ordered                                     │          │
│                                                             │          │
│ Indexes ▾(collapsed)                                        │          │
│ Buffers ▾(collapsed)                                        │          │
│                                                             │          │
│ [Show All Blocks]          ← Toggle for advanced blocks    │          │
│ [15 visible / 50 total]                                     │          │
└──────────┴──────────────────────────────────────────────────┴──────────┘
```

**Elements**:

### Search Bar
- **Icon**: Magnifying glass (clear affordance)
- **Placeholder**: "Search blocks..."
- **Live Filter**: Results update as user types
- **Clear Button**: X to reset search

### Block List Items (Expanded)
```
┌──────────────────────┐
│ 🟣 Block Name        │  ← Icon + Name (draggable)
│    One-line desc     │  ← Short description (2-5 words)
│    [?]               │  ← Help icon (tooltip with more info)
└──────────────────────┘
```

### Search Results
- **Grouped**: Shows which category block belongs to
- **Highlighted**: Matching text is bold
- **No Results**: Helpful message + suggestions

### Show All Toggle
- **Default**: 15 core blocks (MVP scope)
- **Expanded**: All blocks (post-MVP features)
- **Counter**: "15 visible / 50 total" shows what's hidden

**User Actions**:
1. Type in search → Results filter in real-time
2. Hover over block → Tooltip shows full description
3. Click [?] icon → Modal with detailed explanation
4. Toggle "Show All" → Advanced blocks appear

**Design Rationale**:
- Search is fast path for experts (no browsing needed)
- Descriptions visible in list (don't need to hover/click)
- Progressive disclosure (basic blocks first, advanced hidden)

---

## Screen 4: Workload Editor

**Context**: User needs to define what queries/operations to run

```
┌────────────────────────────────────────────────────────────────────────┐
│  Define Workload                                             [✕ Close] │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  Workload Name: [OLTP Mixed Workload_________________________]        │
│                                                                        │
│  Operations:                                                           │
│  ┌──────────────────────────────────────────────────────────────────┐ │
│  │ 1. INSERT into users                               [🗑] [⋮]      │ │
│  │    Weight: [50]% ━━━━━━━━━●━━━━━━━━━━ 50%                        │ │
│  │    Template: INSERT INTO users (id, name) VALUES (?, ?)          │ │
│  ├──────────────────────────────────────────────────────────────────┤ │
│  │ 2. SELECT from users by id                         [🗑] [⋮]      │ │
│  │    Weight: [30]% ━━━━━●━━━━━━━━━━━━━━ 30%                        │ │
│  │    Template: SELECT * FROM users WHERE id = ?                    │ │
│  ├──────────────────────────────────────────────────────────────────┤ │
│  │ 3. UPDATE users set name                           [🗑] [⋮]      │ │
│  │    Weight: [20]% ━━●━━━━━━━━━━━━━━━━━ 20%                        │ │
│  │    Template: UPDATE users SET name = ? WHERE id = ?              │ │
│  └──────────────────────────────────────────────────────────────────┘ │
│                                                                        │
│  [+ Add Operation]                                                     │
│                                                                        │
│  ─────────────────────────────────────────────────────────────────────│
│                                                                        │
│  Distribution:                                                         │
│    ◉ Zipfian (80/20 rule - hot keys)     ← Recommended for OLTP      │
│    ○ Uniform (all keys equally likely)                                │
│                                                                        │
│  Concurrency: [100] parallel operations                                │
│               ━━━━━━━━━━━●━━━━━━━━━━ 100                              │
│                                                                        │
│  Total Operations: [10,000_______]                                     │
│                                                                        │
│  ─────────────────────────────────────────────────────────────────────│
│                                                                        │
│  Load Sample Workloads:                                                │
│  [YCSB-A] [YCSB-B] [TPC-C] [Custom]                                   │
│                                                                        │
│                                  [Cancel]  [Save & Run]                │
└────────────────────────────────────────────────────────────────────────┘
```

**Elements**:

### Workload Name
- **Editable**: Click to rename
- **Default**: "Untitled Workload"

### Operation List
- **Reorderable**: Drag [⋮] handle to reorder
- **Deletable**: [🗑] trash icon to remove
- **Expandable**: Click to show/hide details

### Operation Card
```
┌────────────────────────────────────────┐
│ 1. Operation Type               [🗑][⋮]│
│    Weight: [50]% ━━━●━━━━━ 50%         │ ← Slider + input
│    Template: SQL or description        │
└────────────────────────────────────────┘
```

### Add Operation
- **Button**: Clear affordance
- **Modal**: Opens detailed operation editor (MVP: simple dropdowns)

### Distribution
- **Radio Buttons**: Clear choices
- **Descriptions**: Explain what each means
- **Recommendation**: Suggest best choice for context

### Concurrency Slider
- **Visual**: Slider + numeric input (synced)
- **Range**: 1-1000 (configurable)
- **Unit**: "parallel operations"

### Sample Workloads
- **Quick Start**: Pre-defined workloads
- **Buttons**: Click to load (replaces current)
- **Examples**: YCSB (industry standard), TPC-C (transactional)

### Actions
- **Cancel**: Discard changes
- **Save & Run**: Save workload + execute immediately (primary action)

**User Actions**:
1. Click "+ Add Operation" → Select type → Set weight
2. Adjust slider → Weight redistributes automatically
3. Click sample workload → Confirm → Workload loads
4. Click "Save & Run" → Workload executes on design

**Design Rationale**:
- Weights add up to 100% (visual slider makes this obvious)
- Sample workloads reduce friction (no need to define from scratch)
- Clear primary action (Save & Run is what user wants)

---

## Screen 5: Metrics Dashboard (Post-Execution)

**Context**: User has run workload, results are ready

```
┌────────────────────────────────────────────────────────────────────────┐
│  [🔷 DB Builder]    OLTP Database ⌄      [▶ Run]  [Compare]   [Share▾]│
├──────────┬──────────────────────────────────────────────────┬──────────┤
│          │                                                  │          │
│ BLOCKS   │    ┌──────────┐                                 │          │
│          │    │ Schema   │                                 │          │
│ (blocks  │    │ Table:   │●──┐                             │          │
│  shown   │    │ users    │   │                             │          │
│  above)  │    └──────────┘   │                             │          │
│          │                    ↓                             │          │
│          │    ┌──────────┐   ┌──────────┐                  │          │
│          │    │ Heap     │   │ B-tree   │                  │          │
│          │    │ Storage  │●─→│ Index    │                  │          │
│          │    │          │   │ (id)     │                  │          │
│          │    └──────────┘   └──────────┘                  │          │
│          │                                                  │          │
├──────────┴──────────────────────────────────────────────────┴──────────┤
│ ✓ Execution Complete (3.2s)                        [▼ Collapse] [Export]│
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  Performance Summary:                                                  │
│  ┌─────────────┬─────────────┬─────────────┬─────────────────────┐   │
│  │ Throughput  │   Latency   │   Cache     │      I/O            │   │
│  ├─────────────┼─────────────┼─────────────┼─────────────────────┤   │
│  │  1,234 QPS  │   8.1 ms    │    87%      │  542 pages written  │   │
│  │             │   (p99)     │  hit ratio  │  1,234 pages read   │   │
│  └─────────────┴─────────────┴─────────────┴─────────────────────┘   │
│                                                                        │
│  Time Breakdown:                                                       │
│  ┌────────────────────────────────────────────────────────────────┐   │
│  │ Heap Storage    ████████████████████░░ 52% (1.7s)             │   │
│  │ B-tree Index    ████████████░░░░░░░░░░ 38% (1.2s)             │   │
│  │ Other           ███░░░░░░░░░░░░░░░░░░░ 10% (0.3s)             │   │
│  └────────────────────────────────────────────────────────────────┘   │
│                                                                        │
│  ⚠️ Bottleneck Detected:                                               │
│  Heap Storage is the slowest component. Consider:                     │
│    • Adding a buffer pool (LRU or Clock)                              │
│    • Using clustered storage for better locality                      │
│                                                                        │
│  [View Detailed Metrics] [Compare with Another Design]                │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

**Elements**:

### Status Bar
- **Completion Message**: "✓ Execution Complete"
- **Duration**: Total time taken
- **Collapse**: Hide metrics to see canvas
- **Export**: Download metrics as CSV/JSON

### Performance Summary (Cards)
```
┌─────────────────┐
│  Metric Name    │  ← Clear label
├─────────────────┤
│  1,234 QPS      │  ← Large, bold value
│  (context)      │  ← Small context/unit
└─────────────────┘
```

**Key Metrics** (always visible):
- **Throughput**: Operations per second (primary metric)
- **Latency**: p99 latency (ms)
- **Cache Hit Ratio**: Percentage (indicates buffer effectiveness)
- **I/O**: Pages read/written (indicates disk activity)

### Time Breakdown (Bar Chart)
- **Horizontal Bars**: Easy to compare visually
- **Percentages**: Show relative contribution
- **Absolute Values**: Show actual time (seconds)
- **Sorted**: Slowest first (most important)

### Bottleneck Detection
- **Warning Icon**: ⚠️ draws attention
- **Plain Language**: "Heap Storage is the slowest"
- **Actionable Suggestions**: Specific steps to improve
- **Links**: Can add blocks directly from suggestions

### Actions
- **View Detailed Metrics**: Expand to see block-level details
- **Compare**: Launch comparison flow (primary next action)

**User Actions**:
1. Review summary metrics → Identify issues
2. Read bottleneck suggestions → Click to add suggested block
3. Click "Compare" → Create alternative design
4. Click "Export" → Download data for reporting

**Design Rationale**:
- Most important metrics front-and-center (cards)
- Visual breakdown (chart) is faster than table
- Actionable insights (suggestions) move user forward
- Clear next step (Compare) continues workflow

---

## Screen 6: Comparison View (Side-by-Side)

**Context**: User wants to compare two designs

```
┌────────────────────────────────────────────────────────────────────────┐
│  [🔷 DB Builder]  Compare: Design A vs Design B           [✕ Exit]     │
├────────────────────────────────────┬───────────────────────────────────┤
│                                    │                                   │
│  Design A: OLTP with Heap          │  Design B: OLTP with LSM          │
│  [🔷 View] [▶ Run]                 │  [🔷 View] [▶ Run]                │
├────────────────────────────────────┼───────────────────────────────────┤
│                                    │                                   │
│  ┌──────────┐   ┌──────────┐      │  ┌──────────┐   ┌──────────┐     │
│  │ Schema   │──→│  Heap    │──┐   │  │ Schema   │──→│   LSM    │──┐  │
│  └──────────┘   │ Storage  │  │   │  └──────────┘   │  Tree    │  │  │
│                 └──────────┘  │   │                 └──────────┘  │  │
│                      │         │   │                      │         │  │
│                      ↓         │   │                      ↓         │  │
│                 ┌──────────┐  │   │                 ┌──────────┐  │  │
│                 │ B-tree   │←─┘   │                 │ Skip List│←─┘  │
│                 │ Index    │      │                 │ Index    │     │
│                 └──────────┘      │                 └──────────┘     │
│                                    │                                   │
├────────────────────────────────────┼───────────────────────────────────┤
│  Results:                          │  Results:                         │
│                                    │                                   │
│  Throughput:    1,234 QPS          │  Throughput:    1,856 QPS ✓       │
│  Latency (p99):     8.1 ms         │  Latency (p99):    12.3 ms ✗      │
│  Cache Hit:          87%           │  Cache Hit:          82%           │
│  Pages Written:     542            │  Pages Written:      234 ✓        │
│  Pages Read:      1,234            │  Pages Read:       1,456 ✗        │
│                                    │                                   │
├────────────────────────────────────┴───────────────────────────────────┤
│  Summary:                                                              │
│  • Design B (LSM) has 50% higher throughput (better for writes)        │
│  • Design A (Heap) has 35% lower latency (better for reads)            │
│  • Design B uses 57% fewer page writes (better write amplification)    │
│                                                                        │
│  Recommendation: Choose Design B if write throughput is critical.      │
│                                                                        │
│  [Export Report] [Create New Variant] [Choose Design A] [Choose Design B]│
└────────────────────────────────────────────────────────────────────────┘
```

**Elements**:

### Split Layout
- **50/50 Split**: Equal space for each design
- **Synchronized Scrolling**: Canvas pans together (optional)
- **Independent Actions**: Can run each design separately

### Design Headers
- **Name**: Clearly identifies each design
- **Actions**: View (full-screen), Run (re-execute)

### Canvas (Miniaturized)
- **Simplified View**: Enough to see architecture, not every detail
- **Clickable**: Click to expand full-screen

### Results (Aligned)
- **Same Metrics**: Always show same metrics in same order
- **Visual Indicators**: ✓ (better) and ✗ (worse)
- **Color Coding**: Green for winner, red for loser
- **Percentages**: Show relative difference

### Summary (Bottom)
- **Key Differences**: Bullet points highlighting major findings
- **Plain Language**: No jargon, clear explanations
- **Recommendation**: AI-generated or rule-based suggestion
- **Caveats**: "If X is important, choose Y"

### Actions
- **Export Report**: PDF or Markdown summary
- **Create Variant**: Duplicate one design and modify
- **Choose Design**: Mark winner, continue with that design

**User Actions**:
1. Run both designs (if not already run)
2. Review side-by-side results
3. Read summary and recommendation
4. Choose winning design or create new variant

**Design Rationale**:
- Side-by-side is fastest way to compare (no context switching)
- Visual indicators (✓✗) make differences obvious
- Summary does the thinking for user (reduces cognitive load)
- Clear recommendation helps decision-making

---

## Screen 7: Onboarding Tutorial (First-Time User)

**Context**: User opens tool for first time

### Step 1: Welcome Modal

```
┌──────────────────────────────────────────────┐
│                                              │
│          🔷 Welcome to DB Builder            │
│                                              │
│  Design and compare database architectures   │
│       in minutes, not weeks.                 │
│                                              │
│  Let's build your first database together    │
│              (takes 5 minutes)               │
│                                              │
│                                              │
│       [Start Tutorial]    [Skip Tour]        │
│                                              │
└──────────────────────────────────────────────┘
```

### Step 2: Highlight Block Library

```
┌────────────────────────────────────────────────────────────────────────┐
│  [🔷 DB Builder]    Tutorial: Step 1 of 7              [Skip Tutorial] │
├──────────┬──────────────────────────────────────────────────┬──────────┤
│ ┏━━━━━━━━┓                                                │          │
│ ┃ BLOCKS ┃ ← These are building blocks                     │          │
│ ┃        ┃    for your database.                          │          │
│ ┃[Search]┃                                                │          │
│ ┃        ┃    Let's start with storage.                   │          │
│ ┃Storage▾┃                                                │          │
│ ┃ 🟣 Heap┃ ← Drag this block to                            │          │
│ ┃ 🟣 LSM ┃    the canvas →                                 │          │
│ ┃ 🟣 B+  ┃                                                │          │
│ ┃        ┃                              [Next]             │          │
│ ┗━━━━━━━━┛                                                │          │
│                                                             │          │
└──────────┴──────────────────────────────────────────────────┴──────────┘
```

**Tutorial Overlay**:
- **Spotlight**: Highlight active area (rest dimmed)
- **Arrow/Pointer**: Draw attention to specific element
- **Instructions**: Clear, one action per step
- **Progress**: "Step X of 7"
- **Skip Option**: Always visible (respect user's time)

### Step 3: Drag Block to Canvas

```
┌────────────────────────────────────────────────────────────────────────┐
│  [🔷 DB Builder]    Tutorial: Step 2 of 7              [Skip Tutorial] │
├──────────┬──────────────────────────────────────────────────┬──────────┤
│          │                                                  │          │
│ BLOCKS   │   ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓   │          │
│          │   ┃                                        ┃   │          │
│ Storage ▾│   ┃  Great! You placed your first block.  ┃   │          │
│  🟣 Heap │   ┃                                        ┃   │          │
│  🟣 LSM  │   ┃     ┌──────────┐                       ┃   │          │
│  🟣 B+   │   ┃     │  Heap    │                       ┃   │          │
│          │   ┃     │ Storage  │                       ┃   │          │
│          │   ┃     └──────────┘                       ┃   │          │
│          │   ┃                                        ┃   │          │
│          │   ┃  Now let's add an index.        [Next]┃   │          │
│          │   ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛   │          │
│          │                                                  │          │
└──────────┴──────────────────────────────────────────────────┴──────────┘
```

### Step 4: Connect Blocks

```
┌────────────────────────────────────────────────────────────────────────┐
│  [🔷 DB Builder]    Tutorial: Step 4 of 7              [Skip Tutorial] │
├──────────┬──────────────────────────────────────────────────┬──────────┤
│          │                                                  │          │
│ BLOCKS   │   ┌──────────┐        ┌──────────┐              │          │
│          │   │  Heap    │●       │ B-tree   │              │          │
│          │   │ Storage  │        │ Index    │              │          │
│          │   └──────────┘        └──────────┘              │          │
│          │          ↑                                       │          │
│          │          └─────────────────────────────────┐     │          │
│          │   ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━|━━┓  │          │
│          │   ┃ Click the output port (●) then       |  ┃  │          │
│          │   ┃ click the input port (○) to connect  |  ┃  │          │
│          │   ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛  │          │
└──────────┴──────────────────────────────────────────────────┴──────────┘
```

### Step 5: Run Simulation

```
┌────────────────────────────────────────────────────────────────────────┐
│  [🔷 DB Builder]  Tutorial: Step 6 of 7    ┏━━━━━━━━━━━━━━━━━━━━━━┓  │
│                                            ┃ Click Run to see    ┃  │
│                                            ┃ how your design     ┃  │
│                        [▶ Run] ◄───────────┃ performs!           ┃  │
│                                            ┗━━━━━━━━━━━━━━━━━━━━━━┛  │
├──────────┬──────────────────────────────────────────────────┬──────────┤
│          │                                                  │          │
│ (design  │   ┌──────────┐                                  │          │
│  shown)  │   │  Heap    │●─────→○ ┌──────────┐             │          │
│          │   │ Storage  │         │ B-tree   │             │          │
│          │   └──────────┘         │ Index    │             │          │
│          │                        └──────────┘             │          │
└──────────┴──────────────────────────────────────────────────┴──────────┘
```

### Step 6: View Results

```
┌────────────────────────────────────────────────────────────────────────┐
│  [🔷 DB Builder]    Tutorial: Step 7 of 7              [Skip Tutorial] │
├──────────┴──────────────────────────────────────────────────┴──────────┤
│ ✓ Execution Complete (2.1s)                                           │
├────────────────────────────────────────────────────────────────────────┤
│  ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓  │
│  ┃ 🎉 Congratulations!                                              ┃  │
│  ┃                                                                  ┃  │
│  ┃ You just designed your first database!                          ┃  │
│  ┃                                                                  ┃  │
│  ┃ Your design achieved:                                            ┃  │
│  ┃  • 1,234 operations per second                                   ┃  │
│  ┃  • 8ms latency                                                   ┃  │
│  ┃                                                                  ┃  │
│  ┃ Next steps:                                                      ┃  │
│  ┃  • Try swapping Heap for LSM Tree and run again                 ┃  │
│  ┃  • Compare two designs side-by-side                             ┃  │
│  ┃  • Explore templates for common use cases                       ┃  │
│  ┃                                                                  ┃  │
│  ┃            [Start from Template]  [Explore on My Own]           ┃  │
│  ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛  │
└────────────────────────────────────────────────────────────────────────┘
```

**Tutorial Design Principles**:
- **One Action Per Step**: Don't overwhelm
- **Visual Spotlight**: Dim everything except current focus
- **Progress Indicator**: Show how many steps remain
- **Always Skippable**: Respect expert users
- **Celebration**: Positive reinforcement at end
- **Clear Next Steps**: Continue momentum

---

## Screen 8: Error States

### Error 1: Invalid Connection

```
┌────────────────────────────────────────────────────────────────────────┐
│  [🔷 DB Builder]    OLTP Database ⌄      [▶ Run]  [Compare]   [Share▾]│
├──────────┬──────────────────────────────────────────────────┬──────────┤
│          │                                                  │          │
│ BLOCKS   │   ┌──────────┐        ┌──────────┐              │          │
│          │   │ B-tree   │●·······X│Sequential│ ← Red X     │          │
│          │   │ Index    │        │ Scan     │              │          │
│          │   └──────────┘        └──────────┘              │          │
│          │            └──────────────┘                      │          │
│          │   ┌───────────────────────────────┐             │          │
│          │   │ ❌ Cannot connect             │             │          │
│          │   │ INDEX_LOOKUP cannot connect   │             │          │
│          │   │ to RECORD_STREAM              │             │          │
│          │   │                               │             │          │
│          │   │ Tip: Use Heap Storage before  │             │          │
│          │   │ Sequential Scan.              │             │          │
│          │   └───────────────────────────────┘             │          │
└──────────┴──────────────────────────────────────────────────┴──────────┘
```

**Elements**:
- **Red X**: On target port (clear rejection)
- **Dotted Line**: Connection attempt (visual feedback)
- **Tooltip**: Explains why it failed
- **Suggestion**: Hints at correct approach

### Error 2: Missing Connections (Pre-Run Validation)

```
┌────────────────────────────────────────────────────────────────────────┐
│                    Design Validation Errors                            │
│                                                              [✕ Close]  │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  ⚠️ Your design has 2 issues that must be fixed before running:        │
│                                                                        │
│  1. Sequential Scan block has no input                                │
│     → Connect a Storage block's output to its input                   │
│     [Show on Canvas]                                                  │
│                                                                        │
│  2. B-tree Index block is not connected to anything                   │
│     → Either connect it or remove it from the design                  │
│     [Show on Canvas] [Delete Block]                                   │
│                                                                        │
│                                                                        │
│                                            [Cancel]  [Fix Issues]      │
└────────────────────────────────────────────────────────────────────────┘
```

**Elements**:
- **Modal**: Blocks execution until fixed
- **Numbered List**: Clear enumeration of issues
- **Plain Language**: Describes problem and solution
- **Action Buttons**: Quick fixes (show, delete)
- **Primary Action**: "Fix Issues" (dismisses modal, highlights problems)

### Error 3: Execution Failure

```
┌────────────────────────────────────────────────────────────────────────┐
│                    Execution Failed                                    │
│                                                              [✕ Close]  │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  ❌ The simulation stopped unexpectedly                                │
│                                                                        │
│  Error at operation 1,234 of 10,000:                                  │
│  "Page allocation failed: Out of memory"                              │
│                                                                        │
│  This might be caused by:                                             │
│  • Buffer pool size is too small (increase to 512MB+)                 │
│  • Workload is too large for simulation (reduce to 1,000 ops)        │
│                                                                        │
│  ─────────────────────────────────────────────────────────────────────│
│                                                                        │
│  Technical Details: (for debugging)                                   │
│  Block: Heap Storage (id: block_123)                                  │
│  Stack Trace: [Show]                                                  │
│                                                                        │
│                                                                        │
│                    [Report Bug]  [Adjust Settings]  [Close]           │
└────────────────────────────────────────────────────────────────────────┘
```

**Elements**:
- **Error Icon**: ❌ (signals failure)
- **Context**: Where it failed (operation #, block)
- **Possible Causes**: Help user diagnose
- **Technical Details**: Collapsible (for power users)
- **Actions**: Report (help us fix), Adjust (let user fix), Close

### Error 4: Network/Loading Error

```
┌────────────────────────────────────────────────────────────────────────┐
│  [🔷 DB Builder]    OLTP Database ⌄      [▶ Run]  [Compare]   [Share▾]│
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│                        ⚠️ Connection Lost                              │
│                                                                        │
│                Your design is saved locally.                           │
│                Refresh the page to continue.                           │
│                                                                        │
│                        [Refresh Page]                                  │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

**Elements**:
- **Full Screen**: Critical error, block all interaction
- **Reassurance**: "Your work is saved" (reduce anxiety)
- **Clear Action**: One button, obvious what to do

---

## Screen 9: Mobile View (View-Only)

**Context**: User opens on mobile (out of scope for editing, but should be viewable)

```
┌──────────────────────────┐
│ 🔷 DB Builder            │
│ OLTP Database         ⋮  │
├──────────────────────────┤
│                          │
│  📱 Mobile View Only     │
│                          │
│  ┌────────┐              │
│  │Schema  │              │
│  └───┬────┘              │
│      ↓                   │
│  ┌────────┐  ┌────────┐  │
│  │ Heap   │→ │B-tree  │  │
│  │Storage │  │Index   │  │
│  └────────┘  └────────┘  │
│                          │
│  [View Full Design]      │
│  (opens desktop mode)    │
│                          │
├──────────────────────────┤
│ Results:                 │
│  1,234 QPS               │
│  8.1ms latency           │
│                          │
│ [Request Desktop Site]   │
└──────────────────────────┘
```

**Elements**:
- **Simplified View**: Vertical stack (not canvas)
- **Read-Only**: No editing (too complex for mobile)
- **Call-to-Action**: "Request Desktop Site" or "View Full"
- **Key Metrics**: Show results if already executed

---

## Component Library (Reusable Elements)

### Button Styles

```
Primary:   [▶ Run]         (solid, bold, stands out)
Secondary: [Compare]       (outline, less prominent)
Danger:    [Delete]        (red, destructive action)
Ghost:     [Cancel]        (text only, minimal)
```

### Form Inputs

```
Text:      [Value_________]
Number:    [128___] or [▲128▼]
Slider:    ━━━━━●━━━━━━━━━
Checkbox:  [✓] Option
Radio:     ◉ Selected  ○ Unselected
Dropdown:  [Value ▾]
```

### Icons (Consistent System)

```
Storage:   🟣 (purple)
Index:     🔵 (blue)
Buffer:    🟢 (green)
Execute:   ▶
Settings:  ⚙
Help:      ?
Close:     ✕
Drag:      ⋮
Delete:    🗑
Success:   ✓
Error:     ✗
Warning:   ⚠️
```

### States

```
Default:   Normal appearance
Hover:     Subtle highlight
Active:    Pressed/selected (darker)
Disabled:  Grayed out, cursor: not-allowed
Loading:   Spinner or progress indicator
Error:     Red border/text
```

---

## Design Tokens (For Implementation)

### Spacing Scale
```
xs:  4px   (tight)
sm:  8px   (default)
md:  16px  (sections)
lg:  24px  (major sections)
xl:  32px  (page margins)
```

### Typography
```
Display: 24px Bold
Heading: 18px Semibold
Body:    14px Regular
Small:   12px Regular
Code:    14px Mono
```

### Colors
```
Primary:   #3B82F6  (blue)
Success:   #10B981  (green)
Error:     #EF4444  (red)
Warning:   #F59E0B  (yellow)
Neutral:   #6B7280  (gray)

Storage:   #8B5CF6  (purple)
Index:     #3B82F6  (blue)
Buffer:    #14B8A6  (teal)
Concurrency: #F97316  (orange)
Query:     #6366F1  (indigo)
```

### Borders
```
Default:   1px solid #E5E7EB
Hover:     1px solid #3B82F6
Active:    2px solid #3B82F6
Error:     2px solid #EF4444
```

### Shadows
```
sm:  0 1px 2px rgba(0,0,0,0.05)
md:  0 4px 6px rgba(0,0,0,0.1)
lg:  0 10px 15px rgba(0,0,0,0.1)
```

---

## Interaction Patterns Summary

### Drag and Drop
1. Hover over block → Cursor: grab
2. Click and drag → Ghost appears
3. Hover over canvas → Drop zone highlights
4. Release → Block snaps into place

### Connecting Blocks
1. Click output port → Port highlights
2. Drag to input port → Line follows cursor
3. Hover over valid input → Green glow
4. Hover over invalid input → Red X
5. Release → Connection created (or error shown)

### Parameter Editing
1. Click block → Parameter panel slides in
2. Edit values → Changes apply immediately
3. Click canvas → Panel stays (until another block selected)
4. Press Escape → Deselect block

### Running Simulation
1. Click Run → Validation runs
2. If errors → Show error modal
3. If valid → Progress bar appears
4. On complete → Metrics slide up from bottom
5. Metrics visible → Can collapse/expand

---

## Accessibility Annotations

### Keyboard Navigation
- **Tab**: Move between blocks, inputs, buttons
- **Arrow Keys**: Pan canvas
- **Enter**: Activate button/connect ports
- **Delete**: Remove selected block/connection
- **Escape**: Deselect, close modal
- **Cmd/Ctrl + Z**: Undo
- **Cmd/Ctrl + C/V**: Copy/paste

### Screen Reader
- All blocks have aria-labels: "Heap File Storage block, not connected"
- Connections described: "Connection from Heap Storage to B-tree Index"
- Metrics announced: "Throughput: 1,234 operations per second"

### Color Contrast
- All text meets WCAG AA (4.5:1 minimum)
- Don't rely on color alone (use icons + labels)
- Error states use both color and icon (❌)

---

## Next Steps

1. **Week 1**: Convert wireframes to Figma (high-fidelity mockups)
2. **Week 1**: Build component library (buttons, inputs, cards)
3. **Week 2**: Implement canvas with React Flow
4. **Week 3**: User test wireframes with 3 database engineers

**Questions to validate**:
- Is the layout intuitive?
- Can users find blocks easily?
- Are error messages helpful?
- Does the comparison view make differences clear?

---

**End of Wireframes Document**
