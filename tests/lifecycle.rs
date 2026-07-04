//! Integration tests for Memvid lifecycle operations.
//! Tests: create, open, open_read_only, commit, stats, verify

use memvid_core::{Memvid, PutOptions, VerificationStatus};
use std::fs;
use tempfile::TempDir;

#[cfg(feature = "lex")]
fn search_uris(mem: &mut Memvid, query: &str, top_k: usize) -> Vec<String> {
    mem.search(memvid_core::SearchRequest {
        query: query.to_string(),
        top_k,
        snippet_chars: 160,
        uri: None,
        scope: None,
        cursor: None,
        #[cfg(feature = "temporal_track")]
        temporal: None,
        as_of_frame: None,
        as_of_ts: None,
        no_sketch: true,
        acl_context: None,
        acl_enforcement_mode: memvid_core::AclEnforcementMode::Audit,
    })
    .unwrap()
    .hits
    .into_iter()
    .map(|hit| hit.uri)
    .collect()
}

#[cfg(feature = "lex")]
fn put_search_doc(mem: &mut Memvid, uri: &str, text: &str) {
    let opts = PutOptions {
        uri: Some(uri.to_string()),
        search_text: Some(text.to_string()),
        ..Default::default()
    };
    mem.put_bytes_with_options(text.as_bytes(), opts).unwrap();
}

/// Test basic create and open lifecycle.
#[test]
fn create_and_open() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.mv2");

    // Create new memory
    {
        let mut mem = Memvid::create(&path).unwrap();
        mem.commit().unwrap();
    }

    // Open existing memory
    {
        let _mem = Memvid::open(&path).unwrap();
    }

    // Open read-only
    {
        let _mem = Memvid::open_read_only(&path).unwrap();
    }

    assert!(path.exists(), "MV2 file should exist");
}

/// Test that create handles existing file.
/// Note: The current implementation allows creating even if file exists,
/// this tests that behavior (may change in future versions).
#[test]
fn create_handles_existing_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.mv2");

    // Create first time
    {
        let mut mem = Memvid::create(&path).unwrap();
        let opts = PutOptions {
            uri: Some("mv2://doc1".to_string()),
            ..Default::default()
        };
        mem.put_bytes_with_options(b"First content", opts).unwrap();
        mem.commit().unwrap();
    }

    // Create second time - this tests current behavior
    // (Either it fails OR it creates a new file - both are valid implementations)
    let result = Memvid::create(&path);
    if let Ok(mut mem) = result {
        // If create succeeds, the old data should be gone (new file)
        // let mut mem = result.unwrap();
        mem.commit().unwrap();
        // Reopen and verify it's empty (new file was created)
        let mem = Memvid::open_read_only(&path).unwrap();
        let stats = mem.stats().unwrap();
        assert_eq!(stats.frame_count, 0, "New file should be empty");
    }
    // If it fails, that's also valid behavior
}

/// Test that open fails if file doesn't exist.
#[test]
fn open_fails_if_not_exists() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("nonexistent.mv2");

    let result = Memvid::open(&path);
    assert!(result.is_err(), "Open should fail if file doesn't exist");
}

/// Test stats on empty memory.
#[test]
fn stats_empty_memory() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.mv2");

    {
        let mut mem = Memvid::create(&path).unwrap();
        mem.commit().unwrap();
    }

    let mem = Memvid::open_read_only(&path).unwrap();
    let stats = mem.stats().unwrap();

    assert_eq!(stats.frame_count, 0, "Empty memory should have 0 frames");
}

/// Test stats after adding content.
#[test]
fn stats_with_content() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.mv2");

    {
        let mut mem = Memvid::create(&path).unwrap();

        for i in 0..5 {
            let content = format!("Test content {}", i);
            let opts = PutOptions {
                uri: Some(format!("mv2://doc{}", i)),
                title: Some(format!("Document {}", i)),
                ..Default::default()
            };
            mem.put_bytes_with_options(content.as_bytes(), opts)
                .unwrap();
        }

        mem.commit().unwrap();
    }

    let mem = Memvid::open_read_only(&path).unwrap();
    let stats = mem.stats().unwrap();

    assert_eq!(stats.frame_count, 5, "Should have 5 frames");
}

/// Test verify on healthy file.
#[test]
fn verify_healthy_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.mv2");

    {
        let mut mem = Memvid::create(&path).unwrap();

        let opts = PutOptions {
            uri: Some("mv2://test".to_string()),
            title: Some("Test".to_string()),
            ..Default::default()
        };
        mem.put_bytes_with_options(b"Test content", opts).unwrap();
        mem.commit().unwrap();
    }

    let report = Memvid::verify(&path, false).unwrap();

    assert_eq!(
        report.overall_status,
        VerificationStatus::Passed,
        "Healthy file should verify as passed"
    );
}

/// Test verify detects corruption (footer zeroed).
/// Note: With severe corruption, verify may return an error instead of a report.
#[test]
fn verify_detects_corruption() {
    use std::io::{Seek, SeekFrom, Write};

    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.mv2");

    // Create valid file
    {
        let mut mem = Memvid::create(&path).unwrap();
        let opts = PutOptions {
            uri: Some("mv2://test".to_string()),
            ..Default::default()
        };
        mem.put_bytes_with_options(b"Test content", opts).unwrap();
        mem.commit().unwrap();
    }

    // Corrupt the footer (zero last 16 bytes)
    {
        let mut file = fs::OpenOptions::new().write(true).open(&path).unwrap();
        let len = file.metadata().unwrap().len();
        if len > 16 {
            file.seek(SeekFrom::End(-16)).unwrap();
            file.write_all(&[0u8; 16]).unwrap();
            file.flush().unwrap();
        }
    }

    // With severe corruption, verify may error out entirely
    // or return a failed status - both are valid responses
    let result = Memvid::verify(&path, false);
    match result {
        Ok(report) => {
            assert_ne!(
                report.overall_status,
                VerificationStatus::Passed,
                "Corrupted file should not verify as passed"
            );
        }
        Err(_) => {
            // Error is also a valid response to severe corruption
        }
    }
}

/// Test multiple commits preserve data.
#[test]
fn multiple_commits_preserve_data() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.mv2");

    // First commit
    {
        let mut mem = Memvid::create(&path).unwrap();
        let opts = PutOptions {
            uri: Some("mv2://doc1".to_string()),
            ..Default::default()
        };
        mem.put_bytes_with_options(b"First document", opts).unwrap();
        mem.commit().unwrap();
    }

    // Second commit
    {
        let mut mem = Memvid::open(&path).unwrap();
        let opts = PutOptions {
            uri: Some("mv2://doc2".to_string()),
            ..Default::default()
        };
        mem.put_bytes_with_options(b"Second document", opts)
            .unwrap();
        mem.commit().unwrap();
    }

    // Verify both documents exist
    let mem = Memvid::open_read_only(&path).unwrap();
    let stats = mem.stats().unwrap();

    assert_eq!(
        stats.frame_count, 2,
        "Should have 2 frames after multiple commits"
    );
}

/// Regression for reopen-per-append storage amplification.
///
/// A common service lifecycle is `open -> append -> commit -> close` for each
/// small write. That must overwrite the previous derived index/TOC layer on
/// each commit instead of appending a new full layer after the old one.
#[test]
#[cfg(feature = "lex")]
fn reopen_per_append_keeps_single_compact_index_layer() {
    const DOCS: u32 = 150;
    const MAX_REASONABLE_SIZE: u64 = 8 * 1024 * 1024;

    let dir = TempDir::new().unwrap();
    let path = dir.path().join("reopen_amplification.mv2");

    {
        let mut mem = Memvid::create(&path).unwrap();
        mem.enable_lex().unwrap();
        mem.commit().unwrap();
    }

    for i in 0..DOCS {
        let mut mem = Memvid::open(&path).unwrap();
        let text =
            format!("pioneer memory reopen amplification regression document {i} alpha beta gamma");
        let opts = PutOptions {
            uri: Some(format!("mv2://reopen-amplification/{i}")),
            title: Some(format!("doc-{i}")),
            search_text: Some(text.clone()),
            ..Default::default()
        };
        mem.put_bytes_with_options(text.as_bytes(), opts)
            .unwrap_or_else(|err| panic!("put {i} failed: {err}"));
        mem.commit()
            .unwrap_or_else(|err| panic!("commit {i} failed: {err}"));
    }

    let mut mem = Memvid::open_read_only(&path).unwrap();
    let stats = mem.stats().unwrap();
    assert_eq!(stats.frame_count, u64::from(DOCS));
    assert!(stats.has_lex_index, "lex index should be persisted");

    let result = mem.search(memvid_core::SearchRequest {
        query: "reopen amplification regression".into(),
        top_k: 5,
        snippet_chars: 160,
        uri: None,
        scope: None,
        cursor: None,
        #[cfg(feature = "temporal_track")]
        temporal: None,
        as_of_frame: None,
        as_of_ts: None,
        no_sketch: false,
        acl_context: None,
        acl_enforcement_mode: memvid_core::AclEnforcementMode::Audit,
    });
    assert!(
        result.unwrap().total_hits > 0,
        "lex search should still work"
    );

    let size = fs::metadata(&path).unwrap().len();
    assert!(
        size <= MAX_REASONABLE_SIZE,
        "reopen-per-append produced an oversized file: {size} bytes for {DOCS} small docs"
    );
}

#[test]
#[cfg(feature = "lex")]
fn reopen_append_after_skip_finalize_keeps_indexes_searchable() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("skip_finalize_reopen.mv2");

    {
        let mut mem = Memvid::create(&path).unwrap();
        mem.enable_lex().unwrap();
        for i in 0..16 {
            let text = format!("bulk finalize searchable document {i}");
            let opts = PutOptions {
                uri: Some(format!("mv2://bulk/{i}")),
                search_text: Some(text.clone()),
                ..Default::default()
            };
            mem.put_bytes_with_options(text.as_bytes(), opts).unwrap();
        }
        mem.commit_skip_indexes().unwrap();
        mem.finalize_indexes().unwrap();
    }

    {
        let mut mem = Memvid::open(&path).unwrap();
        let text = "bulk finalize searchable reopenedmarker";
        let opts = PutOptions {
            uri: Some("mv2://bulk/reopened".to_string()),
            search_text: Some(text.to_string()),
            ..Default::default()
        };
        mem.put_bytes_with_options(text.as_bytes(), opts).unwrap();
        mem.commit().unwrap();
    }

    let mut mem = Memvid::open_read_only(&path).unwrap();
    let stats = mem.stats().unwrap();
    assert_eq!(stats.frame_count, 17);
    assert!(stats.has_lex_index);

    let response = mem
        .search(memvid_core::SearchRequest {
            query: "reopenedmarker".into(),
            top_k: 5,
            snippet_chars: 160,
            uri: None,
            scope: None,
            cursor: None,
            #[cfg(feature = "temporal_track")]
            temporal: None,
            as_of_frame: None,
            as_of_ts: None,
            no_sketch: true,
            acl_context: None,
            acl_enforcement_mode: memvid_core::AclEnforcementMode::Audit,
        })
        .unwrap();
    assert_eq!(
        response.hits.first().map(|hit| hit.uri.as_str()),
        Some("mv2://bulk/reopened")
    );
}

#[test]
#[cfg(feature = "lex")]
fn reopen_delete_then_append_keeps_tombstone_and_search_consistent() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("delete_reopen.mv2");

    {
        let mut mem = Memvid::create(&path).unwrap();
        mem.enable_lex().unwrap();
        let deleted_opts = PutOptions {
            uri: Some("mv2://delete/old".to_string()),
            search_text: Some("obsolete tombstone onlyword".to_string()),
            ..Default::default()
        };
        mem.put_bytes_with_options(b"obsolete tombstone onlyword", deleted_opts)
            .unwrap();
        let keep_opts = PutOptions {
            uri: Some("mv2://delete/keep".to_string()),
            search_text: Some("retained live document".to_string()),
            ..Default::default()
        };
        mem.put_bytes_with_options(b"retained live document", keep_opts)
            .unwrap();
        mem.commit().unwrap();
    }

    {
        let mut mem = Memvid::open(&path).unwrap();
        let old = mem.frame_by_uri("mv2://delete/old").unwrap();
        mem.delete_frame(old.id).unwrap();
        mem.commit().unwrap();
    }

    {
        let mut mem = Memvid::open(&path).unwrap();
        let opts = PutOptions {
            uri: Some("mv2://delete/new".to_string()),
            search_text: Some("new live append document".to_string()),
            ..Default::default()
        };
        mem.put_bytes_with_options(b"new live append document", opts)
            .unwrap();
        mem.commit().unwrap();
    }

    let mut mem = Memvid::open_read_only(&path).unwrap();
    let deleted = mem
        .search(memvid_core::SearchRequest {
            query: "onlyword".into(),
            top_k: 5,
            snippet_chars: 120,
            uri: None,
            scope: None,
            cursor: None,
            #[cfg(feature = "temporal_track")]
            temporal: None,
            as_of_frame: None,
            as_of_ts: None,
            no_sketch: false,
            acl_context: None,
            acl_enforcement_mode: memvid_core::AclEnforcementMode::Audit,
        })
        .unwrap();
    assert_eq!(
        deleted.total_hits, 0,
        "deleted frame must not be searchable"
    );

    let live = mem
        .search(memvid_core::SearchRequest {
            query: "new live append".into(),
            top_k: 5,
            snippet_chars: 120,
            uri: None,
            scope: None,
            cursor: None,
            #[cfg(feature = "temporal_track")]
            temporal: None,
            as_of_frame: None,
            as_of_ts: None,
            no_sketch: false,
            acl_context: None,
            acl_enforcement_mode: memvid_core::AclEnforcementMode::Audit,
        })
        .unwrap();
    assert_eq!(
        live.hits.first().map(|hit| hit.uri.as_str()),
        Some("mv2://delete/new")
    );
}

#[test]
#[cfg(feature = "lex")]
fn reopen_append_preserves_existing_search_ranking() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("ranking_reopen.mv2");

    {
        let mut mem = Memvid::create(&path).unwrap();
        mem.enable_lex().unwrap();
        let docs = [
            ("mv2://rank/exact", "alpha beta gamma gamma precise match"),
            ("mv2://rank/secondary", "alpha beta supporting context"),
            ("mv2://rank/alpha", "alpha only unrelated"),
            ("mv2://rank/beta", "beta only unrelated"),
        ];
        for (uri, text) in docs {
            let opts = PutOptions {
                uri: Some(uri.to_string()),
                search_text: Some(text.to_string()),
                ..Default::default()
            };
            mem.put_bytes_with_options(text.as_bytes(), opts).unwrap();
        }
        mem.commit().unwrap();
    }

    let before = {
        let mut mem = Memvid::open_read_only(&path).unwrap();
        let response = mem
            .search(memvid_core::SearchRequest {
                query: "alpha beta".into(),
                top_k: 10,
                snippet_chars: 160,
                uri: None,
                scope: None,
                cursor: None,
                #[cfg(feature = "temporal_track")]
                temporal: None,
                as_of_frame: None,
                as_of_ts: None,
                no_sketch: true,
                acl_context: None,
                acl_enforcement_mode: memvid_core::AclEnforcementMode::Audit,
            })
            .unwrap();
        response
            .hits
            .into_iter()
            .map(|hit| hit.uri)
            .collect::<Vec<_>>()
    };
    assert!(
        before.len() >= 2,
        "test corpus should produce multiple ranked hits"
    );

    {
        let mut mem = Memvid::open(&path).unwrap();
        let text = "omega epsilon zeta unrelated append";
        let opts = PutOptions {
            uri: Some("mv2://rank/unrelated-append".to_string()),
            search_text: Some(text.to_string()),
            ..Default::default()
        };
        mem.put_bytes_with_options(text.as_bytes(), opts).unwrap();
        mem.commit().unwrap();
    }

    let after = {
        let mut mem = Memvid::open_read_only(&path).unwrap();
        let response = mem
            .search(memvid_core::SearchRequest {
                query: "alpha beta".into(),
                top_k: 10,
                snippet_chars: 160,
                uri: None,
                scope: None,
                cursor: None,
                #[cfg(feature = "temporal_track")]
                temporal: None,
                as_of_frame: None,
                as_of_ts: None,
                no_sketch: true,
                acl_context: None,
                acl_enforcement_mode: memvid_core::AclEnforcementMode::Audit,
            })
            .unwrap();
        response
            .hits
            .into_iter()
            .map(|hit| hit.uri)
            .collect::<Vec<_>>()
    };

    assert_eq!(
        after, before,
        "reopen append should not change ranking for an existing query"
    );
}

#[test]
#[cfg(feature = "lex")]
fn reopen_append_matches_single_session_search_quality_suite() {
    let dir = TempDir::new().unwrap();
    let batch_path = dir.path().join("search_quality_batch.mv2");
    let reopen_path = dir.path().join("search_quality_reopen.mv2");

    let initial_docs = [
        (
            "mv2://quality/rust-ownership",
            "rust ownership borrowing lifetimes compiler memory safety",
        ),
        (
            "mv2://quality/rust-async",
            "rust async await futures runtime task scheduling",
        ),
        (
            "mv2://quality/python-dataframe",
            "python pandas dataframe groupby analysis notebook",
        ),
        (
            "mv2://quality/ml-transformers",
            "machine learning transformers attention neural networks embeddings",
        ),
        (
            "mv2://quality/database-index",
            "database indexing btree query planner transaction log",
        ),
        (
            "mv2://quality/vector-search",
            "vector search embeddings nearest neighbor index recall",
        ),
        (
            "mv2://quality/climate-policy",
            "climate sustainability renewable energy carbon policy",
        ),
        (
            "mv2://quality/temporal-memory",
            "temporal memory timeline events timestamp retrieval",
        ),
    ];
    let appended_docs = [
        (
            "mv2://quality/rust-reopen",
            "rust borrow checker ownership lifetime practical example",
        ),
        (
            "mv2://quality/vector-reopen",
            "vector embeddings semantic search nearest neighbor retrieval",
        ),
        (
            "mv2://quality/database-reopen",
            "database query planner indexing btree storage engine",
        ),
        (
            "mv2://quality/unrelated-reopen",
            "orchestra violin concert hall acoustic performance",
        ),
    ];
    let queries = [
        "rust ownership",
        "python dataframe",
        "machine learning embeddings",
        "database indexing",
        "vector search",
        "renewable carbon",
        "temporal memory",
    ];

    {
        let mut mem = Memvid::create(&batch_path).unwrap();
        mem.enable_lex().unwrap();
        for (uri, text) in initial_docs.iter().chain(appended_docs.iter()) {
            put_search_doc(&mut mem, uri, text);
        }
        mem.commit().unwrap();
    }

    {
        let mut mem = Memvid::create(&reopen_path).unwrap();
        mem.enable_lex().unwrap();
        for (uri, text) in initial_docs {
            put_search_doc(&mut mem, uri, text);
        }
        mem.commit().unwrap();
    }
    for (uri, text) in appended_docs {
        let mut mem = Memvid::open(&reopen_path).unwrap();
        put_search_doc(&mut mem, uri, text);
        mem.commit().unwrap();
    }

    let mut batch = Memvid::open_read_only(&batch_path).unwrap();
    let mut reopen = Memvid::open_read_only(&reopen_path).unwrap();
    for query in queries {
        let batch_uris = search_uris(&mut batch, query, 5);
        let reopen_uris = search_uris(&mut reopen, query, 5);
        assert!(
            !batch_uris.is_empty(),
            "quality fixture query should produce hits: {query}"
        );
        assert_eq!(
            reopen_uris, batch_uris,
            "reopen append should match single-session ranking for query: {query}"
        );
    }
}

#[test]
fn reopen_append_keeps_vec_index_searchable() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("vec_reopen.mv2");

    {
        let mut mem = Memvid::create(&path).unwrap();
        mem.enable_vec().unwrap();
        mem.put_with_embedding(b"north vector", vec![0.0, 1.0])
            .unwrap();
        mem.commit().unwrap();
    }

    {
        let mut mem = Memvid::open(&path).unwrap();
        mem.put_with_embedding(b"east vector", vec![1.0, 0.0])
            .unwrap();
        mem.commit().unwrap();
    }

    let mut mem = Memvid::open_read_only(&path).unwrap();
    let stats = mem.stats().unwrap();
    assert!(
        stats.has_vec_index,
        "vec index should survive reopen append"
    );
    let hits = mem.search_vec(&[1.0, 0.0], 2).unwrap();
    assert_eq!(hits.first().map(|hit| hit.frame_id), Some(1));
}

#[test]
#[cfg(all(feature = "lex", feature = "temporal_track"))]
fn reopen_append_keeps_temporal_filter_searchable() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("temporal_reopen.mv2");

    {
        let mut mem = Memvid::create(&path).unwrap();
        mem.enable_lex().unwrap();
        let old_opts = PutOptions {
            uri: Some("mv2://temporal/old".to_string()),
            search_text: Some("temporal regression shared term".to_string()),
            timestamp: Some(1_700_000_000),
            ..Default::default()
        };
        mem.put_bytes_with_options(b"temporal old", old_opts)
            .unwrap();
        mem.commit().unwrap();
    }

    {
        let mut mem = Memvid::open(&path).unwrap();
        let new_opts = PutOptions {
            uri: Some("mv2://temporal/new".to_string()),
            search_text: Some("temporal regression shared term".to_string()),
            timestamp: Some(1_700_100_000),
            ..Default::default()
        };
        mem.put_bytes_with_options(b"temporal new", new_opts)
            .unwrap();
        mem.commit().unwrap();
    }

    let mut mem = Memvid::open_read_only(&path).unwrap();
    let response = mem
        .search(memvid_core::SearchRequest {
            query: "temporal regression".into(),
            top_k: 10,
            snippet_chars: 120,
            uri: None,
            scope: None,
            cursor: None,
            temporal: Some(memvid_core::TemporalFilter {
                start_utc: Some(1_700_050_000),
                end_utc: Some(1_700_150_000),
                phrase: None,
                tz: None,
            }),
            as_of_frame: None,
            as_of_ts: None,
            no_sketch: false,
            acl_context: None,
            acl_enforcement_mode: memvid_core::AclEnforcementMode::Audit,
        })
        .unwrap();
    assert_eq!(response.hits.len(), 1);
    assert_eq!(response.hits[0].uri, "mv2://temporal/new");
}

/// Test commit without changes is a no-op.
#[test]
fn commit_without_changes() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.mv2");

    {
        let mut mem = Memvid::create(&path).unwrap();
        let opts = PutOptions {
            uri: Some("mv2://doc1".to_string()),
            ..Default::default()
        };
        mem.put_bytes_with_options(b"Content", opts).unwrap();
        mem.commit().unwrap();
    }

    let size_before = fs::metadata(&path).unwrap().len();

    // Open and commit without changes
    {
        let mut mem = Memvid::open(&path).unwrap();
        mem.commit().unwrap();
    }

    let size_after = fs::metadata(&path).unwrap().len();

    // Size should be approximately the same (may differ slightly due to timestamp updates)
    assert!(
        (size_after as i64 - size_before as i64).abs() < 1024,
        "Commit without changes should not significantly change file size"
    );
}
