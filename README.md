


<img width="2000" height="491" alt="Social Cover (6)" src="https://github.com/user-attachments/assets/4e256804-53ac-4173-bcff-81994d52bf5c" />



<p align="center">
  <strong>Memvid is a single-file memory layer for AI agents with instant retrieval and long-term memory.</strong><br/>
  Persistent, versioned, and portable memory, without databases.
</p>

<p align="center">
  <a href="https://www.memvid.com">Website</a>
  ·
  <a href="https://docs.memvid.com">Docs</a>
  ·
  <a href="https://github.com/memvid/memvid/discussions">Discussions</a>
</p>

<p align="center">
  <img src="https://img.shields.io/github/stars/memvid/memvid?style=flat-square" />
  <img src="https://img.shields.io/github/issues/memvid/memvid?style=flat-square" />
  <img src="https://img.shields.io/badge/status-v2%20in%20progress-blue?style=flat-square" />
</p>

<h2 align="center">⭐️ Leave a star to support the project ⭐️</h2>


## 🚀 Memvid v2 Launching On: **January 5, 2026**

> **Note**  
> Memvid v1 has been removed to avoid confusion. This repository represents **Memvid v2**, a revised and improved version of the project.
> Thanks to everyone who provided feedback on V1. The comments and issues helped shape the improvements included in V2.

---

## What is Memvid?

Memvid is a portable AI memory system that packages your data, embeddings, search structure, and metadata into a single file. 

Instead of running complex RAG pipelines or server-based vector databases, Memvid enables fast retrieval directly from the file. 

The result is a model-agnostic, infrastructure-free memory layer that gives AI agents persistent, long-term memory they can carry anywhere.

---

## Why Video Frames?

Memvid draws inspiration from video encoding, not to store video, but to **organize AI memory as an append-only, ultra-efficient sequence of Smart Frames.**

A Smart Frame is an immutable unit that stores content along with timestamps, checksums and basic metadata.
Frames are grouped in a way that allows efficient compression, indexing, and parallel reads.

This frame-based design enables:

- Append-only writes without modifying or corrupting existing data
- Queries over past memory states
- Timeline-style inspection of how knowledge evolves
- Crash safety through committed, immutable frames
- Efficient compression using techniques adapted from video encoding

The result is a single file that behaves like a rewindable memory timeline for AI systems.

---

## Core Concepts (v2)

- **Living Memory Engine**  
  Continuously append, branch, and evolve memory across sessions.

- **Capsule Context (`.mv2`)**  
  Self-contained, shareable memory capsules with rules and expiry.

- **Time-Travel Debugging**  
  Rewind, replay, or branch any memory state.

- **Smart Recall**  
  Sub-5ms local memory access with predictive caching.

- **Codec Intelligence**  
  Auto-selects and upgrades compression over time.

---

## Use Cases
Memvid is a portable, serverless memory layer that gives AI agents persistent memory and fast recall. Because it’s model-agnostic, multi-modal, and works fully offline, developers are using Memvid across a wide range of real-world applications.

- Long-Running AI Agents
- Enterprise Knowledge Bases
- Offline-First AI Systems
- Codebase Understanding
- Customer Support Agents
- Workflow Automation
- Sales and Marketing Copilots
- Personal Knowledge Assistants
- Medical, Legal, and Financial Agents
- Auditable and Debuggable AI Workflows
- Custom Applications 

---

## Status

- Core architecture finalized  
- APIs stabilized
- Docs and SDKs coming soon
  
**Official v2 public release: January 5, 2026**

---

## License

Apache License 2.0 — see the [LICENSE](LICENSE) file for details.
