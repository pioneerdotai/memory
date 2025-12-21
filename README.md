


<img width="2000" height="491" alt="Social Cover (6)" src="https://github.com/user-attachments/assets/4e256804-53ac-4173-bcff-81994d52bf5c" />



<p align="center">
  <strong>Memvid is a video-native memory engine for AI.</strong><br/>
  Persistent, versioned, and portable memory — without databases.
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

<h2 align="center">⭐️ Star this repository to support Memvid ⭐️</h2>

## 🚀 Memvid v2 launches **January 5, 2026**

> **Note**  
> Memvid v1 has been removed to avoid confusion.  
> This repository represents **Memvid v2**, a revised and improved version of the project.
> Thanks to everyone who provided feedback on V1. The comments and issues helped shape the improvements included in V2.

---

## What is Memvid?

Memvid turns knowledge into **living memory capsules** using video compression.

Instead of storing context in vector databases, Memvid encodes memory into
**video-native formats** that are compact, fast to recall, offline-first,
and future-proof through modern codecs.

Think of Memvid as **long-term memory infrastructure for LLMs**.

---

## Why Video Frames?

Memvid borrows ideas from video encoding — not to store video, but to store
**memory as an append-only sequence of frames**.

Each frame contains content plus metadata, timestamps, and checksums. Frames
are grouped into segments for efficient compression, indexing, and parallel
access.

This frame-based design enables:

- Append-only writes that never corrupt existing data  
- Time-travel queries over historical memory states  
- Timeline-style browsing of knowledge evolution  
- Crash safety via committed, immutable frames  
- Efficient compression using proven video techniques  

The result is a **single `.mv2` file** that behaves like a rewindable memory
timeline for AI systems. 

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

- Long-running AI agents  
- Knowledge assistants  
- Offline-first AI systems  
- Auditable and debuggable AI workflows  

---

## Status

- Architecture locked  
- APIs stabilizing  
- Docs and SDKs coming soon  
- **Public v2 release: January 5, 2026**

---

## License

Apache License 2.0 — see the [LICENSE](LICENSE) file for details.
