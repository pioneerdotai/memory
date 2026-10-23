# Memvid search pagination — external review handoff

Worktree: `/Users/alexander/Code/pioneer/memory-search-pagination`.
Branch: `feat/search-pagination`.
Base: `fe0b3d9ac883c667587c262f7b372bf06eb605d5` (the pinned 3.1.8, not the older fork main).
Final revision: the commit containing this document (`git rev-parse feat/search-pagination`).
This revision is local only; no push, merge, tag, release, deploy, or CI was requested or performed.

## Existing API and Pioneer integration contract

No new public methods were introduced. `SearchRequest` now has an optional
`query_embedding: Option<Vec<f32>>` (serde defaults to `None`). All Rust struct
literals need this field, including Pioneer callers.

- `mem.search(request)` with `query_embedding: None` uses the existing lexical path.
- With `query_embedding: Some(prepared_vector)` it retrieves lexical and vector
  prefixes, applies source filters/ACL, fuses with the same RRF helper used by
  `ask`, and slices the requested combined page. It performs no embedding calls.
- Prepare the vector once outside per-file iteration, validate provider/model
  compatibility in Pioneer as before, and clone this small query vector into each
  request. Memvid still checks the stored dimension.
- Set `no_sketch: true` for complete lexical corpus coverage. The pre-existing
  optional sketch accelerator still limits candidate discovery when enabled.
- `SearchRequest.cursor` and `SearchResponse.next_cursor` are decimal offsets.
  Keep query, embedding, source filters, ACL context, and page size consistent.
  The cursor grants no access; checks run again on each independent call.
- `next_cursor: None` marks exhaustion. `total_hits` is the number of accepted
  candidates observed in the fetched prefix, not an exact whole-corpus count.
  Use the cursor rather than `total_hits` to determine completion.

Example using an existing request assembled by the caller:

```rust
request.top_k = 100;
request.cursor = None;
request.no_sketch = true;
request.query_embedding = prepared_embedding.clone();
let first = mem.search(request.clone())?;
request.cursor = first.next_cursor;
let second = mem.search(request)?;
```

The existing `vec_search_with_embedding_acl` now has a final argument
`cursor: Option<&str>`; pass `None` initially and `response.next_cursor.as_deref()`
for continuation. Its old arguments retain their meaning. The simple
`vec_search_with_embedding` wrapper keeps its signature and requests the first
page. Internal adaptive callers were updated, including returned-count stats.

```rust
let first = mem.vec_search_with_embedding_acl(
    query, &prepared, 100, 240, scope, acl, enforcement, None,
)?;
let second = mem.vec_search_with_embedding_acl(
    query, &prepared, 100, 240, scope, acl, enforcement,
    first.next_cursor.as_deref(),
)?;
```

`ask` retains its answer-oriented heuristics, including analytical timeline
retrieval. `context_only` still means synthesis is disabled; it is not a switch
to raw retrieval. Pioneer ordinary search must use `search`, not `ask`.
`AskRequest.cursor` now slices after combined ranking and ACL, before synthesis
and context-fragment construction. It is no longer passed into just the lexical
component. The vector 64-candidate ceiling and analytical timeline 100-entry
ceiling were removed; the requested prefix determines retrieval width.

## Requirement map

- A: `src/vec.rs`: zero/empty exits; output buffer bounded by graph size and
  requested limit; search width at least the output size (minimum 50, capped by
  graph size); only the returned neighbor slice is converted to hits. No panic
  catching or result ceiling of 50.
- B: `src/memvid/search/api.rs`: growing vector candidate prefixes, active-frame
  and scope checks, request URI/replay/temporal filters when supplied, ACL before
  deciding whether a page is full, positional cursor, and continuation.
- C: `src/memvid/search/tantivy.rs`: grow the document window until accepted
  snippets cover the requested offset/page plus lookahead, or the engine is
  exhausted. `fallback.rs` also includes offset plus lookahead in per-document
  snippet limits. `acl.rs` and `search/mod.rs` intersect ACL-allowed frames with
  existing filters before lexical pagination; sketch intersection cannot undo
  an existing source or ACL restriction.
- D: `src/memvid/ask.rs`: combined offset pages, unrestricted request-driven
  vector width, deterministic frame-ID tie-breaking in shared RRF, and page
  slicing before answer/citation/context construction.
- E: `SearchRequest.query_embedding` routes existing `search` through raw hybrid
  retrieval. No analytical timeline heuristic is applied. Shared internal
  helpers reuse existing lexical search, vector conversion/filtering, and RRF.

No Pioneer concepts, file format changes, sidecars, new index, registry, global
cache, or immutable search-session infrastructure were added. No model calls or
vector export occur in the new search orchestration.

## Semantics and limits

Pages are positional. ANN ranking can change with larger prefixes; overlap or
omissions caused by that approximation are allowed. There is no pinned generation
or saved full result set. Deeper pages and extensive rejections cost more internal
work. Candidate windows grow by two rather than imposing a fixed corpus ceiling.
The minimum internal starting prefix is 32 for vector/hybrid and 20 documents for
Tantivy; these are service quanta, not maximum searched counts. Vector snippets
retain the existing minimum of 80 characters. Existing ask correction retrieval
still uses its separate 10-correction heuristic; ordinary `search` does not use it.
There is no new query-length limit or page-size maximum in Memvid.
Pioneer owns its 100-fragment/2 MiB output policy and current-source authorization.

## Written regressions (NOT RUN)

- `src/vec.rs::tests::hnsw_large_requests_and_empty_indexes_do_not_panic`:
  1,100 vectors; limits 0, 100, 200, 1,200; unique valid IDs; empty/single indexes.
- `tests/search_pagination.rs::lexical_pages_reach_matches_after_rejected_document_windows`:
  120 stronger out-of-scope candidates followed by 13 matches; pages 5/5/3.
- `analytical_wording_does_not_replace_search_with_timeline`: prepared-vector
  hybrid retrieval for ordinary and analytical wording selects the relevant older
  source over an unrelated newer source.
- `hybrid_pages_fill_after_vector_scope_and_acl_rejections`: 1,320 records;
  550 scope rejections, 550 ACL rejections, 220 permitted records; both the public
  vector API and hybrid `search` return pages 100/100/20. No assertion requires
  perfectly stable ANN page boundaries.
- `ask_hybrid_continuation_pages_the_combined_order`: 180 sources, pages 100/80,
  matching context-fragment page sizes and final exhaustion.
- `lexical_pages_apply_acl_before_counting_hits`: 100 forbidden candidates before
  three permitted sources; pages 2/1.
- `lexical_continuation_reaches_later_snippets_of_one_document`: eight separated
  matches in one source remain reachable through four pages of two.

Existing examples, benchmarks, unit/integration fixtures, replay construction,
and graph-search callers received only the required `query_embedding: None`
field where applicable. They were not executed.

## Static audit and commands

Read `CLAUDE.md` and `CONTRIBUTING.md`; `rg --files -g AGENTS.md .` found no
repository-local AGENTS.md. User prohibitions override their build/test guidance.
Read the installed `hnsw-0.11.0` source (not modified), including `nearest`,
`search_layer`, and search-width handling. Reviewed every changed file and both
public vector callers and internal adaptive/ask callers. Reviewed optional
feature gates and source/ACL filters before pagination. Fixed a cursor-assignment
mistake discovered during static diff review before committing.

State/read-only commands used included:

```text
pwd
git status --short --branch
git rev-parse HEAD
git branch --show-current
git diff --stat
git diff --numstat
git diff -- <reviewed source, examples, benches, and test paths>
rg --files -g AGENTS.md .
cat CLAUDE.md CONTRIBUTING.md
rg -n <symbols/callers> src tests examples benches
sed -n <line ranges> <reviewed source and installed hnsw paths>
```

Edits used `python3` text-edit scripts and shell heredocs; these did not import or
execute project code. Exact formatting/whitespace commands executed:

```text
cargo fmt
cargo +1.98.1 fmt
cargo +1.98.1 fmt --check
git diff --check
```

The first `cargo fmt` triggered rustup toolchain synchronization/download for the
repository's pinned 1.90.0 and was interrupted with Ctrl-C. Formatting was then
performed with the already installed 1.98.1 formatter. `cargo +1.98.1 fmt --check`
and `git diff --check` completed successfully. No build or project execution
occurred. The commit is created with hooks disabled:

```text
git add <all reviewed task paths>
git -c core.hooksPath=/dev/null commit -m "fix(search): paginate filtered lexical vector and hybrid retrieval"
```

Тесты, cargo check, компиляция, сборки, приложение, CLI-агенты и генераторы не запускались. Компилируемость и прохождение тестов не подтверждены.

This is a local implementation candidate for complete external review, not an
acceptance claim. The new Memvid revision is not published on GitHub; a separate
authorized publication is required before an external Git consumer can fetch it.
