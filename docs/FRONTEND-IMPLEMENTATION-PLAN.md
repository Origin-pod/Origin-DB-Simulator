# Frontend Implementation Plan: DB Simulator

**Created**: 2026-01-26
**Goal**: Build the visual canvas and mock frontend for the DB Simulator, then integrate with the Rust block system backend.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Frontend (React/TS)                         │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────────────┐  ┌──────────────────┐   │
│  │ Block       │  │     Canvas          │  │ Parameter        │   │
│  │ Palette     │  │   (React Flow)      │  │ Panel            │   │
│  │             │  │                     │  │                  │   │
│  │ - Search    │  │ ┌─────┐   ┌─────┐  │  │ - Block config   │   │
│  │ - Categories│  │ │Block│──→│Block│  │  │ - Validation     │   │
│  │ - Drag/Drop │  │ └─────┘   └─────┘  │  │ - Help tooltips  │   │
│  └─────────────┘  └─────────────────────┘  └──────────────────┘   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    Metrics Dashboard                          │   │
│  │  Throughput | Latency | I/O | Block Breakdown                │   │
│  └─────────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────────┤
│                        State Management (Zustand)                   │
│  - Canvas state (blocks, connections, selection)                    │
│  - Workload state (operations, distribution)                        │
│  - Execution state (running, metrics, errors)                       │
├─────────────────────────────────────────────────────────────────────┤
│                         WASM Bridge (Later)                         │
│  - Compile Rust block-system to WASM                               │
│  - Execute blocks in browser                                        │
│  - Collect metrics from Rust                                        │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Tech Stack

| Layer | Technology | Purpose |
|-------|------------|---------|
| Framework | React 18 + TypeScript 5 | UI Components |
| Build Tool | Vite | Fast HMR, ESBuild |
| Canvas | React Flow | Node-based visual editor |
| Styling | Tailwind CSS | Utility-first styling |
| State | Zustand | Simple state management |
| Charts | Recharts | Metrics visualization |
| Icons | Lucide React | Consistent icon system |
| Storage | IndexedDB (Dexie) | Persist designs locally |

---

## Milestone Breakdown

### Phase 1: Foundation (Canvas + Static Blocks)
**Duration**: 3-4 sessions
**Goal**: Working canvas with draggable blocks and connections

### Phase 2: Block System Integration
**Duration**: 2-3 sessions
**Goal**: TypeScript block definitions matching Rust, parameter configuration

### Phase 3: Mock Execution Engine
**Duration**: 2-3 sessions
**Goal**: Simulated execution with mock metrics (before WASM integration)

### Phase 4: Metrics & Visualization
**Duration**: 2 sessions
**Goal**: Dashboard showing execution results

### Phase 5: Comparison & Templates
**Duration**: 2-3 sessions
**Goal**: Multi-design support, side-by-side comparison

### Phase 6: Polish & UX
**Duration**: 2 sessions
**Goal**: Onboarding, error handling, accessibility

### Phase 7: WASM Integration
**Duration**: 3-4 sessions
**Goal**: Connect to Rust block-system via WASM

---

# Milestone 1: Project Setup & Basic Canvas

## Goal
Set up the React project with React Flow, implement basic canvas with pan/zoom, and create the layout shell.

## Deliverables
- [ ] Vite + React + TypeScript project scaffold
- [ ] Tailwind CSS configured
- [ ] React Flow integration
- [ ] Basic 3-panel layout (palette | canvas | properties)
- [ ] Empty canvas with pan/zoom/grid
- [ ] Top bar with logo and placeholder buttons

## File Structure
```
/frontend
├── src/
│   ├── main.tsx
│   ├── App.tsx
│   ├── components/
│   │   ├── layout/
│   │   │   ├── TopBar.tsx
│   │   │   ├── BlockPalette.tsx
│   │   │   ├── Canvas.tsx
│   │   │   └── ParameterPanel.tsx
│   │   └── ui/
│   │       ├── Button.tsx
│   │       └── Input.tsx
│   ├── hooks/
│   ├── stores/
│   │   └── canvasStore.ts
│   ├── types/
│   │   └── index.ts
│   └── styles/
│       └── globals.css
├── package.json
├── tsconfig.json
├── vite.config.ts
└── tailwind.config.js
```

## Prompt for Milestone 1

```
I'm building a database simulator with a visual canvas editor. Set up the frontend project with:

**Tech Stack:**
- Vite + React 18 + TypeScript 5 (strict mode)
- React Flow for the node-based canvas editor
- Tailwind CSS for styling
- Zustand for state management
- Lucide React for icons

**Layout Requirements:**
Create a 3-panel layout:
1. **Left sidebar (240px)**: Block palette - will contain draggable blocks
2. **Center**: React Flow canvas with grid background, pan/zoom
3. **Right sidebar (280px)**: Parameter panel - shows when a block is selected

**Top Bar:**
- Logo/brand on left
- Design name (editable) in center
- Action buttons on right: [Run] [Compare] [Share]

**Canvas Features:**
- Grid background (subtle gray dots)
- Minimap in bottom-right
- Zoom controls
- Fit-to-view button

**State Management (Zustand):**
Create a canvas store with:
- nodes: ReactFlow nodes array
- edges: ReactFlow edges array
- selectedNode: currently selected node ID
- Actions: addNode, removeNode, updateNode, addEdge, removeEdge, setSelectedNode

**Styling:**
- Dark mode ready (but start with light mode)
- Use Inter font for UI, JetBrains Mono for code/technical text
- Design tokens for colors matching this palette:
  - Primary: #3B82F6 (blue)
  - Storage blocks: #8B5CF6 (purple)
  - Index blocks: #3B82F6 (blue)
  - Buffer blocks: #14B8A6 (teal)
  - Success: #10B981
  - Error: #EF4444

Create all files needed for a working project scaffold. The canvas should be interactive (pan, zoom, select) even if empty.
```

---

# Milestone 2: Block Definitions & Palette

## Goal
Define the block type system and create a functional block palette with categories.

## Deliverables
- [ ] TypeScript interfaces for Block, Port, Connection
- [ ] Block category definitions (Storage, Index, Buffer, Concurrency, Execution)
- [ ] Block palette component with collapsible categories
- [ ] Search/filter functionality
- [ ] Drag from palette to canvas (creates node)
- [ ] 5 initial block definitions (placeholder implementations)

## Key Types
```typescript
// src/types/block.ts
interface BlockDefinition {
  type: string;
  name: string;
  description: string;
  category: BlockCategory;
  icon: string;
  color: string;
  inputs: PortDefinition[];
  outputs: PortDefinition[];
  parameters: ParameterDefinition[];
  documentation?: BlockDocumentation;
}

interface PortDefinition {
  name: string;
  type: PortType;
  dataType: DataType;
  description: string;
  required: boolean;
}

interface ParameterDefinition {
  name: string;
  type: 'string' | 'number' | 'boolean' | 'enum';
  default: any;
  description: string;
  constraints?: ParameterConstraints;
  uiHint?: UIHint;
}
```

## Prompt for Milestone 2

```
Continue building the DB Simulator frontend. Now implement the block type system and palette.

**Block Type System:**
Create TypeScript interfaces that mirror the Rust block system:

1. `BlockDefinition` - static definition of a block type:
   - type: unique identifier (e.g., "heap_storage")
   - name: display name ("Heap File Storage")
   - description: short description
   - category: BlockCategory enum
   - icon: Lucide icon name
   - color: hex color for the category
   - inputs: array of PortDefinition
   - outputs: array of PortDefinition
   - parameters: array of ParameterDefinition
   - documentation: optional detailed docs

2. `BlockCategory` enum:
   - Storage, Index, Buffer, Concurrency, Execution, Transaction, Compression, Partitioning, Optimization, Distribution

3. `PortDefinition`:
   - name, type (Input/Output), dataType (DataStream/SingleValue/Batch/Signal)
   - description, required flag

4. `ParameterDefinition`:
   - name, type (string/number/boolean/enum)
   - default value, description
   - constraints (min/max for numbers, pattern for strings)
   - uiHint (slider/input/select/checkbox)

5. `BlockInstance` - a placed block on canvas:
   - id: unique instance ID
   - type: references BlockDefinition.type
   - position: {x, y}
   - parameters: current parameter values
   - state: 'idle' | 'running' | 'error' | 'complete'

**Block Registry:**
Create a registry with 5 initial blocks:

1. **Schema Definition** (Storage category)
   - Outputs: schema
   - Parameters: tableName, columns (array)

2. **Heap File Storage** (Storage category)
   - Inputs: records (DataStream)
   - Outputs: storedRecords (DataStream)
   - Parameters: pageSize (default: 8192)

3. **B-tree Index** (Index category)
   - Inputs: records (DataStream)
   - Outputs: indexLookup (SingleValue)
   - Parameters: keyColumn, fanout (default: 128), unique (boolean)

4. **Sequential Scan** (Execution category)
   - Inputs: storage (DataStream)
   - Outputs: records (DataStream)
   - Parameters: none

5. **LRU Buffer Pool** (Buffer category)
   - Inputs: pageRequests (DataStream)
   - Outputs: pages (DataStream)
   - Parameters: size (MB), pageSize

**Block Palette Component:**
- Collapsible category sections
- Block items show icon + name + short description
- Draggable (use @dnd-kit/core or React Flow's built-in drag)
- Search bar at top filters blocks
- "Show all blocks" toggle (for MVP vs advanced)

**Drag to Canvas:**
- Dragging a block from palette to canvas creates a BlockInstance
- Node appears at drop position
- Node shows block name, icon, input/output ports
```

---

# Milestone 3: Block Nodes & Connections

## Goal
Create custom React Flow nodes for blocks, implement port connections with validation.

## Deliverables
- [ ] Custom BlockNode component for React Flow
- [ ] Port handles (input left, output right)
- [ ] Connection validation (type checking)
- [ ] Visual feedback for valid/invalid connections
- [ ] Edge styling with arrows
- [ ] Connection removal

## Prompt for Milestone 3

```
Continue the DB Simulator frontend. Implement custom block nodes and port connections.

**Custom Block Node:**
Create a React Flow custom node component `BlockNode`:

Visual design:
```
┌──────────────────────────────┐
│ [Icon] Block Name      [⚙️]  │  ← Header with category color
├──────────────────────────────┤
│                              │
│ ○ input1          output1 ● │  ← Ports with handles
│ ○ input2          output2 ● │
│                              │
│ Key param: value             │  ← Optional: show key parameter
└──────────────────────────────┘
```

- Header bar colored by category (Storage=purple, Index=blue, etc.)
- Icon from Lucide matching block type
- Settings gear icon opens parameter panel (or click anywhere)
- Input ports on left (small circles), output ports on right
- Port tooltips show name and data type
- Selected state: blue border + shadow
- Running state: animated border
- Error state: red border + error icon

**Port Handles:**
- Use React Flow's Handle component
- Position: left for inputs, right for outputs
- Color-coded by data type:
  - DataStream: blue
  - SingleValue: green
  - Batch: purple
  - Signal: orange

**Connection Validation:**
Create a `canConnect(sourcePort, targetPort)` function:
- Same data type required (DataStream → DataStream)
- Output can only connect to Input
- One connection per input port (unless multi-input supported)
- Return error message if invalid

**Edge Styling:**
- Bezier curves (smooth)
- Animated dashes when data is flowing
- Arrow at target end
- Clickable to select (then Delete key removes)
- Hover shows tooltip with connection info

**Visual Feedback:**
- Dragging from port: temporary edge follows cursor
- Valid target: port glows green
- Invalid target: port shows red X, tooltip explains why
- Connection created: brief flash animation

**Store Updates:**
- When connection created: add to edges, validate design
- When connection removed: update edges, re-validate
- When block removed: remove all connected edges
```

---

# Milestone 4: Parameter Panel & Configuration

## Goal
Implement the right sidebar parameter panel for configuring selected blocks.

## Deliverables
- [ ] Parameter panel component
- [ ] Dynamic form based on block's parameter definitions
- [ ] Input types: number, text, boolean, enum/select
- [ ] Validation with error messages
- [ ] Real-time updates to block state
- [ ] Help tooltips for each parameter

## Prompt for Milestone 4

```
Continue the DB Simulator frontend. Implement the parameter configuration panel.

**Parameter Panel (Right Sidebar):**
Shows when a block is selected on canvas.

Layout:
```
┌─────────────────────────────┐
│ B-tree Index          [🗑️]  │  ← Block name + delete button
│ Creates a B-tree index...   │  ← Short description
├─────────────────────────────┤
│ Configuration               │
│                             │
│ Key Column                  │
│ [id________________] ▾      │  ← Select dropdown
│                             │
│ Fanout                      │
│ [128_____] ─────●──────     │  ← Number + slider
│ Higher = fewer levels       │  ← Help text
│                             │
│ Unique                      │
│ [✓] Enforce uniqueness      │  ← Checkbox
│                             │
├─────────────────────────────┤
│ Ports                       │
│                             │
│ ● Input: records            │
│   DataStream (connected)    │
│                             │
│ ○ Output: indexLookup       │
│   SingleValue (not connected)│
└─────────────────────────────┘
```

**Form Components:**

1. `NumberInput`:
   - Input field + optional slider
   - min/max validation from constraints
   - Step size from uiHint
   - Shows unit if specified (MB, ms, etc.)

2. `TextInput`:
   - Standard text input
   - Regex validation if pattern constraint
   - Max length if specified

3. `BooleanInput`:
   - Checkbox with label

4. `EnumInput`:
   - Select dropdown
   - Options from parameter's allowedValues

5. `ArrayInput` (for columns, etc.):
   - List with add/remove buttons
   - Reorderable

**Validation:**
- Validate on change
- Show inline error message below input
- Prevent invalid values (e.g., number below min)
- Show warning for unusual values (not blocking)

**Store Integration:**
- On parameter change: update BlockInstance.parameters
- Debounce updates (don't spam on every keystroke)
- Persist to IndexedDB

**Help System:**
- [?] icon next to each parameter
- Click shows tooltip or popover with:
  - Full description
  - Default value
  - Impact on performance (if known)
```

---

# Milestone 5: Workload Editor

## Goal
Create the workload definition modal/panel for defining test operations.

## Deliverables
- [ ] Workload modal component
- [ ] Operation list (add/remove operations)
- [ ] Operation types: INSERT, SELECT, UPDATE, DELETE
- [ ] Weight sliders (percentages sum to 100%)
- [ ] Distribution selector (uniform, zipfian)
- [ ] Concurrency slider
- [ ] Preset workloads (YCSB-A, YCSB-B, etc.)

## Prompt for Milestone 5

```
Continue the DB Simulator frontend. Implement the workload definition editor.

**Workload Editor Modal:**
Opens when user clicks "Define Workload" or before running.

Layout:
```
┌───────────────────────────────────────────────────────────┐
│ Define Workload                                    [✕]    │
├───────────────────────────────────────────────────────────┤
│ Name: [OLTP Mixed Workload________________]               │
│                                                           │
│ Operations:                                               │
│ ┌─────────────────────────────────────────────────────┐   │
│ │ 1. INSERT into table                    [🗑️] [≡]   │   │
│ │    Weight: [50]% ━━━━━━━━━━●━━━━━━━━━━              │   │
│ │    Template: INSERT INTO {table} VALUES (?)         │   │
│ ├─────────────────────────────────────────────────────┤   │
│ │ 2. SELECT by primary key                [🗑️] [≡]   │   │
│ │    Weight: [30]% ━━━━━━●━━━━━━━━━━━━━━              │   │
│ │    Template: SELECT * FROM {table} WHERE id = ?     │   │
│ ├─────────────────────────────────────────────────────┤   │
│ │ 3. UPDATE by primary key                [🗑️] [≡]   │   │
│ │    Weight: [20]% ━━━━●━━━━━━━━━━━━━━━━              │   │
│ │    Template: UPDATE {table} SET col = ? WHERE id = ?│   │
│ └─────────────────────────────────────────────────────┘   │
│                                                           │
│ [+ Add Operation]                                         │
│                                                           │
│ ─────────────────────────────────────────────────────────│
│                                                           │
│ Distribution:                                             │
│   ◉ Zipfian (hot keys - realistic OLTP)                  │
│   ○ Uniform (all keys equally likely)                    │
│   ○ Latest (recent records more likely)                  │
│                                                           │
│ Concurrency: [100______] operations in parallel          │
│              ━━━━━━━━━━━━●━━━━━━━━ 100                   │
│                                                           │
│ Total Operations: [10,000____]                           │
│                                                           │
│ ─────────────────────────────────────────────────────────│
│                                                           │
│ Presets: [YCSB-A] [YCSB-B] [YCSB-C] [TPC-C] [Custom]    │
│                                                           │
│                              [Cancel]  [Save & Close]    │
└───────────────────────────────────────────────────────────┘
```

**Workload State (Zustand):**
```typescript
interface Workload {
  id: string;
  name: string;
  operations: Operation[];
  distribution: 'uniform' | 'zipfian' | 'latest';
  concurrency: number;
  totalOperations: number;
}

interface Operation {
  id: string;
  type: 'INSERT' | 'SELECT' | 'UPDATE' | 'DELETE' | 'SCAN';
  weight: number; // percentage 0-100
  template: string;
}
```

**Weight Sliders:**
- All weights must sum to 100%
- Auto-redistribute when one changes
- Visual bar shows proportion

**Preset Workloads:**
- YCSB-A: 50% read, 50% update
- YCSB-B: 95% read, 5% update
- YCSB-C: 100% read
- TPC-C: Insert-heavy transactional

**Validation:**
- At least one operation required
- Weights must sum to 100%
- Total operations > 0
- Concurrency > 0
```

---

# Milestone 6: Mock Execution Engine

## Goal
Create a TypeScript mock execution engine that simulates running workloads and generates realistic metrics.

## Deliverables
- [ ] Execution engine class
- [ ] Graph validation (all required inputs connected)
- [ ] Topological sort for execution order
- [ ] Mock execution with progress
- [ ] Simulated metrics generation
- [ ] Execution state management (idle, running, complete, error)

## Prompt for Milestone 6

```
Continue the DB Simulator frontend. Implement a mock execution engine (before WASM integration).

**Execution Engine:**
Create a class that validates and "executes" the design.

```typescript
class MockExecutionEngine {
  // Validate the design can be executed
  validate(design: Design): ValidationResult;

  // Execute the workload on the design
  async execute(design: Design, workload: Workload): Promise<ExecutionResult>;

  // Cancel running execution
  cancel(): void;
}

interface ValidationResult {
  valid: boolean;
  errors: ValidationError[];
  warnings: ValidationWarning[];
}

interface ExecutionResult {
  success: boolean;
  duration: number;
  metrics: ExecutionMetrics;
  blockMetrics: Map<string, BlockMetrics>;
  errors?: ExecutionError[];
}

interface ExecutionMetrics {
  throughput: number;      // ops/sec
  latency: {
    avg: number;
    p50: number;
    p95: number;
    p99: number;
  };
  totalOperations: number;
  successfulOperations: number;
  failedOperations: number;
}

interface BlockMetrics {
  blockId: string;
  blockType: string;
  executionTime: number;
  percentage: number;
  counters: Record<string, number>;  // pages_read, cache_hits, etc.
}
```

**Validation Rules:**
1. All required input ports must be connected
2. No cycles in the graph (DAG only)
3. At least one storage block
4. At least one output/query block
5. Parameter constraints satisfied

**Mock Execution Flow:**
1. Validate design → show errors if invalid
2. Build execution graph (topological sort)
3. For each block in order:
   - Simulate execution time (based on block type + parameters)
   - Generate mock metrics (realistic ranges)
   - Report progress (percentage complete)
4. Aggregate metrics
5. Return results

**Mock Metrics Generation:**
Generate realistic-looking metrics based on block configuration:

- Heap Storage:
  - pages_written = totalOps * 0.1 (10% fill rate)
  - write_time = pages_written * 0.5ms

- B-tree Index:
  - lookups = totalOps * readRatio
  - lookup_time = lookups * log2(dataSize) * 0.1ms

- LRU Buffer:
  - cache_hits = totalOps * hitRate (based on size)
  - cache_misses = totalOps * (1 - hitRate)

**Progress Reporting:**
- Use a callback or observable pattern
- Report: { phase: string, progress: number, currentBlock: string }
- Update UI every 100ms or 1% progress

**Execution State (Zustand):**
```typescript
interface ExecutionState {
  status: 'idle' | 'validating' | 'running' | 'complete' | 'error';
  progress: number;  // 0-100
  currentBlock: string | null;
  result: ExecutionResult | null;
  error: string | null;

  // Actions
  startExecution: (design: Design, workload: Workload) => Promise<void>;
  cancelExecution: () => void;
  clearResults: () => void;
}
```
```

---

# Milestone 7: Metrics Dashboard

## Goal
Create the metrics visualization panel that shows execution results.

## Deliverables
- [ ] Metrics dashboard component
- [ ] Summary cards (throughput, latency, I/O)
- [ ] Block breakdown chart (horizontal bar)
- [ ] Time series chart (optional, for longer runs)
- [ ] Bottleneck detection and suggestions
- [ ] Export metrics (JSON/CSV)

## Prompt for Milestone 7

```
Continue the DB Simulator frontend. Implement the metrics dashboard.

**Metrics Dashboard:**
Appears at bottom of screen after execution completes. Expandable/collapsible.

Layout:
```
┌────────────────────────────────────────────────────────────────────────┐
│ ✓ Execution Complete (3.2s)                       [▼ Collapse] [Export]│
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  Performance Summary:                                                  │
│  ┌─────────────┬─────────────┬─────────────┬─────────────────────┐    │
│  │ Throughput  │   Latency   │   Cache     │      I/O            │    │
│  ├─────────────┼─────────────┼─────────────┼─────────────────────┤    │
│  │  1,234      │   8.1 ms    │    87%      │  542 writes         │    │
│  │  ops/sec    │   (p99)     │  hit ratio  │  1,234 reads        │    │
│  └─────────────┴─────────────┴─────────────┴─────────────────────┘    │
│                                                                        │
│  Block Breakdown:                                                      │
│  ┌────────────────────────────────────────────────────────────────┐   │
│  │ Heap Storage    ████████████████████░░░░░░░░░ 52% (1.7s)       │   │
│  │ B-tree Index    ████████████░░░░░░░░░░░░░░░░░ 38% (1.2s)       │   │
│  │ LRU Buffer      ███░░░░░░░░░░░░░░░░░░░░░░░░░░ 10% (0.3s)       │   │
│  └────────────────────────────────────────────────────────────────┘   │
│                                                                        │
│  ⚠️ Bottleneck Detected:                                               │
│  Heap Storage consumed 52% of execution time. Consider:               │
│    • Adding a buffer pool to reduce disk I/O                          │
│    • Using clustered storage for better locality                      │
│                                                                        │
│  [View Detailed Metrics] [Compare with Another Design]                │
└────────────────────────────────────────────────────────────────────────┘
```

**Components:**

1. **MetricCard:**
   - Large value display
   - Label below
   - Optional comparison indicator (▲ +20% or ▼ -15%)
   - Tooltip with more details

2. **BlockBreakdownChart (Recharts):**
   - Horizontal bar chart
   - Sorted by percentage (largest first)
   - Color-coded by block category
   - Click bar to see block details

3. **BottleneckAnalyzer:**
   - Identify slowest block (>40% of time)
   - Generate suggestions based on block type
   - Suggestions are actionable (link to add recommended block)

**Suggestions Logic:**
```typescript
function generateSuggestions(metrics: ExecutionMetrics): Suggestion[] {
  const suggestions = [];

  // If storage is bottleneck, suggest buffer
  const storageBlock = findStorageBlock(metrics);
  if (storageBlock && storageBlock.percentage > 40) {
    suggestions.push({
      type: 'performance',
      message: 'Storage is the bottleneck. Add a buffer pool.',
      action: { type: 'addBlock', blockType: 'lru_buffer' }
    });
  }

  // If cache hit rate is low, suggest larger buffer
  if (metrics.cacheHitRate < 0.7) {
    suggestions.push({
      type: 'performance',
      message: 'Cache hit rate is low (${metrics.cacheHitRate}%). Increase buffer size.',
      action: { type: 'editParameter', blockId: bufferId, param: 'size' }
    });
  }

  return suggestions;
}
```

**Detailed Metrics View (Modal):**
- Per-block metrics table
- Latency histogram
- Operation breakdown (INSERT vs SELECT times)

**Export:**
- JSON: Full metrics object
- CSV: Summary table
- Clipboard: Quick copy summary
```

---

# Milestone 8: Multi-Design & Comparison

## Goal
Support multiple designs and side-by-side comparison view.

## Deliverables
- [ ] Design tabs or workspace selector
- [ ] Duplicate design functionality
- [ ] Side-by-side comparison mode
- [ ] Metrics diff highlighting
- [ ] Winner/loser indicators

## Prompt for Milestone 8

```
Continue the DB Simulator frontend. Implement multi-design support and comparison.

**Multi-Design Support:**

Add tabs or a design selector:
```
[Design A: OLTP] [Design B: Write-Heavy] [+ New Design]
```

**Design Store (Zustand):**
```typescript
interface DesignStore {
  designs: Design[];
  activeDesignId: string;

  // Actions
  createDesign: (name: string, template?: string) => Design;
  duplicateDesign: (designId: string) => Design;
  deleteDesign: (designId: string) => void;
  setActiveDesign: (designId: string) => void;
  renameDesign: (designId: string, name: string) => void;
}

interface Design {
  id: string;
  name: string;
  nodes: BlockInstance[];
  edges: Connection[];
  workload: Workload;
  lastExecutionResult?: ExecutionResult;
  createdAt: Date;
  updatedAt: Date;
}
```

**Comparison Mode:**

When user clicks "Compare" with 2+ designs:
```
┌─────────────────────────────────────┬────────────────────────────────────┐
│        Design A: OLTP               │       Design B: Write-Heavy        │
│        [Run] [Edit]                 │       [Run] [Edit]                 │
├─────────────────────────────────────┼────────────────────────────────────┤
│                                     │                                    │
│  ┌────────┐     ┌────────┐         │  ┌────────┐     ┌────────┐        │
│  │ Schema │────→│  Heap  │         │  │ Schema │────→│  LSM   │        │
│  └────────┘     └────────┘         │  └────────┘     │  Tree  │        │
│                      │              │                 └────────┘        │
│                      ↓              │                      │             │
│                ┌────────┐          │                      ↓             │
│                │ B-tree │          │                ┌────────┐         │
│                └────────┘          │                │ Skip   │         │
│                                     │                │ List   │         │
│                                     │                └────────┘         │
├─────────────────────────────────────┼────────────────────────────────────┤
│ Results:                            │ Results:                           │
│                                     │                                    │
│ Throughput:    1,234 ops/sec        │ Throughput:    1,856 ops/sec ✓    │
│ Latency (p99):     8.1 ms      ✓   │ Latency (p99):    12.3 ms         │
│ Cache Hit:          87%             │ Cache Hit:          82%            │
│ Pages Written:     542              │ Pages Written:      234 ✓         │
│ Pages Read:      1,234              │ Pages Read:       1,456            │
├─────────────────────────────────────┴────────────────────────────────────┤
│ Summary:                                                                 │
│ • Design B has 50% higher throughput (better for writes)                 │
│ • Design A has 35% lower latency (better for reads)                      │
│ • Design B uses 57% fewer page writes (better write amplification)       │
│                                                                          │
│ Recommendation: Choose Design B if write throughput is the priority.     │
│                                                                          │
│ [Export Report] [Choose A] [Choose B] [Exit Comparison]                  │
└──────────────────────────────────────────────────────────────────────────┘
```

**Comparison Logic:**
```typescript
interface ComparisonResult {
  designA: Design;
  designB: Design;
  metrics: {
    metric: string;
    valueA: number;
    valueB: number;
    winner: 'A' | 'B' | 'tie';
    difference: number;  // percentage
  }[];
  summary: string[];
  recommendation: string;
}

function compareDesigns(a: ExecutionResult, b: ExecutionResult): ComparisonResult {
  // Compare each metric
  // Determine winners
  // Generate summary sentences
  // Make recommendation based on workload type
}
```

**Visual Indicators:**
- ✓ Green checkmark on winner
- Percentage difference shown (+50%, -35%)
- Highlight winning values in green
- Summary uses plain language

**Actions:**
- "Choose A/B": Sets as active design, exits comparison
- "Export Report": Generate PDF/Markdown summary
- "Edit": Switch to single-design view for that design
```

---

# Milestone 9: Templates & Presets

## Goal
Create pre-built design templates and improve onboarding.

## Deliverables
- [ ] Template library (OLTP, Write-Heavy, Read-Heavy)
- [ ] "New from Template" flow
- [ ] Template preview before selecting
- [ ] Quick start guide / onboarding tutorial

## Prompt for Milestone 9

```
Continue the DB Simulator frontend. Implement templates and onboarding.

**Template Library:**

Templates are pre-configured designs users can start from:

```typescript
interface Template {
  id: string;
  name: string;
  description: string;
  category: 'oltp' | 'olap' | 'write-heavy' | 'read-heavy' | 'custom';
  thumbnail?: string;  // Preview image
  design: Partial<Design>;  // Blocks and connections
  workload: Workload;  // Matching workload
  tags: string[];
}
```

**Built-in Templates:**

1. **OLTP Balanced**
   - Schema → Heap Storage → B-tree Index
   - LRU Buffer Pool (128MB)
   - Workload: 50% read, 30% update, 20% insert

2. **Write-Heavy (Logging)**
   - Schema → LSM Tree Storage
   - WAL for durability
   - Workload: 80% insert, 20% read

3. **Read-Heavy (Analytics)**
   - Schema → Clustered B-tree Storage → Covering Index
   - Large LRU Buffer (512MB)
   - Workload: 95% read, 5% update

**Template Selection Modal:**
```
┌───────────────────────────────────────────────────────────────────┐
│ Start from Template                                        [✕]    │
├───────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐    │
│  │  [Preview img]  │ │  [Preview img]  │ │  [Preview img]  │    │
│  │                 │ │                 │ │                 │    │
│  │  OLTP Balanced  │ │  Write-Heavy    │ │  Read-Heavy     │    │
│  │                 │ │                 │ │                 │    │
│  │  Balanced read/ │ │  Optimized for  │ │  Optimized for  │    │
│  │  write workload │ │  high insert    │ │  query perf     │    │
│  │                 │ │  throughput     │ │                 │    │
│  │  [Use Template] │ │  [Use Template] │ │  [Use Template] │    │
│  └─────────────────┘ └─────────────────┘ └─────────────────┘    │
│                                                                   │
│  Or: [Start from Scratch]                                        │
└───────────────────────────────────────────────────────────────────┘
```

**Onboarding Tutorial:**

First-time user experience (can be skipped):

Step 1: Welcome modal
- "Design databases visually, compare performance instantly"
- [Start Tutorial] [Skip]

Step 2: Highlight block palette
- "These are building blocks. Each represents a database component."
- Pulse animation on palette

Step 3: Guide to drag a block
- "Drag 'Heap Storage' onto the canvas"
- Wait for user action

Step 4: Guide to connect blocks
- "Click the output port, then click an input port"
- Show connection animation

Step 5: Show parameter panel
- "Click a block to configure it"
- Highlight parameter panel

Step 6: Guide to run
- "Click Run to see how your design performs"
- Point to Run button

Step 7: Show metrics
- "These metrics show your design's performance"
- Highlight key metrics

Step 8: Celebrate
- "You just designed your first database!"
- [Try a Template] [Explore on Your Own]

**Tutorial State:**
```typescript
interface TutorialState {
  enabled: boolean;
  currentStep: number;
  completedSteps: string[];

  // Actions
  startTutorial: () => void;
  nextStep: () => void;
  skipTutorial: () => void;
  completeTutorial: () => void;
}
```

Use localStorage to track if user has completed tutorial.
```

---

# Milestone 10: Error Handling & Validation

## Goal
Comprehensive error handling, validation, and user feedback.

## Deliverables
- [ ] Pre-execution validation with clear errors
- [ ] Invalid connection feedback
- [ ] Execution error handling
- [ ] Toast notifications
- [ ] Undo/redo support

## Prompt for Milestone 10

```
Continue the DB Simulator frontend. Implement comprehensive error handling.

**Validation System:**

Create a validation system that checks designs before execution:

```typescript
interface Validator {
  validate(design: Design): ValidationResult;
}

interface ValidationResult {
  valid: boolean;
  errors: ValidationError[];
  warnings: ValidationWarning[];
}

interface ValidationError {
  code: string;
  message: string;
  blockId?: string;
  portId?: string;
  suggestion?: string;
}
```

**Validation Rules:**

1. **Missing Connections**
   - "Sequential Scan block has no input connection"
   - Suggestion: "Connect a Storage block's output to its input"

2. **Type Mismatch**
   - "Cannot connect INDEX_LOOKUP output to RECORD_STREAM input"
   - Suggestion: "Use a compatible port type"

3. **Cycle Detection**
   - "Design contains a cycle (Block A → Block B → Block A)"
   - Suggestion: "Remove one of the connections to break the cycle"

4. **Missing Required Block**
   - "Design has no storage block"
   - Suggestion: "Add a storage block (Heap, B-tree, or LSM Tree)"

5. **Invalid Parameters**
   - "Buffer size must be at least 1MB"
   - Suggestion: "Increase the buffer size parameter"

**Validation UI:**

Pre-run validation modal:
```
┌─────────────────────────────────────────────────────────┐
│ ⚠️ Design Has Issues                             [✕]    │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  Cannot run this design. Fix the following issues:      │
│                                                         │
│  ❌ Sequential Scan has no input                        │
│     → Connect a Storage block to its input              │
│     [Show on Canvas]                                    │
│                                                         │
│  ❌ B-tree Index is not connected to anything           │
│     → Connect it or remove it                           │
│     [Show on Canvas] [Delete Block]                     │
│                                                         │
│  ⚠️ Buffer size is small (32MB)                         │
│     → Consider increasing for better cache performance  │
│                                                         │
│                          [Cancel]  [Fix Issues]         │
└─────────────────────────────────────────────────────────┘
```

**Connection Feedback:**

When user tries invalid connection:
- Cursor shows ❌
- Target port shows red glow
- Tooltip: "Cannot connect: Type mismatch (DataStream → SingleValue)"
- Connection is not created

**Toast Notifications:**

Use a toast system for transient messages:
- Success: "Design saved" (green)
- Error: "Execution failed" (red)
- Warning: "Large workload may be slow" (yellow)
- Info: "Tip: Try adding a buffer pool" (blue)

```typescript
interface Toast {
  id: string;
  type: 'success' | 'error' | 'warning' | 'info';
  message: string;
  duration?: number;  // Auto-dismiss after ms
  action?: { label: string; onClick: () => void };
}
```

**Undo/Redo:**

Implement undo/redo for canvas operations:
- Add block
- Remove block
- Move block
- Create connection
- Remove connection
- Change parameter

Use a command pattern or Zustand middleware:
```typescript
interface HistoryState {
  past: CanvasState[];
  present: CanvasState;
  future: CanvasState[];

  undo: () => void;
  redo: () => void;
  canUndo: boolean;
  canRedo: boolean;
}
```

Keyboard shortcuts:
- Cmd/Ctrl+Z: Undo
- Cmd/Ctrl+Shift+Z: Redo
- Delete/Backspace: Remove selected
```

---

# Milestone 11: Persistence & Export

## Goal
Save designs locally and support import/export.

## Deliverables
- [ ] IndexedDB persistence (Dexie)
- [ ] Auto-save
- [ ] Export design as JSON
- [ ] Import design from JSON
- [ ] Export metrics report

## Prompt for Milestone 11

```
Continue the DB Simulator frontend. Implement persistence and export.

**IndexedDB Persistence (using Dexie):**

```typescript
// src/db/database.ts
import Dexie from 'dexie';

class DBSimulatorDatabase extends Dexie {
  designs!: Table<Design>;
  workloads!: Table<Workload>;
  settings!: Table<UserSettings>;

  constructor() {
    super('DBSimulator');
    this.version(1).stores({
      designs: '++id, name, updatedAt',
      workloads: '++id, name',
      settings: 'key'
    });
  }
}

export const db = new DBSimulatorDatabase();
```

**Auto-Save:**
- Debounce saves (500ms after last change)
- Save on: block add/remove, connection change, parameter change
- Show save indicator in top bar (Saved ✓ / Saving...)

**Design Schema:**
```typescript
interface PersistedDesign {
  id: string;
  name: string;
  version: number;  // Schema version for migrations
  nodes: SerializedNode[];
  edges: SerializedEdge[];
  workload: Workload;
  lastExecutionResult?: ExecutionResult;
  createdAt: string;  // ISO date
  updatedAt: string;
}
```

**Export Design:**
```typescript
function exportDesign(design: Design): string {
  const exportData = {
    version: '1.0',
    exportedAt: new Date().toISOString(),
    design: serializeDesign(design)
  };
  return JSON.stringify(exportData, null, 2);
}
```

Export options:
- Copy to clipboard
- Download as .json file
- Download as .dbsim file (same JSON, custom extension)

**Import Design:**
```typescript
async function importDesign(json: string): Promise<Design> {
  const data = JSON.parse(json);

  // Validate version
  if (!isCompatibleVersion(data.version)) {
    throw new Error('Incompatible design version');
  }

  // Validate structure
  const validation = validateDesignStructure(data.design);
  if (!validation.valid) {
    throw new Error(`Invalid design: ${validation.errors.join(', ')}`);
  }

  return deserializeDesign(data.design);
}
```

Import UI:
- Drag & drop .json file onto canvas
- Or: File menu → Import → File picker

**Export Metrics Report:**

Generate a shareable report:
```markdown
# Database Design Comparison Report

Generated: 2026-01-26 14:30:00

## Design: OLTP with Heap Storage

### Architecture
- Storage: Heap File (8KB pages)
- Index: B-tree on id column
- Buffer: LRU (128MB)

### Workload
- 10,000 operations
- 50% SELECT, 30% UPDATE, 20% INSERT
- Zipfian distribution
- 100 concurrent operations

### Results
| Metric | Value |
|--------|-------|
| Throughput | 1,234 ops/sec |
| Latency (p99) | 8.1 ms |
| Cache Hit Rate | 87% |
| Pages Written | 542 |
| Pages Read | 1,234 |

### Block Breakdown
| Block | Time | Percentage |
|-------|------|------------|
| Heap Storage | 1.7s | 52% |
| B-tree Index | 1.2s | 38% |
| LRU Buffer | 0.3s | 10% |
```

Export formats:
- Markdown (.md)
- JSON (raw data)
- CSV (metrics table)
```

---

# Milestone 12: WASM Integration Prep

## Goal
Prepare the frontend for integration with the Rust block-system compiled to WASM.

## Deliverables
- [ ] WASM bridge interface definition
- [ ] Block execution interface matching Rust traits
- [ ] Metrics collection interface
- [ ] Fallback to mock engine if WASM not loaded

## Prompt for Milestone 12

```
Continue the DB Simulator frontend. Prepare for WASM integration with the Rust block-system.

**WASM Bridge Architecture:**

```
Frontend (React/TS)
       │
       ▼
┌─────────────────┐
│  WASM Bridge    │  ← TypeScript interface
│  (Abstraction)  │
└─────────────────┘
       │
       ▼
┌─────────────────┐
│  wasm-bindgen   │  ← Auto-generated bindings
│  Generated API  │
└─────────────────┘
       │
       ▼
┌─────────────────┐
│  Rust Block     │  ← Compiled to WASM
│  System         │
└─────────────────┘
```

**Bridge Interface:**

```typescript
// src/wasm/bridge.ts

interface WASMBridge {
  // Check if WASM is loaded and ready
  isReady(): boolean;

  // Initialize the block runtime
  initRuntime(): Promise<void>;

  // Register a block configuration
  registerBlock(config: BlockConfig): Promise<string>;

  // Create a connection between blocks
  createConnection(source: PortRef, target: PortRef): Promise<string>;

  // Validate the design
  validate(): Promise<ValidationResult>;

  // Execute the workload
  execute(workload: WorkloadConfig): Promise<ExecutionResult>;

  // Get metrics from last execution
  getMetrics(): Promise<MetricsResult>;

  // Cancel running execution
  cancel(): void;
}

// Matches Rust BlockMetadata
interface BlockConfig {
  type: string;
  id: string;
  parameters: Record<string, any>;
}

interface PortRef {
  blockId: string;
  portName: string;
}

interface WorkloadConfig {
  operations: OperationConfig[];
  distribution: string;
  concurrency: number;
  totalOps: number;
}
```

**WASM Loading:**

```typescript
// src/wasm/loader.ts

let wasmModule: WASMModule | null = null;

export async function loadWASM(): Promise<void> {
  try {
    // Dynamic import of WASM module
    const module = await import('../pkg/block_system');
    await module.default();  // Initialize WASM
    wasmModule = module;
    console.log('WASM loaded successfully');
  } catch (error) {
    console.error('Failed to load WASM:', error);
    // Fallback to mock engine
  }
}

export function isWASMReady(): boolean {
  return wasmModule !== null;
}
```

**Execution Engine Abstraction:**

```typescript
// src/engine/ExecutionEngine.ts

interface ExecutionEngine {
  validate(design: Design): Promise<ValidationResult>;
  execute(design: Design, workload: Workload): Promise<ExecutionResult>;
  cancel(): void;
}

// Factory function
export function createExecutionEngine(): ExecutionEngine {
  if (isWASMReady()) {
    return new WASMExecutionEngine();
  } else {
    console.warn('WASM not available, using mock engine');
    return new MockExecutionEngine();
  }
}
```

**WASM Execution Engine:**

```typescript
// src/engine/WASMExecutionEngine.ts

class WASMExecutionEngine implements ExecutionEngine {
  async validate(design: Design): Promise<ValidationResult> {
    const bridge = getWASMBridge();

    // Clear previous state
    await bridge.initRuntime();

    // Register all blocks
    for (const node of design.nodes) {
      await bridge.registerBlock({
        type: node.type,
        id: node.id,
        parameters: node.parameters
      });
    }

    // Create connections
    for (const edge of design.edges) {
      await bridge.createConnection(
        { blockId: edge.source, portName: edge.sourceHandle },
        { blockId: edge.target, portName: edge.targetHandle }
      );
    }

    // Validate
    return bridge.validate();
  }

  async execute(design: Design, workload: Workload): Promise<ExecutionResult> {
    await this.validate(design);

    const bridge = getWASMBridge();
    return bridge.execute({
      operations: workload.operations.map(op => ({
        type: op.type,
        weight: op.weight,
        template: op.template
      })),
      distribution: workload.distribution,
      concurrency: workload.concurrency,
      totalOps: workload.totalOperations
    });
  }

  cancel(): void {
    getWASMBridge().cancel();
  }
}
```

**Progress Callbacks:**

For long-running WASM execution, use a callback mechanism:

```typescript
interface ExecutionCallbacks {
  onProgress: (progress: number, phase: string) => void;
  onBlockComplete: (blockId: string, metrics: BlockMetrics) => void;
  onComplete: (result: ExecutionResult) => void;
  onError: (error: Error) => void;
}
```

**Type Mapping:**

Create TypeScript types that match Rust types exactly:

```typescript
// Must match block-system/src/core/port.rs
type PortType = 'DataStream' | 'SingleValue' | 'Batch' | 'Signal' | 'Transaction' | 'Schema' | 'Statistics' | 'Config';

// Must match block-system/src/categories/mod.rs
type BlockCategory = 'Storage' | 'Index' | 'Buffer' | 'Concurrency' | 'Execution' | 'Transaction' | 'Compression' | 'Partitioning' | 'Optimization' | 'Distribution';

// Must match block-system/src/core/metrics.rs
interface Metric {
  name: string;
  type: 'Counter' | 'Gauge' | 'Histogram' | 'Timing';
  value: number | number[];
}
```
```

---

# Summary: Milestone Prompts Quick Reference

| # | Milestone | Key Deliverable | Est. Sessions |
|---|-----------|-----------------|---------------|
| 1 | Project Setup | Vite + React Flow + Layout shell | 1-2 |
| 2 | Block Definitions | Type system + Block palette | 2 |
| 3 | Block Nodes | Custom nodes + Connections | 2 |
| 4 | Parameter Panel | Configuration UI + Validation | 1-2 |
| 5 | Workload Editor | Operations + Distribution | 1-2 |
| 6 | Mock Execution | Simulated engine + Progress | 2 |
| 7 | Metrics Dashboard | Charts + Bottleneck detection | 2 |
| 8 | Multi-Design | Tabs + Comparison view | 2-3 |
| 9 | Templates | Presets + Onboarding | 1-2 |
| 10 | Error Handling | Validation + Undo/Redo | 2 |
| 11 | Persistence | IndexedDB + Export/Import | 1-2 |
| 12 | WASM Prep | Bridge interface + Abstraction | 2 |

**Total: ~20-25 sessions**

---

# Getting Started

To begin, use the **Milestone 1 prompt** to set up the project scaffold. Each subsequent milestone builds on the previous, so follow the order.

After the frontend is complete through Milestone 11, we can:
1. Compile the Rust block-system to WASM
2. Integrate using the Milestone 12 bridge
3. Replace mock execution with real block execution

The mock execution engine (Milestone 6) ensures we can develop and test the full UI before the WASM integration is complete.
