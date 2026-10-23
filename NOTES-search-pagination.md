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

- A: `src/vec.rs`: zero/empty exits; ANN graph traversal with an empty output
  destination (safe even for a short actual neighbor pool); collect metric
  evaluations through Memvid's own metric and rank only visited candidates.
  Search width is max(50, requested), capped by graph size. No panic catching,
  result ceiling of 50, or full-corpus distance scan.
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
retain the existing minimum of 80 characters. Ask correction retrieval now grows
with the shared prefix, so its former 10-correction window cannot hide later
eligible corrections. There is no new query-length limit or page-size maximum
in Memvid.
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


## Revision round 1 — postfilter completion and short HNSW pools

Previous candidate commit: `da735d06816032c718df419fedd9fe4c23ada633`.
The public API contract above is unchanged by this revision.

### Corrected page completion

`ask_unpaged` now retains engine continuation flags before time/ACL postfilters
and RRF source deduplication. If fewer than offset + page size + one eligible
sources remain while any participating engine still has candidates, it doubles
the prefix and repeats retrieval internally. Query embedding is computed before
this loop and reused. Synthesis/citations/context are still generated only for
the final requested page. The retry block wraps existing answer-oriented query
heuristics; raw `search` remains separate and does not acquire those heuristics.

Vector recall uses the existing shared `vec_search_page` primitive and carries
its continuation flag. Optional adaptive ranking still applies its configured
score cutoff, but its retrieval width grows with the requested prefix and the
underlying engine continuation is preserved. Corrections use the growing prefix.
Timeline fallback signals a full service window so it can grow too; a full window
is not asserted to be the true end. Final time checks also cover correction and
timeline candidates before page completion is decided.

### HNSW dependency contract

Source inspection confirmed a healthy graph can have fewer reachable neighbors
than either ef_search or graph.len(), notably when equal distances prevent reverse
edges from replacing full neighbor lists. Merely clamping the destination to
min(requested, graph.len()) was therefore insufficient.

The new adapter calls the public `nearest` method with an empty destination. In
hnsw 0.11 `found = min(dest.len(), searcher.nearest.len())` is then always zero,
so `dest.copy_from_slice(&nearest[..found])` has matching lengths for every actual
pool size. Its graph traversal is independent of destination length. Memvid's
existing Euclidean metric observes the candidate references/distances evaluated
by that traversal; it does not inspect Searcher's private fields. The result is
the best requested number of actually visited candidates, potentially fewer
when the reachable pool is short. ANN order may differ from the former pool-only
selection, which is within the allowed approximate semantics.

A per-index lazy scalar map associates addresses of publicly borrowed
`graph.feature(i)` references with existing frame IDs. It never dereferences
stored addresses and assumes no private object layout, pointer offset, debug
format, or serialization layout. The graph is immutable after construction.
Cloning resets this map; deserialization defaults it empty. The field is
`serde(skip)`, leaving the stored three-field HNSW representation unchanged.
The first query creates O(number of graph features) scalar identity entries;
queries do not export or clone all vectors or compute full-corpus distances.
Only ANN's actual metric evaluations are ranked. Query observation is thread-local
scratch, cleared by a Drop guard, not a query/vector cache.

### Additional regressions (NOT RUN)

- `src/memvid/search/tantivy.rs::tests::postfilter_rejections_do_not_end_the_initial_document_window`:
  120 strongly ranked Tantivy documents remain indexed while authoritative
  frame.search_text is changed to unrelated text. No scope/filter pushdown is
  used. Thirteen later eligible sources remain reachable through pages 5/5/3,
  with engine=Tantivy and exact eligible frame IDs.
- `tests/search_pagination.rs::ask_pages_grow_past_duplicate_snippets_before_later_sources`:
  multiple initial snippets collapse into one source; later sources still fill
  two pages of two, with no premature EOF.
- `ask_pages_grow_past_timestamp_postfilters_before_later_sources`:
  one early eligible source, forty timestamp-rejected candidates, then three
  later eligible sources; pages 2/2. This regression is gated to no temporal_track,
  where timestamp filtering is necessarily after lexical retrieval. The duplicate
  snippet regression covers all lex feature configurations.
- `src/vec.rs::tests::hnsw_short_neighbor_pool_and_cloned_decoded_indexes_are_safe`:
  1100 identical embeddings inserted through the real HNSW builder produce a
  short reachable pool; requests 1100 and 1200 return its actual valid neighbors
  without panic. Cloned and decoded indexes remain searchable. Serialized bytes
  after populating the transient map equal the legacy three-field representation.

### Revision audit and exact validation commands

Re-read the full substantive diff using `git diff -w -- src/memvid/ask.rs` and
all four changed code/test paths. Inspected hnsw 0.11 nearest/search_layer,
search_zero_layer/search_single_layer, initialize_searcher, add_neighbor,
and public feature lookup contracts from the installed source without editing
it. Checked vector_hits callers, all engine-continuation capture points, timestamp
postfilters, adaptive handling, clone/serde behavior, and optional feature gates.
The former scope integration test remains coverage for scope pagination; the new
authoritative-text unit test specifically owns post-retrieval window rejection.

Revision validation commands:

```text
cargo +1.98.1 fmt
cargo +1.98.1 fmt --check
git diff --check
git diff -w -- src/memvid/ask.rs
git diff -- src/vec.rs src/memvid/search/tantivy.rs tests/search_pagination.rs NOTES-search-pagination.md
git add src/vec.rs src/memvid/ask.rs src/memvid/search/tantivy.rs tests/search_pagination.rs NOTES-search-pagination.md
git -c core.hooksPath=/dev/null commit -m "fix(search): fill ask pages after postfilters and safely collect ANN candidates"
git diff HEAD^ HEAD --check
git status --short --branch
git rev-parse HEAD
```

All test cases remain unexecuted. No compiler, build, project code, app, agent,
example, benchmark, schema generator, hook, CI, or publication was run.
Компилируемость и прохождение тестов не подтверждены.
