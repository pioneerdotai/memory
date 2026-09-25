# Changelog

All notable changes to Memvid will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial public release of Memvid core library
- Single-file `.mv2` format for portable AI memory
- Full-text search with BM25 ranking (Tantivy)
- Vector similarity search with HNSW
- PDF, DOCX, XLSX document ingestion
- CLIP visual embeddings for image search
- Whisper audio transcription
- Timeline queries for chronological browsing
- Crash-safe WAL-based writes
- Blake3 checksums for data integrity
- Ed25519 signatures for authenticity
- Optional AES-256-GCM encryption

### Security
- Embedded WAL prevents data corruption
- Atomic commits ensure consistency
- File locking prevents concurrent write conflicts

## [3.1.6] - 2026-09-25

### Fixed
- Replay pending WAL records on an atomic staging copy, preserving the committed capsule and pending WAL if index rebuilding fails or the recovery process exits before publication.
- Apply the same publication protection to recovery-time lexical index flushes without pending frame records, retaining writer-lock handoff and post-publication error handling.

### Compatibility
- The public API and `.mv2` format are unchanged. This prevents destructive in-place WAL recovery; it does not reconstruct TOCs already lost from damaged capsules.

## [3.1.5] - 2026-09-21

### Fixed
- Keep writer locks attached to the published capsule across atomic replacement, reopen stale waiting writers on the current inode, and prevent further writes after failed publication or loss of snapshot continuity.
- Preserve lock ownership during shared/exclusive transitions and protect manifest WAL and replay session cleanup from stale handles.
- Bound TOC recovery hashing and decoding across footer, hint, and fallback searches, eliminating quadratic suffix hashing while preserving checksum verification and current/legacy TOC support.
- Recover segmented TOCs, inaccurate header hints, missing or damaged footers, and supported large TOCs without exhausting the decode allowance on false hash candidates.

### Compatibility
- The public API and `.mv2` format are unchanged. Blind TOC scanning remains limited to the last 64 MiB; crafted false candidates can exhaust the bounded recovery budget and return an error without modifying the file.

## [3.1.4] - 2026-09-20

### Fixed
- Reuse and truncate the derived-index tail inside atomic commits so repeated vector and sketch snapshots no longer accumulate in `.mv2` files, including sketch-only and parallel commits.
- Preserve segment-backed, pending, HNSW, and PQ vectors during rebuilds, including pending embedding replacements, and reject malformed HNSW/PQ structures before destructive writes.
- Preserve opaque replay ranges in writers built without the replay feature and keep replay offsets consistent when the WAL grows.
- Preserve sketch data and clear empty sketch manifests across commit, recovery, vacuum, and atomic index finalization.

### Compatibility
- The public API and `.mv2` format are unchanged. The next successful modifying commit reclaims obsolete derived snapshots from ordinary bloated files; an unchanged commit remains a no-op. Preserved replay ranges can retain earlier gaps that require separate defragmentation.

## [3.1.3] - 2026-09-20

### Fixed
- Disabled Tantivy's redundant automatic reader reload because all commit paths already reload synchronously, preventing macOS `.tantivy-meta.lock` races after commits.

## [3.1.2] - 2026-07-08

### Changed
- Disabled the `tokenizers` `esaxx_fast` default path for vector builds so Windows MSVC consumers avoid mixed CRT link failures; projects that need the C++ suffix-array trainer can opt in with `tokenizers_esaxx_fast`.

## [2.0.0] - 2026-01-05

### Added
- Complete rewrite in Rust for performance and safety
- New `.mv2` file format (single-file, no sidecars)
- Append-only frame-based architecture
- Built-in full-text and vector search
- Cross-platform support (macOS, Linux, Windows)

### Changed
- Migrated from Python to Rust
- New API design focused on simplicity
- Improved memory efficiency

### Removed
- Legacy Python implementation
- QR code video encoding (replaced with efficient binary format)

---

[Unreleased]: https://github.com/pioneerdotai/memory/compare/v3.1.5...HEAD
[3.1.5]: https://github.com/pioneerdotai/memory/compare/v3.1.4...v3.1.5
[3.1.4]: https://github.com/pioneerdotai/memory/compare/v3.1.3...v3.1.4
[3.1.3]: https://github.com/memvid/memvid/compare/v3.1.2...v3.1.3
[3.1.2]: https://github.com/memvid/memvid/compare/v3.1.1...v3.1.2
[2.0.0]: https://github.com/memvid/memvid/releases/tag/v2.0.0
