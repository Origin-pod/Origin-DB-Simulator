# Block System Architecture

**Last Updated**: 2026-01-20

## Overview

The Block System is the foundational component of the Modular DB Builder. Each block represents a self-contained, reusable database component that can be composed with other blocks to build custom database architectures.

## Core Concepts

### Block Definition

A **block** is a self-contained, reusable component that implements a specific database function.

```typescript
interface Block {
  // Identity
  id: string;
  name: string;
  category: BlockCategory;
  description: string;

  // Interface
  inputs: Port[];      // What data/signals it receives
  outputs: Port[];     // What it produces
  parameters: Parameter[];  // Configurable settings

  // Behavior
  implementation: BlockImplementation;

  // Constraints
  requires: Constraint[];   // Dependencies (e.g., "needs WAL")
  guarantees: Guarantee[];  // What it provides (e.g., "ACID compliance")

  // Metadata
  metrics: Metric[];        // What it measures (IO, latency)
  documentation: string;    // How it works
  references: string[];     // Papers, implementations
}
```

## Block Categories

The Block System is organized into 10 main categories:

1. **Storage Engine Blocks** (`src/blocks/storage/`)
2. **Index Structure Blocks** (`src/blocks/index/`)
3. **Concurrency Control Blocks** (`src/blocks/concurrency/`)
4. **Buffer Management Blocks** (`src/blocks/buffer/`)
5. **Query Execution Blocks** (`src/blocks/query-execution/`)
6. **Transaction & Recovery Blocks** (`src/blocks/transaction-recovery/`)
7. **Compression Blocks** (`src/blocks/compression/`)
8. **Partitioning Blocks** (`src/blocks/partitioning/`)
9. **Optimization Blocks** (`src/blocks/optimization/`)
10. **Distribution Blocks** (`src/blocks/distribution/`)

## Design Principles

### 1. Composability
- Blocks are **independent** — no tight coupling
- Clear **interfaces** — well-defined inputs/outputs
- **Mix and match** — any valid combination should work

### 2. Reusability
- Blocks are **parameterized** — one block, many configurations
- **Domain-agnostic** — works for OLTP, OLAP, streaming, etc.
- **Shareable** — export/import block libraries

### 3. Observability
- Every block is **instrumented** — built-in metrics
- **Visual feedback** — see data flowing in real-time
- **Debuggable** — step through execution, inspect state

### 4. Extensibility
- **Open architecture** — users can add new blocks
- **Plugin system** — custom blocks without modifying core
- **Community-driven** — share blocks, fork designs

## Implementation Structure

```
src/blocks/
├── core/                    # Core block system infrastructure
│   ├── types/              # TypeScript interfaces and types
│   ├── registry/           # Block registry and discovery
│   ├── validation/         # Connection and constraint validation
│   └── execution/          # Block execution engine
│
├── storage/                # Storage Engine Blocks
├── index/                  # Index Structure Blocks
├── concurrency/            # Concurrency Control Blocks
├── buffer/                 # Buffer Management Blocks
├── query-execution/        # Query Execution Blocks
├── transaction-recovery/   # Transaction & Recovery Blocks
├── compression/            # Compression Blocks
├── partitioning/           # Partitioning Blocks
├── optimization/           # Optimization Blocks
└── distribution/           # Distribution Blocks
```

## Next Steps

1. Define core TypeScript interfaces (`core/types/`)
2. Implement block registry system (`core/registry/`)
3. Build validation system (`core/validation/`)
4. Create execution engine (`core/execution/`)
5. Implement first MVP blocks (Storage, Index, Buffer)
