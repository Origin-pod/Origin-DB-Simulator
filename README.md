# DB Simulator

> Because reading about databases is boring. Building them is fun.

## The Origin Story

You know that feeling when you're reading *Designing Data-Intensive Applications* or *Database Internals* and your eyes start glazing over at page 47?

Yeah, me too.

Books about B-trees, LSM trees, MVCC, and WAL are dense. The concepts are fascinating, but reading 800 pages of text about how databases work is... well, let's just say it's not the most exciting Saturday night.

Then I had a thought: **What if instead of just reading about databases, I could build one? And not just one - what if I could build dozens, trying different combinations, seeing the tradeoffs play out in real-time?**

What if learning database internals felt like playing with LEGO?

That's this project.

## What is this?

**DB Simulator** is a visual tool where you take database building blocks and compose them together to create your own database. Then you can run queries, watch how it executes, and see the tradeoffs in action.

Think:
- **Unreal Engine Blueprints**, but you're learning databases
- **Scratch programming**, but the output is a working storage engine
- **Figma**, but you're designing MVCC instead of UI

## Why This Exists

Reading about databases teaches you *what* things do.

Building databases teaches you *why* they do it that way.

Want to understand why Cassandra uses LSM trees instead of B-trees? Build both and run the same workload. Watch the LSM tree handle writes faster but reads slower. See the write amplification. Feel the tradeoff.

Want to grok the difference between 2PL and MVCC? Build both. Watch how 2PL blocks readers, but MVCC lets them run free. See why PostgreSQL chose MVCC.

**Learning by doing beats reading every time.**

## What's Inside

This project provides modular blocks across the entire database stack:

```
src/blocks/
├── storage/           # LSM trees, B-trees, Log-structured storage
├── index/             # Hash indexes, B+ trees, Learned indexes
├── concurrency/       # 2PL, MVCC, OCC, timestamp ordering
├── transaction-recovery/ # WAL, ARIES, shadow paging
├── query-execution/   # Volcano, vectorized, JIT compilation
├── optimization/      # Rule-based, cost-based, learned optimizers
├── buffer/            # LRU, Clock, 2Q policies
├── compression/       # Dictionary, RLE, Snappy, Zstd
├── partitioning/      # Range, hash, list partitioning
└── distribution/      # Replication, sharding, consensus
```

## How It Works

### The Old Way (Reading)
1. Read chapter about B-trees: **1 hour**
2. Understand roughly how they work: **Maybe?**
3. Read chapter about LSM trees: **1 hour**
4. Try to remember what B-trees were: **???**
5. Read about when to use each: **30 minutes**
6. Still not sure which is better for your use case: **Forever**

### The New Way (Building)
1. Drag a B-tree block onto canvas: **10 seconds**
2. Add a write-heavy workload: **20 seconds**
3. Hit run and watch: **10 seconds**
4. Now try an LSM tree with same workload: **20 seconds**
5. See the difference visually: **Immediately**
6. Actually understand the tradeoff: **Finally!**

## Quick Start

```bash
# Clone the repo
git clone https://github.com/yourname/DB-Simulator.git
cd DB-Simulator

# Explore the block system
ls src/blocks/

# Read the vision
cat docs/Modular\ DB\ Builder\ -\ PRD\ \(Shreyas\ Style\).md
```

## Who Is This For?

### You'll love this if you:
- Are reading DDIA or Database Internals and want to *actually* understand it
- Learn better by doing than by reading
- Want to build a custom DB for a unique use case but don't know where to start
- Think databases are interesting but find the textbooks dry
- Want to understand why databases make the choices they do
- Wish you could experiment with "what if MongoDB did X instead of Y?"
- Just think databases are neat and want to play with them

### You'll hate this if you:
- Love reading 800-page technical books cover-to-cover
- Think building things is a waste of time
- Believe the only way to learn is through pure theory
- Already understand everything about databases

## Project Status

**Current Stage**: Early prototype / Design phase

We're building the foundation. Check the roadmap:
- `/docs/8-Week Roadmap - Modular DB Builder.md` - Implementation timeline
- `/docs/Design Document - Modular DB Builder.md` - Technical architecture
- `/docs/Wireframes - Modular DB Builder.md` - UI mockups

## The Dream

Can we make learning database internals as fun as using them?

If this works, people won't just read about databases - they'll build them. They'll understand the *why* behind every design decision. They'll stop cargo-culting PostgreSQL and start designing databases that fit their actual use cases.

And maybe, just maybe, someone will finally understand what "everything is a log" actually means by building it themselves.

**The best way to understand how something works is to build it yourself. Let's make that actually enjoyable.**

## Contributing

This is an early-stage project born from frustration with boring textbooks and a desire to learn by building.

If you're excited about:
- Making learning more hands-on and visual
- Database internals (or want to understand them better)
- Building tools that make learning fun
- The idea that understanding comes from experimentation

...then you're in the right place. Check `/docs` for the full vision and roadmap.

**This project is as much about the journey of learning as it is about the destination.**

## License

TBD

---

*"What I cannot create, I do not understand." - Richard Feynman*
