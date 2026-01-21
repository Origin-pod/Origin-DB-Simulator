# Design Document: Modular DB Builder

**Last Updated**: 2026-01-20
**Design Lead**: [TBD]
**Status**: Pre-Design → Design → Implementation

---

## Step 1: Understand the Problem

### The User's Goal (One Sentence)

**"I need to know which database architecture performs better for my workload — without spending weeks coding it."**

### Primary User: The Database Architect

**Who they are**:
- Senior engineer or technical lead
- Designing a new system or re-platforming
- Understands database internals (B-trees, MVCC, LSM)
- Time-constrained, budget-constrained
- Works in high-stakes environments (wrong choice = expensive)

**What they're trying to achieve**:
1. **Evaluate tradeoffs** between design choices (LSM vs B-tree, normalized vs denormalized)
2. **Experiment quickly** without writing thousands of lines of code
3. **Make data-driven decisions** backed by metrics, not guesses
4. **Communicate designs** to team members or stakeholders

**What they're NOT trying to do**:
- Learn basic database concepts (they already know them)
- Build a production database from scratch
- Write SQL queries (that's a different tool)

### Secondary Users (Post-MVP)

- **Educators**: Teaching database internals to students
- **Researchers**: Prototyping new algorithms (e.g., learned indexes)
- **Database vendors**: Experimenting with new features

### User Context

**When will they use this?**
- Early in a project (design phase)
- When facing a performance problem (need to re-architect)
- During research or learning (understanding tradeoffs)

**What are they doing right before/after?**
- Before: Reading papers, benchmarks, documentation
- After: Implementing the chosen design in production code

**What devices/environments?**
- Desktop/laptop (not mobile — too complex for small screens)
- Modern browsers (Chrome, Firefox, Safari)
- Likely engineers working from home or office

---

## Step 2: Define Success

### Measurable Outcomes

**Primary Success Metrics**:

1. **Time to Decision**: User completes a design comparison in <30 minutes
   - Today: 2-4 weeks to implement and test one alternative
   - Target: 30 minutes to test 3 alternatives

2. **Decision Confidence**: User says "I'm confident this design is better" (qualitative)
   - Measure via post-session survey: "How confident are you in your decision?" (1-10 scale)
   - Target: ≥8/10 confidence

3. **Design Iteration Rate**: User creates and compares ≥3 designs per session
   - If they only create 1 design, tool isn't enabling experimentation
   - Target: 3+ designs per session

**Secondary Metrics**:

- **Comprehension**: User understands what each block does (measured by asking "what does this block do?" for 3 random blocks)
- **Task Completion**: User successfully builds a working design from scratch (without template) in <45 minutes
- **Retention**: User returns within 7 days to try another workload

### What "Good" Looks Like

**Scenario**: Database architect is designing a time-series database for IoT data.

**Current State** (without tool):
- Reads 5 papers on LSM trees and B-trees
- Guesses that LSM might be better for writes
- Spends 2 weeks implementing LSM prototype
- Discovers it's only 20% faster (not worth the complexity)
- Ships B-tree because re-work would take another 2 weeks

**Future State** (with tool):
- Opens tool, starts from "Write-Heavy" template
- Swaps LSM for B-tree
- Runs workload (10K inserts/sec, 100 reads/sec)
- Sees LSM is 3x faster for this specific workload
- Ships LSM with confidence

**Time**: 30 minutes vs 2 weeks

---

## Step 3: Structure Before Styling

### Information Architecture

The system has **5 core objects** users interact with:

```
1. Blocks — Reusable components (storage, indexes, etc.)
2. Canvas — Visual workspace where blocks are composed
3. Connections — Wires between blocks (data flow)
4. Workload — Query patterns to execute
5. Metrics — Performance results
```

**Hierarchy** (what's most important):

```
Level 1 (Critical Path):
  ├─ Canvas (where user spends 60% of time)
  ├─ Metrics (where user makes decisions)

Level 2 (Supporting):
  ├─ Block Library (where user finds components)
  ├─ Workload Editor (where user defines patterns)

Level 3 (Tertiary):
  ├─ Settings/Config
  ├─ Help/Docs
  ├─ Export/Share
```

### Mental Models

#### User's Existing Mental Model

Most database architects think in terms of:

1. **Layers**: Storage → Indexing → Query Execution
2. **Data flow**: Records flow through system (insert → storage → index)
3. **Tradeoffs**: Every design choice has pros/cons (read speed vs write speed)

**We should match this mental model.**

#### The Core Metaphor

**"Database as LEGO"** — Users compose independent blocks to build systems.

**Why this works**:
- ✅ Familiar (everyone understands LEGO)
- ✅ Emphasizes modularity (blocks are independent)
- ✅ Encourages experimentation (easy to swap pieces)
- ❌ Risk: May oversimplify complexity (not all blocks compose cleanly)

**Alternative metaphor** (considered but rejected):
- "Database as Circuit Board" — More accurate (wires, signals) but less approachable
- "Database as Factory" — Implies sequential pipeline, less flexible

### Layout (High-Level)

**Primary Layout** (desktop, 16:9 screen):

```
┌────────────────────────────────────────────────────────────┐
│  Top Bar: [Logo] [Design Name] [Run] [Compare]    [Share] │
├──────┬──────────────────────────────────────────┬──────────┤
│      │                                          │          │
│ 1    │              2                           │    3     │
│ Block│          Canvas                          │  Params  │
│ Lib  │      (Drag & Drop Area)                  │  Panel   │
│      │                                          │          │
│ [B1] │   ┌────┐      ┌────┐                     │  Name:   │
│ [B2] │   │ S1 │─────→│ I1 │                     │  B-tree  │
│ [B3] │   └────┘      └────┘                     │          │
│ [B4] │                                          │  Fanout: │
│      │                                          │  [128__] │
│      │                                          │          │
├──────┴──────────────────────────────────────────┴──────────┤
│                    4                                        │
│              Metrics Dashboard                              │
│  Throughput: 1,234 ops/sec | Latency: 8ms | IO: 542 pages │
└────────────────────────────────────────────────────────────┘
```

**Zones**:

1. **Block Library** (left, 15% width)
   - Scrollable list of blocks
   - Grouped by category (Storage, Indexes, etc.)
   - Search bar at top

2. **Canvas** (center, 60% width)
   - Main workspace
   - Infinite pan/zoom
   - Grid background (subtle)

3. **Parameter Panel** (right, 25% width)
   - Shows when block is selected
   - Contextual (changes based on block)

4. **Metrics Dashboard** (bottom, collapsible)
   - Hidden by default (only shows after execution)
   - Expandable to full-screen for deep analysis

**Why this layout**:
- ✅ Matches user flow: Find block (left) → Place on canvas (center) → Configure (right) → See results (bottom)
- ✅ Canvas is largest area (where user focuses)
- ✅ Metrics are below (natural reading order: design → results)
- ❌ Risk: Right panel may feel cramped on smaller screens

---

### User Flows

#### Flow 1: First-Time User (Guided Tutorial)

**Goal**: Build confidence, reduce intimidation

```
1. Welcome Modal
   "Let's design your first database in 5 minutes"
   [Start Tutorial] [Skip]

2. Highlight Block Library
   "These are the building blocks. Let's start simple."
   → Auto-select "Heap File Storage"

3. Prompt: Drag to Canvas
   "Drag this block onto the canvas"
   → User drags, block snaps into place

4. Highlight Parameter Panel
   "Each block has settings. Let's keep defaults for now."

5. Add Another Block
   "Now add a B-tree Index"
   → User drags B-tree onto canvas

6. Connect Blocks
   "Click the output port, then click the input port"
   → User creates connection

7. Define Workload
   "Let's simulate 1000 inserts"
   → Pre-filled workload appears

8. Run Simulation
   "Click Run to see what happens"
   → Metrics appear

9. Celebrate
   "You just designed your first database! 🎉"
   "Now try changing Heap Storage to LSM Tree and run again"

10. End Tutorial
    [Explore Templates] [Start from Scratch]
```

**Time**: 3-5 minutes

**Why this works**:
- ✅ Hands-on immediately (not passive reading)
- ✅ Small wins build confidence
- ✅ Ends with clear next step

---

#### Flow 2: Experienced User (Fast Path)

**Goal**: Get out of the way, let them work

```
1. Open Tool
   → Canvas is blank, templates in top-right

2. Start from Template
   "New from Template" → Select "OLTP (Balanced)"
   → Canvas populates with working design

3. Customize
   → Swap Heap Storage for Clustered B-tree
   → Adjust buffer size to 256MB

4. Run Workload
   → Click Run
   → Metrics appear in 3 seconds

5. Compare Alternative
   → Click "Duplicate Design"
   → Swap B-tree for LSM Tree
   → Click "Compare" → Split screen appears

6. Make Decision
   → See Design B is 40% faster
   → Export design as JSON or screenshot

7. Done
```

**Time**: 10-15 minutes

**Why this works**:
- ✅ No handholding (respects expertise)
- ✅ Templates accelerate start
- ✅ Comparison is one-click

---

#### Flow 3: Iterative Refinement

**Goal**: Enable rapid experimentation

```
1. User has working design (e.g., Heap + B-tree)

2. Hypothesis: "LSM might be faster for writes"

3. Swap Block
   → Select Heap Storage block
   → Press Delete
   → Drag LSM Tree Storage
   → Connections auto-reconnect (smart routing)

4. Run Again
   → Click Run
   → New metrics appear

5. Compare
   → Toggle between "Before" and "After" metrics
   → See 2x write throughput improvement

6. Refine Further
   → Adjust LSM compaction strategy
   → Run again
   → See marginal improvement

7. Decide
   → LSM with Tiered Compaction is the winner
   → Export final design
```

**Time**: 5 minutes per iteration

**Why this works**:
- ✅ Low friction (swap blocks easily)
- ✅ Immediate feedback (fast execution)
- ✅ Encourages experimentation

---

### Interaction Patterns

#### Pattern 1: Block Placement

**Affordance**: Draggable blocks, canvas accepts drops

**Interaction**:
1. Hover over block in library → Cursor changes to grab hand
2. Click and drag → Ghost/preview appears
3. Hover over canvas → Drop zone highlights
4. Release → Block snaps to grid

**Visual Feedback**:
- Dragging: Semi-transparent block follows cursor
- Valid drop zone: Canvas area glows subtly
- Snap to grid: Block animates into position

**What could confuse users**:
- ❌ "Can I place a block anywhere?" (Yes, but connections matter)
- ❌ "What if I place blocks with no connections?" (Design won't execute)

**How we reduce confusion**:
- Show validation errors before execution ("Block X has no inputs")
- Suggest valid placements (e.g., "Storage blocks usually go here")

---

#### Pattern 2: Connection Creation

**Affordance**: Ports are clickable, lines connect them

**Interaction**:
1. Click output port → Port highlights
2. Drag to input port → Temporary line follows cursor
3. Hover over compatible input → Input glows green
4. Hover over incompatible input → Input shows red X
5. Release → Connection created (or error shown)

**Visual Feedback**:
- Compatible: Green glow + green line
- Incompatible: Red X + error tooltip ("Type mismatch: INDEX_LOOKUP cannot connect to RECORD_STREAM")
- Connected: Solid line with arrow (indicates direction)

**What could confuse users**:
- ❌ "Why can't I connect these?" (Type mismatch)
- ❌ "Which direction is data flowing?" (Arrows help)

**How we reduce confusion**:
- Type labels on hover ("Output: RECORD_STREAM", "Input: RECORD_STREAM")
- Color-coded port types (all RECORD_STREAM ports are blue)

---

#### Pattern 3: Block Configuration

**Affordance**: Click block → Parameter panel appears

**Interaction**:
1. Click block on canvas → Block highlights
2. Parameter panel slides in from right
3. User edits parameters (number inputs, checkboxes, dropdowns)
4. Changes apply in real-time (or on "Apply" button)

**Visual Feedback**:
- Selected block: Blue outline, slightly elevated (shadow)
- Parameter changes: Input highlights briefly when changed
- Validation errors: Red border + error text below input

**What could confuse users**:
- ❌ "Do I need to save changes?" (Auto-save vs explicit save)
- ❌ "What do these parameters mean?" (Tooltips needed)

**How we reduce confusion**:
- Auto-save by default (no explicit save button)
- Inline help icons (? icon) with explanations
- Sensible defaults (user can ignore params if they want)

---

### Progressive Disclosure

**Principle**: Show only what's needed, hide complexity until requested.

**Level 1** (Beginner):
- Show 5-10 most common blocks
- Hide advanced blocks (e.g., custom compaction strategies)
- Pre-filled workloads
- Simple metrics (throughput, latency)

**Level 2** (Intermediate):
- Show all 15 blocks
- Custom workloads
- Detailed metrics (breakdown by block)

**Level 3** (Advanced):
- Custom blocks (SDK)
- Advanced configuration (cost model tuning)
- Export to code

**How to implement**:
- Default view is Level 1
- "Show Advanced Blocks" checkbox in Block Library
- "Advanced Metrics" toggle in dashboard

---

## Step 4: Justify Design Decisions

### Decision 1: Visual Canvas vs Code-Based Editor

**Choice**: Visual canvas with drag-and-drop blocks

**Problem it solves**:
- Reduces friction (no syntax to learn)
- Makes architecture visible (see entire design at once)
- Encourages experimentation (easy to swap blocks)

**Why this option is better than alternatives**:

**Alternative A**: Code-based DSL (e.g., YAML config)
```yaml
design:
  storage: heap_file
  indexes:
    - type: btree
      columns: [id]
```
- ✅ Pros: Precise, version-controllable, familiar to engineers
- ❌ Cons: Higher friction, requires syntax knowledge, less visual

**Alternative B**: Form-based wizard
```
Step 1: Choose storage engine [Heap | LSM | B-tree]
Step 2: Add indexes...
```
- ✅ Pros: Guided, simple
- ❌ Cons: Linear (can't see whole design), less flexible

**Why visual wins**:
- Matches mental model (architects think visually)
- Faster iteration (drag-drop faster than typing)
- Better for comparison (see two designs side-by-side)

**Tradeoff we accept**: Less precise than code, harder to version control

**Mitigation**: Export designs as JSON (can be diffed)

---

### Decision 2: Block Granularity (Medium)

**Choice**: One block = one conceptual component (e.g., "B-tree Index", "Heap Storage")

**Problem it solves**:
- Balances flexibility and simplicity
- Matches how database architects think

**Why this option is better than alternatives**:

**Alternative A**: Fine-grained (atomic blocks)
- Example: "B-tree Node", "Page Allocator", "Split Logic"
- ✅ Pros: Maximum flexibility, can customize everything
- ❌ Cons: Too complex (50+ blocks for simple design), overwhelming

**Alternative B**: Coarse-grained (monolithic blocks)
- Example: "PostgreSQL Storage Layer" (includes heap + indexes + buffer)
- ✅ Pros: Simple, fewer blocks
- ❌ Cons: Not flexible (can't swap components), defeats purpose

**Why medium wins**:
- User can understand what each block does in one sentence
- Can compose different designs without combinatorial explosion
- Aligns with how databases are conceptually structured

**Tradeoff we accept**: Not perfect for every use case (some users want finer control)

**Mitigation**: Block parameters allow tuning (e.g., B-tree fanout, fill factor)

---

### Decision 3: Metrics Dashboard Placement (Bottom)

**Choice**: Metrics appear at bottom of screen, collapsible

**Problem it solves**:
- Natural reading order (design above, results below)
- Doesn't occlude canvas during design phase
- Easy to expand for deep analysis

**Why this option is better than alternatives**:

**Alternative A**: Right sidebar (next to parameters)
```
[Block Library] [Canvas        ] [Params | Metrics]
```
- ✅ Pros: All info on right side
- ❌ Cons: Cramped, competes with parameters, less space for charts

**Alternative B**: Modal/overlay
```
[Run] → Metrics appear as overlay on top of canvas
```
- ✅ Pros: Full-screen metrics
- ❌ Cons: Hides design, can't compare visually, disruptive

**Alternative C**: Separate page
```
[Run] → Navigate to Results page
```
- ✅ Pros: Full-screen, dedicated space
- ❌ Cons: Context switch, can't see design + metrics together

**Why bottom wins**:
- Design and metrics visible simultaneously (no context switch)
- Collapsible (doesn't waste space when not needed)
- Expandable (can go full-screen if needed)

**Tradeoff we accept**: Less vertical space for canvas

**Mitigation**: Canvas is pan/zoomable (infinite space)

---

### Decision 4: Pre-Built Templates vs Blank Canvas

**Choice**: Offer templates but also allow blank canvas

**Problem it solves**:
- Reduces time to first working design (templates)
- Doesn't force users down a path (blank canvas available)

**Why this option is better than alternatives**:

**Alternative A**: Always start from template (forced)
- ✅ Pros: Faster onboarding, less intimidating
- ❌ Cons: Frustrating for experts, may bias toward certain designs

**Alternative B**: Always start from blank canvas
- ✅ Pros: Maximum flexibility
- ❌ Cons: Intimidating for beginners, slower start

**Why hybrid wins**:
- Beginners use templates (guided)
- Experts use blank canvas (freedom)
- Templates are educational (show best practices)

**Tradeoff we accept**: Need to maintain template library

**Mitigation**: Start with 3 templates (OLTP, Write-Heavy, Read-Heavy), add more post-MVP

---

### Decision 5: Real-Time Execution vs Step-Through Debugger

**Choice**: Real-time execution (run entire workload, show results), with optional step-through

**Problem it solves**:
- Fast feedback (3-5 seconds for 10K operations)
- Matches user goal (compare designs quickly)

**Why this option is better than alternatives**:

**Alternative A**: Step-through only (execute one operation at a time)
```
Operation 1: INSERT → Heap Storage writes page 1
Operation 2: INSERT → Heap Storage writes page 1 (same page)
...
```
- ✅ Pros: Educational, shows exactly what happens
- ❌ Cons: Slow (10K operations = 10K clicks), tedious

**Alternative B**: Animated execution (watch data flow through blocks in real-time)
- ✅ Pros: Engaging, visual
- ❌ Cons: Slow, may be distracting

**Why real-time wins**:
- Optimizes for speed (user's #1 need)
- Still educational (metrics show what happened)
- Optional step-through for deep dives

**Tradeoff we accept**: Less visibility into individual operations

**Mitigation**: Detailed metrics (show breakdown by block, operation type)

---

## Step 5: Iterate (Alternative Designs)

### Alternative 1: Minimalist (Text-First)

**What if we prioritized simplicity over visuals?**

**Design**:
```
Simple list-based interface:

Design: My OLTP Database

Storage Engine: [Heap File ▼]
Indexes:
  + Add Index
    ├─ B-tree on id (primary)
    └─ B-tree on email
Buffer Pool: [LRU, 128MB ▼]
Concurrency: [MVCC ▼]

[Run Workload]

Results:
  Throughput: 1,234 ops/sec
  Latency: 8ms
```

**Pros**:
- ✅ Simpler to build (no canvas complexity)
- ✅ Faster for experts (keyboard-driven)
- ✅ Easier to understand (linear, no spatial reasoning)

**Cons**:
- ❌ Less visual (can't see architecture at a glance)
- ❌ Harder to compare (two designs = two lists)
- ❌ Less engaging (may feel like a boring form)

**When this might be better**:
- If users are CLI-first (command-line tools)
- If visual canvas is too complex to build
- If simplicity is the #1 priority

**Why we're not choosing this**:
- Database architecture is inherently visual (layers, data flow)
- Comparison is harder (can't put two lists side-by-side meaningfully)
- Less differentiated (feels like a form, not a design tool)

---

### Alternative 2: Maximalist (Full IDE)

**What if we built a full database IDE with code generation?**

**Design**:
```
┌────────────────────────────────────────────────┐
│ [File] [Edit] [View] [Build] [Run] [Debug]    │
├──────┬─────────────────────────────────┬───────┤
│      │                                 │       │
│ File │  Code Editor                    │ Props │
│ Tree │  (TypeScript/Rust)              │       │
│      │                                 │       │
│ ├ src│  class HeapFileStorage {        │       │
│ ├ lib│    pages: Page[] = [];          │       │
│ └ cfg│    insert(record: Record) {     │       │
│      │      // ...                     │       │
│      │    }                            │       │
│      │  }                              │       │
│      │                                 │       │
├──────┴─────────────────────────────────┴───────┤
│ Terminal:                                      │
│ $ npm run build                                │
│ ✓ Compiled successfully                        │
└────────────────────────────────────────────────┘
```

**Pros**:
- ✅ Maximum power (can customize everything)
- ✅ Production-ready (generate real code)
- ✅ Familiar to engineers (IDE-like)

**Cons**:
- ❌ Months to build (huge scope)
- ❌ High friction (requires coding)
- ❌ Misses the point (goal is to avoid coding)

**When this might be better**:
- If generating production code is the goal
- If users want to extend deeply
- If we have 6+ months and a large team

**Why we're not choosing this**:
- Defeats the purpose (we want to avoid coding during design phase)
- Too slow to build (MVP needs to ship in 8 weeks)
- Doesn't optimize for speed to decision

---

### Alternative 3: Conversational (AI-Driven)

**What if we used AI to design databases for users?**

**Design**:
```
┌────────────────────────────────────────────────┐
│ AI Database Design Assistant                   │
├────────────────────────────────────────────────┤
│                                                │
│ You: "I need a database for IoT time-series   │
│       data with high write throughput"         │
│                                                │
│ AI:  "Based on your requirements, I recommend  │
│       LSM Tree storage with tiered compaction. │
│       Would you like me to generate a design?" │
│                                                │
│ You: "Yes"                                     │
│                                                │
│ AI:  [Generates design]                        │
│      "I've created a design with:              │
│       - LSM Tree Storage                       │
│       - Skip List Index                        │
│       - Clock Buffer Pool                      │
│                                                │
│      Estimated throughput: 50K inserts/sec     │
│      Should I simulate this?"                  │
│                                                │
│ [Yes] [Modify] [Compare Alternatives]          │
└────────────────────────────────────────────────┘
```

**Pros**:
- ✅ Lowest friction (just describe your needs)
- ✅ Guides users (AI knows best practices)
- ✅ Futuristic (on-trend with AI)

**Cons**:
- ❌ Black box (user doesn't learn, just trusts AI)
- ❌ Requires LLM integration (complex, expensive)
- ❌ May not be accurate (AI hallucinations)

**When this might be better**:
- If users are non-technical
- If we have strong AI capabilities
- If guidance is more important than education

**Why we're not choosing this (for MVP)**:
- Goal is to enable experimentation, not automate it away
- Users want to understand tradeoffs, not just get an answer
- MVP should be simple (AI adds complexity)

**Post-MVP consideration**: Add AI as a co-pilot (suggest blocks, not replace canvas)

---

## Design System (Visual Language)

### Color Palette

**Primary Colors** (convey meaning):
- **Blue** (#3B82F6): Default/neutral (blocks, connections)
- **Green** (#10B981): Success (valid connections, positive metrics)
- **Red** (#EF4444): Error (invalid connections, bottlenecks)
- **Yellow** (#F59E0B): Warning (approaching limits)
- **Gray** (#6B7280): Secondary/inactive

**Category Colors** (for blocks):
- **Storage**: Purple (#8B5CF6)
- **Indexes**: Blue (#3B82F6)
- **Buffers**: Teal (#14B8A6)
- **Concurrency**: Orange (#F97316)
- **Query Execution**: Indigo (#6366F1)

**Why color matters**:
- ✅ Reduces cognitive load (recognize block types by color)
- ✅ Provides visual feedback (green = good, red = bad)
- ❌ Risk: Colorblind users may struggle

**Mitigation**: Also use icons and labels (not color alone)

---

### Typography

**Principles**:
- **Clarity over style** (readable > decorative)
- **Hierarchy through size and weight** (not just color)
- **Monospace for technical terms** (block names, parameters)

**Type Scale**:
- **Display** (24px, Bold): Page titles
- **Heading** (18px, Semibold): Section headers
- **Body** (14px, Regular): Default text
- **Small** (12px, Regular): Labels, hints
- **Code** (14px, Monospace): Block names, parameter values

**Font Families**:
- **Sans-serif**: Inter or System UI (body text)
- **Monospace**: JetBrains Mono or Fira Code (technical text)

**Why this matters**:
- ✅ Technical users expect monospace for code-like content
- ✅ Clear hierarchy guides eye to important info
- ❌ Risk: Too much variation = visual noise

---

### Spacing & Layout

**Principles**:
- **8px grid system** (all spacing in multiples of 8)
- **Generous whitespace** (reduce clutter)
- **Group related elements** (proximity = relationship)

**Spacing Scale**:
- **xs**: 4px (tight spacing, inline elements)
- **sm**: 8px (default spacing)
- **md**: 16px (between sections)
- **lg**: 24px (major sections)
- **xl**: 32px (page margins)

**Why this matters**:
- ✅ Consistent spacing creates rhythm
- ✅ Visual grouping reduces cognitive load
- ✅ 8px grid works well with modern displays

---

### Iconography

**Principles**:
- **One style throughout** (outline or filled, not mixed)
- **Size consistency** (16px or 24px, not arbitrary)
- **Meaningful, not decorative** (every icon serves a purpose)

**Icon Library**: Heroicons or Lucide (open-source, consistent)

**Key Icons**:
- **Storage**: Hard drive / database cylinder
- **Index**: Tree structure / book index
- **Buffer**: Layers / cache
- **Concurrency**: Lock / parallel lines
- **Query**: Magnifying glass / search
- **Run**: Play button
- **Compare**: Side-by-side arrows
- **Help**: Question mark
- **Settings**: Gear

**Why this matters**:
- ✅ Icons provide quick recognition (faster than reading text)
- ✅ Saves space (icon + label or icon alone)
- ❌ Risk: Unclear icons confuse users

**Mitigation**: Always pair icons with labels (at least on first use)

---

## Accessibility Considerations

### Keyboard Navigation

**Must support**:
- **Tab**: Move between blocks, parameters, buttons
- **Arrow keys**: Navigate canvas (pan)
- **Enter**: Activate selected element (run, connect)
- **Delete**: Remove selected block or connection
- **Cmd/Ctrl + Z**: Undo
- **Cmd/Ctrl + C/V**: Copy/paste blocks

**Why this matters**:
- ✅ Faster for power users (keyboard > mouse)
- ✅ Required for accessibility (screen reader users)

---

### Screen Reader Support

**Must provide**:
- **ARIA labels**: Describe blocks, connections, metrics
- **Semantic HTML**: Use buttons (not divs) for actions
- **Focus indicators**: Clear outline on focused elements
- **Alt text**: For all icons and visualizations

**Example**:
```html
<button aria-label="Run workload on current design">
  <PlayIcon /> Run
</button>
```

**Why this matters**:
- ✅ Inclusive (users with visual impairments)
- ✅ Legal requirement (ADA compliance in some contexts)

---

### Color Contrast

**WCAG AA Standard**: 4.5:1 contrast ratio for text

**Must ensure**:
- Text on background: High contrast
- Disabled states: Still readable (3:1 minimum)
- Focus indicators: Distinct from background

**Tools**: Use Contrast Checker during design

**Why this matters**:
- ✅ Readable for users with low vision
- ✅ Works in different lighting conditions

---

## Responsive Design

### Primary Target: Desktop (1920x1080 and up)

**Optimized for**:
- Large monitors (27"+)
- Users who need screen space for complex designs

**Layout adjustments**:
- 1920x1080: Default layout (all panels visible)
- 2560x1440: More canvas space, larger metrics
- 3840x2160 (4K): Even more real estate

---

### Secondary Target: Laptop (1366x768 to 1920x1080)

**Optimized for**:
- MacBook Pro 13" (1440x900)
- Common laptop resolutions

**Layout adjustments**:
- Narrower panels (Block Library: 12%, Params: 20%)
- Collapsible panels (hide Block Library when not in use)
- Smaller fonts (13px instead of 14px)

---

### Out of Scope: Mobile (0-768px)

**Why**:
- ❌ Too complex for small screens (can't see full design)
- ❌ Touch interactions not ideal for precise connections
- ❌ Mobile users unlikely to design databases on phone

**Future consideration**: View-only mode (see designs, can't edit)

---

## Error States & Edge Cases

### Error State 1: Invalid Connection

**Scenario**: User tries to connect incompatible ports

**What happens**:
1. Cursor shows red X
2. Tooltip appears: "Cannot connect INDEX_LOOKUP to RECORD_STREAM"
3. Connection is not created

**Visual**:
- Red glow around target port
- Animated shake (brief)
- Error icon with message

**Why this matters**:
- ✅ Prevents invalid designs (fail fast)
- ✅ Teaches user about types

---

### Error State 2: Missing Connections

**Scenario**: User tries to run design with unconnected blocks

**What happens**:
1. Validation runs before execution
2. Error modal appears: "Design is incomplete"
3. Offending blocks highlight in red
4. Message lists issues: "Sequential Scan block has no input"

**Visual**:
```
┌────────────────────────────────────┐
│ ⚠️ Design Cannot Execute           │
├────────────────────────────────────┤
│ The following blocks have issues:  │
│                                    │
│ • Sequential Scan: Missing input   │
│ • Hash Join: Missing right input   │
│                                    │
│ Fix these issues and try again.    │
│                                    │
│ [Show Issues on Canvas]  [Cancel]  │
└────────────────────────────────────┘
```

**Why this matters**:
- ✅ Prevents wasted time (don't run invalid designs)
- ✅ Guides user to fix issues

---

### Error State 3: Execution Failure

**Scenario**: Simulation crashes mid-execution (bug or edge case)

**What happens**:
1. Execution stops
2. Error modal appears: "Simulation failed at operation 1,234"
3. Stack trace (optional, for debugging)
4. Option to report bug

**Visual**:
```
┌────────────────────────────────────┐
│ ❌ Execution Failed                │
├────────────────────────────────────┤
│ The simulation stopped unexpectedly│
│ at operation 1,234.                │
│                                    │
│ This may be a bug. Please report   │
│ it so we can fix it.               │
│                                    │
│ [Report Bug]  [Close]              │
└────────────────────────────────────┘
```

**Why this matters**:
- ✅ Transparent (don't hide errors)
- ✅ Actionable (user can report, we can fix)

---

### Edge Case 1: Empty Design

**Scenario**: Canvas is blank, user clicks Run

**What happens**:
1. Gentle message: "Add blocks to your design first"
2. Highlight Block Library (pulsing animation)

**Why this matters**:
- ✅ Non-intimidating (no harsh error)
- ✅ Guides next action

---

### Edge Case 2: Very Large Designs

**Scenario**: User creates design with 50+ blocks

**What happens**:
1. Canvas performance may degrade (slow pan/zoom)
2. Warning: "Large designs may run slower"
3. Offer to collapse blocks into groups

**Why this matters**:
- ✅ Sets expectations (don't surprise user with slowness)
- ✅ Provides solution (grouping)

---

## Open Design Questions

### Question 1: Auto-Layout vs Manual Layout

**Should blocks auto-arrange themselves, or should users place them manually?**

**Option A: Manual** (user drags to position)
- ✅ Pros: User has full control, intentional design
- ❌ Cons: Takes time, may result in messy layouts

**Option B: Auto** (algorithm arranges blocks)
- ✅ Pros: Fast, clean layouts, less work
- ❌ Cons: User loses control, may not match mental model

**Hybrid approach**:
- Default: Manual (user places)
- Optional: "Auto-arrange" button (cleans up layout)

**Need to decide**: Test both with users

---

### Question 2: Connection Routing (Straight vs Curved)

**Should connections be straight lines or curved (Bezier)?**

**Option A: Straight lines**
- ✅ Pros: Simple, clean, minimalist
- ❌ Cons: May overlap at angles, less aesthetic

**Option B: Curved (Bezier)**
- ✅ Pros: Visually pleasing, flows better
- ❌ Cons: May look cluttered with many connections

**Recommendation**: Curved (React Flow default), with smooth animations

---

### Question 3: Real-Time Metrics vs Post-Execution

**Should metrics update in real-time during execution, or only at the end?**

**Option A: Real-time** (updates every 100ms)
- ✅ Pros: Engaging, shows progress
- ❌ Cons: May be distracting, harder to implement

**Option B: Post-execution** (shows after completion)
- ✅ Pros: Simpler, less distracting
- ❌ Cons: No feedback during long runs

**Hybrid approach**:
- Show progress bar during execution (% complete)
- Show final metrics at end
- Optional: Real-time mode for advanced users

**Recommendation**: Hybrid (progress bar + final metrics)

---

## Design Validation Plan

### Week 1: Wireframes

**Deliverable**: Low-fidelity wireframes (Figma or sketches)

**Test with**: 3 database engineers (informal review)

**Questions**:
- Is the layout clear?
- Can you find the blocks you need?
- Does the flow make sense?

---

### Week 3: Interactive Prototype

**Deliverable**: Clickable prototype (Figma or working code)

**Test with**: 5 database engineers (moderated sessions)

**Tasks**:
1. "Build a design with Heap Storage and B-tree Index"
2. "Run a workload and find the throughput metric"
3. "Compare two designs"

**Measure**:
- Task completion rate
- Time to complete
- Errors / confusion points

---

### Week 7: Alpha Testing

**Deliverable**: Working MVP

**Test with**: 5-10 database engineers (unmoderated)

**Method**: Give them access, ask them to complete a real task

**Measure**:
- How many create ≥2 designs?
- How many complete comparison?
- Qualitative feedback (survey)

---

## Success Criteria (Design Quality)

### Must Have (MVP)

- [ ] User can place and connect blocks without errors
- [ ] User can run workload in <5 seconds (10K ops)
- [ ] Metrics are clearly visible and understandable
- [ ] User can compare two designs side-by-side
- [ ] Error messages are actionable (tell user how to fix)

### Should Have (MVP)

- [ ] Keyboard navigation works for common tasks
- [ ] Responsive for laptop screens (1366x768+)
- [ ] Color contrast meets WCAG AA
- [ ] Loading states show progress (not blank screen)

### Could Have (Post-MVP)

- [ ] Animations for data flow (show records moving)
- [ ] Dark mode
- [ ] Collaborative editing (real-time)
- [ ] Mobile view-only mode

---

## Conclusion

This design is optimized for **speed to decision**. Every choice — from visual canvas to bottom-placed metrics to pre-built templates — serves the goal of enabling database architects to compare design alternatives in 30 minutes instead of 2 weeks.

The design respects user expertise (no hand-holding for basics) while reducing friction (templates, validation, clear errors). It's visual where it matters (architecture, data flow) and textual where it's clearer (parameters, metrics).

Most importantly, it's testable. We can validate every assumption (Do users understand blocks? Can they complete a comparison in 30 min?) through rapid prototyping and user testing.

**Next steps**:
1. Create wireframes (Week 1)
2. Build interactive prototype (Week 3)
3. Alpha test with real users (Week 7)
4. Iterate based on feedback

**Design is never done, but it's ready to start.**

---

**End of Design Document**
