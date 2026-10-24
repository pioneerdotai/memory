#![cfg(feature = "lex")]

use memvid_core::types::{AclContext, AclEnforcementMode};
use memvid_core::{Memvid, PutOptions, SearchRequest};
use tempfile::TempDir;

// These fixtures exercise committed retrieval, not enrichment or instant indexing.
// Stable metadata keeps additional inferred query words out of snippet counts.
fn search_options() -> memvid_core::PutOptionsBuilder {
    PutOptions::builder()
        .title("Search pagination fixture")
        .instant_index(false)
        .auto_tag(false)
        .extract_dates(false)
        .extract_triplets(false)
        .extraction_budget_ms(0)
}

fn request(query: &str, limit: usize, cursor: Option<String>) -> SearchRequest {
    SearchRequest {
        query_embedding: None,
        query: query.into(),
        top_k: limit,
        snippet_chars: 120,
        uri: None,
        scope: None,
        cursor,
        #[cfg(feature = "temporal_track")]
        temporal: None,
        as_of_frame: None,
        as_of_ts: None,
        no_sketch: true,
        acl_context: None,
        acl_enforcement_mode: AclEnforcementMode::Audit,
    }
}

#[test]
fn lexical_pages_reach_matches_after_rejected_document_windows() {
    let dir = TempDir::new().unwrap();
    let mut mem = Memvid::create(dir.path().join("scope.mv2")).unwrap();
    mem.enable_lex().unwrap();
    for i in 0..120 {
        mem.put_bytes_with_options(
            b"needle",
            search_options().uri(format!("mv2://other/{i}")).build(),
        )
        .unwrap();
    }
    for i in 0..13 {
        let text = format!("needle {}", "filler ".repeat(100));
        mem.put_bytes_with_options(
            text.as_bytes(),
            search_options().uri(format!("mv2://wanted/{i}")).build(),
        )
        .unwrap();
    }
    mem.commit().unwrap();
    let mut cursor = None;
    let mut found = std::collections::HashSet::new();
    for page in 0..3 {
        let mut req = request("needle", 5, cursor);
        req.scope = Some("mv2://wanted/".into());
        let response = mem.search(req).unwrap();
        assert_eq!(response.hits.len(), if page == 2 { 3 } else { 5 });
        for hit in response.hits {
            assert!(found.insert(hit.uri));
        }
        cursor = response.next_cursor;
        assert_eq!(cursor.is_none(), page == 2);
    }
    assert_eq!(found.len(), 13);
}

#[test]
fn analytical_wording_does_not_replace_search_with_timeline() {
    let dir = TempDir::new().unwrap();
    let mut mem = Memvid::create(dir.path().join("analytical.mv2")).unwrap();
    mem.enable_lex().unwrap();
    mem.enable_vec().unwrap();
    mem.put_with_embedding_and_options(
        b"compare changes providers",
        vec![1.0, 0.0],
        PutOptions {
            uri: Some("mv2://providers".into()),
            timestamp: Some(100),
            ..search_options().build()
        },
    )
    .unwrap();
    mem.put_with_embedding_and_options(
        b"unrelated recent entry",
        vec![0.0, 1.0],
        PutOptions {
            timestamp: Some(200),
            ..search_options().build()
        },
    )
    .unwrap();
    mem.commit().unwrap();
    for query in ["providers", "compare changes providers"] {
        let mut req = request(query, 1, None);
        req.query_embedding = Some(vec![1.0, 0.0]);
        let response = mem.search(req).unwrap();
        assert_eq!(response.hits.len(), 1);
        assert_eq!(response.hits[0].uri, "mv2://providers");
        assert!(response.next_cursor.is_some());
    }
}

#[test]
fn hybrid_pages_fill_after_vector_scope_and_acl_rejections() {
    let dir = TempDir::new().unwrap();
    let mut mem = Memvid::create(dir.path().join("filtered.mv2")).unwrap();
    mem.enable_lex().unwrap();
    mem.enable_vec().unwrap();
    for i in 0..1320 {
        let allowed = i >= 1100;
        let options = PutOptions {
            uri: Some(format!(
                "mv2://{}/{i}",
                if i < 550 { "other" } else { "wanted" }
            )),
            extra_metadata: [
                (
                    "acl_tenant_id".into(),
                    if allowed || i < 550 { "ours" } else { "other" }.into(),
                ),
                ("acl_visibility".into(), "public".into()),
            ]
            .into_iter()
            .collect(),
            ..search_options().build()
        };
        mem.put_with_embedding_and_options(b"indexed document", vec![i as f32, 1.0], options)
            .unwrap();
    }
    mem.commit().unwrap();
    for hybrid in [false, true] {
        let mut cursor = None;
        for page in 0..3 {
            let mut req = request("absentlexicaltoken", 100, cursor);
            req.query_embedding = Some(vec![0.0, 1.0]);
            req.scope = Some("mv2://wanted/".into());
            req.acl_context = Some(AclContext {
                tenant_id: Some("ours".into()),
                ..Default::default()
            });
            req.acl_enforcement_mode = AclEnforcementMode::Enforce;
            let response = if hybrid {
                mem.search(req).unwrap()
            } else {
                mem.vec_search_with_embedding_acl(
                    &req.query,
                    &[0.0, 1.0],
                    req.top_k,
                    req.snippet_chars,
                    req.scope.as_deref(),
                    req.acl_context.as_ref(),
                    req.acl_enforcement_mode,
                    req.cursor.as_deref(),
                )
                .unwrap()
            };
            assert_eq!(response.hits.len(), if page == 2 { 20 } else { 100 });
            assert!(response.hits.iter().all(|hit| {
                hit.uri
                    .rsplit('/')
                    .next()
                    .unwrap()
                    .parse::<usize>()
                    .unwrap()
                    >= 1100
            }));
            cursor = response.next_cursor;
            assert_eq!(cursor.is_none(), page == 2);
        }
    }
}

#[test]
fn ask_hybrid_continuation_pages_the_combined_order() {
    struct Embedder;
    impl memvid_core::VecEmbedder for Embedder {
        fn embed_query(&self, _: &str) -> memvid_core::Result<Vec<f32>> {
            Ok(vec![1.0, 0.0])
        }
        fn embedding_dimension(&self) -> usize {
            2
        }
    }
    let dir = TempDir::new().unwrap();
    let mut mem = Memvid::create(dir.path().join("ask.mv2")).unwrap();
    mem.enable_lex().unwrap();
    mem.enable_vec().unwrap();
    for i in 0..180 {
        mem.put_with_embedding_and_options(
            b"needle",
            vec![1.0, i as f32 / 180.0],
            search_options().uri(format!("mv2://doc/{i}")).build(),
        )
        .unwrap();
    }
    mem.commit().unwrap();
    let mut req: memvid_core::AskRequest = serde_json::from_value(serde_json::json!({
        "question": "needle", "top_k": 100, "snippet_chars": 120,
        "context_only": true, "mode": "hybrid"
    }))
    .unwrap();
    let first = mem.ask(req.clone(), Some(&Embedder)).unwrap();
    assert_eq!(first.retrieval.hits.len(), 100);
    assert_eq!(first.context_fragments.len(), 100);
    req.cursor = Some(first.retrieval.next_cursor.unwrap());
    let second = mem.ask(req, Some(&Embedder)).unwrap();
    assert_eq!(second.retrieval.hits.len(), 80);
    assert_eq!(second.context_fragments.len(), 80);
    assert!(second.retrieval.next_cursor.is_none());
}

#[test]
fn lexical_pages_apply_acl_before_counting_hits() {
    let dir = TempDir::new().unwrap();
    let mut mem = Memvid::create(dir.path().join("lexical-acl.mv2")).unwrap();
    mem.enable_lex().unwrap();
    for i in 0..103 {
        let permitted = i >= 100;
        let text = if permitted {
            format!("needle {}", "filler ".repeat(100))
        } else {
            "needle".into()
        };
        mem.put_bytes_with_options(
            text.as_bytes(),
            PutOptions {
                extra_metadata: [
                    (
                        "acl_tenant_id".into(),
                        if permitted { "ours" } else { "other" }.into(),
                    ),
                    ("acl_visibility".into(), "public".into()),
                ]
                .into_iter()
                .collect(),
                ..search_options().build()
            },
        )
        .unwrap();
    }
    mem.commit().unwrap();
    let mut cursor = None;
    for size in [2, 1] {
        let mut req = request("needle", 2, cursor);
        req.acl_context = Some(AclContext {
            tenant_id: Some("ours".into()),
            ..Default::default()
        });
        req.acl_enforcement_mode = AclEnforcementMode::Enforce;
        let response = mem.search(req).unwrap();
        assert_eq!(response.hits.len(), size);
        assert!(response.hits.iter().all(|hit| hit.frame_id >= 100));
        cursor = response.next_cursor;
        assert_eq!(cursor.is_none(), size == 1);
    }
}

#[test]
fn lexical_continuation_reaches_later_snippets_of_one_document() {
    let dir = TempDir::new().unwrap();
    let mut mem = Memvid::create(dir.path().join("snippets.mv2")).unwrap();
    mem.enable_lex().unwrap();
    let text = format!("needle. {}", "filler. ".repeat(30)).repeat(8);
    mem.put_bytes_with_options(
        text.as_bytes(),
        search_options()
            .title("Snippet pagination document")
            .auto_tag(false)
            .extract_triplets(false)
            .build(),
    )
    .unwrap();
    mem.commit().unwrap();
    let mut cursor = None;
    let mut ranges = std::collections::HashSet::new();
    for page in 0..4 {
        let response = mem.search(request("needle", 2, cursor)).unwrap();
        assert_eq!(response.hits.len(), 2);
        for hit in response.hits {
            assert!(ranges.insert((hit.frame_id, hit.range)));
        }
        cursor = response.next_cursor;
        assert_eq!(cursor.is_none(), page == 3);
    }
    assert_eq!(ranges.len(), 8);
}

#[test]
fn ask_pages_grow_past_duplicate_snippets_before_later_sources() {
    let dir = TempDir::new().unwrap();
    let mut mem = Memvid::create(dir.path().join("ask-snippets.mv2")).unwrap();
    mem.enable_lex().unwrap();
    // Keep each payload below automatic document chunking: this case needs
    // repeated snippets of one frame, followed by three other source frames.
    let repeated = format!("needle. {}", "padding. ".repeat(20)).repeat(12);
    let mut expected = std::collections::HashSet::new();
    expected.insert("mv2://first".to_string());
    mem.put_bytes_with_options(
        repeated.as_bytes(),
        search_options().uri("mv2://first").build(),
    )
    .unwrap();
    for i in 0..3 {
        let text = format!("needle {}", "padding ".repeat(100));
        let uri = format!("mv2://later/{i}");
        expected.insert(uri.clone());
        mem.put_bytes_with_options(text.as_bytes(), search_options().uri(uri).build())
            .unwrap();
    }
    mem.commit().unwrap();
    let mut cursor = None;
    let mut seen = std::collections::HashSet::new();
    for page in 0..2 {
        let req: memvid_core::AskRequest = serde_json::from_value(serde_json::json!({
            "question": "needle", "top_k": 2, "snippet_chars": 80,
            "context_only": true, "mode": "lex", "cursor": cursor,
        }))
        .unwrap();
        let response = mem.ask::<dyn memvid_core::VecEmbedder>(req, None).unwrap();
        assert_eq!(response.retrieval.hits.len(), 2);
        for hit in response.retrieval.hits {
            assert!(seen.insert(hit.uri));
        }
        cursor = response.retrieval.next_cursor;
        assert_eq!(cursor.is_none(), page == 1);
    }
    assert_eq!(seen, expected);
}

#[test]
#[cfg(not(feature = "temporal_track"))]
fn ask_pages_grow_past_timestamp_postfilters_before_later_sources() {
    // Without temporal_track, ask's timestamp checks are strictly post-retrieval.
    let dir = TempDir::new().unwrap();
    let mut mem = Memvid::create(dir.path().join("ask-time.mv2")).unwrap();
    mem.enable_lex().unwrap();
    mem.put_bytes_with_options(
        b"needle",
        PutOptions {
            uri: Some("mv2://first".into()),
            timestamp: Some(200),
            ..search_options().build()
        },
    )
    .unwrap();
    for _ in 0..40 {
        mem.put_bytes_with_options(
            b"needle padding",
            PutOptions {
                timestamp: Some(50),
                ..search_options().build()
            },
        )
        .unwrap();
    }
    let mut expected: std::collections::HashSet<_> =
        ["mv2://first".to_string()].into_iter().collect();
    for i in 0..3 {
        let text = format!("needle {}", "padding ".repeat(100));
        let uri = format!("mv2://later/{i}");
        expected.insert(uri.clone());
        mem.put_bytes_with_options(
            text.as_bytes(),
            PutOptions {
                timestamp: Some(200),
                uri: Some(uri),
                ..search_options().build()
            },
        )
        .unwrap();
    }
    mem.commit().unwrap();
    let mut cursor = None;
    let mut seen = std::collections::HashSet::new();
    for page in 0..2 {
        let req: memvid_core::AskRequest = serde_json::from_value(serde_json::json!({
            "question": "needle", "top_k": 2, "snippet_chars": 80,
            "context_only": true, "mode": "lex", "start": 100, "cursor": cursor,
        }))
        .unwrap();
        let response = mem.ask::<dyn memvid_core::VecEmbedder>(req, None).unwrap();
        assert_eq!(response.retrieval.hits.len(), 2);
        for hit in response.retrieval.hits {
            assert!(seen.insert(hit.uri));
        }
        cursor = response.retrieval.next_cursor;
        assert_eq!(cursor.is_none(), page == 1);
    }
    assert_eq!(seen, expected);
}

#[test]
fn hybrid_query_fields_filter_semantic_candidates_before_paging() {
    let dir = TempDir::new().unwrap();
    let mut mem = Memvid::create(dir.path().join("query-fields.mv2")).unwrap();
    mem.enable_lex().unwrap();
    mem.enable_vec().unwrap();
    for i in 0..85 {
        mem.put_with_embedding_and_options(
            b"semantically related document",
            vec![1.0, i as f32 / 100.0],
            PutOptions {
                tags: vec![if i >= 80 { "allowed" } else { "denied" }.into()],
                timestamp: Some(if i >= 80 { 1_704_067_200 } else { 946_684_800 }),
                ..search_options().build()
            },
        )
        .unwrap();
    }
    mem.commit().unwrap();
    let mut search_page = |query: &str, cursor: Option<String>, hybrid: bool| {
        if hybrid {
            let mut req = request(query, 2, cursor);
            req.query_embedding = Some(vec![1.0, 0.0]);
            mem.search(req)
        } else {
            mem.vec_search_with_embedding_acl(
                query,
                &[1.0, 0.0],
                2,
                120,
                None,
                None,
                AclEnforcementMode::Audit,
                cursor.as_deref(),
            )
        }
        .unwrap_or_else(|err| panic!("query={query}, hybrid={hybrid}: {err:?}"))
    };
    for query in [
        "absenttoken AND tag:allowed",
        "absenttoken AND NOT tag:denied",
        "absenttoken AND (tag:allowed OR tag:missing)",
        "absenttoken AND NOT (tag:denied OR tag:missing)",
        "absenttoken AND date:[2024-01-01 TO 2024-12-31]",
        "NOT (semantically OR tag:denied)",
        "(absenttoken AND tag:allowed) OR (anotherabsent AND tag:missing)",
        "absenttoken AND NOT date:[2000-01-01 TO 2000-12-31]",
        "(needle OR tag:allowed) AND NOT needle",
        "(NEEDLE OR tag:allowed) AND NOT \"needle\"",
        "(needle OR otherneedle OR tag:allowed) AND NOT needle AND NOT otherneedle",
        "(need* OR tag:allowed) AND NOT need*",
    ] {
        for hybrid in [false, true] {
            let mut cursor = None;
            let mut seen = std::collections::HashSet::new();
            for count in [2, 2, 1] {
                let response = search_page(query, cursor, hybrid);
                assert_eq!(response.hits.len(), count, "{query}, hybrid={hybrid}");
                for hit in response.hits {
                    assert!(hit.frame_id >= 80, "{query}, hybrid={hybrid}");
                    seen.insert(hit.frame_id);
                }
                cursor = response.next_cursor;
                assert_eq!(cursor.is_none(), count == 1);
            }
            assert_eq!(seen.len(), 5);
        }
    }
    for query in [
        "needle AND NOT needle",
        "(needle OR otherneedle) AND NOT needle AND NOT otherneedle",
        "\"same phrase\" AND NOT \"SAME PHRASE\"",
        "need* AND NOT need*",
    ] {
        for hybrid in [false, true] {
            let response = search_page(query, None, hybrid);
            assert!(response.hits.is_empty(), "{query}, hybrid={hybrid}");
            assert!(response.next_cursor.is_none(), "{query}, hybrid={hybrid}");
        }
    }
}

#[test]
fn hybrid_uri_uses_lexical_prefix_case_and_scope_precedence() {
    let dir = TempDir::new().unwrap();
    let mut mem = Memvid::create(dir.path().join("uri.mv2")).unwrap();
    mem.enable_lex().unwrap();
    mem.enable_vec().unwrap();
    // Short exact text matches outrank the wanted, longer documents in BM25.
    // These nearer vectors also precede the wanted ones in semantic retrieval.
    // Both paths must refill past their first internal candidate window.
    for i in 0..80 {
        mem.put_with_embedding_and_options(
            b"needle",
            vec![1.0, 0.0],
            search_options().uri(format!("mv2://other/{i}")).build(),
        )
        .unwrap();
    }
    for uri in [
        "mv2://Wanted/parent#one",
        "mv2://Wanted/parent#two",
        "mv2://other",
    ] {
        mem.put_with_embedding_and_options(
            b"needle with additional context about the saved decision",
            vec![1.0, 1.0],
            search_options().uri(uri).build(),
        )
        .unwrap();
    }
    mem.commit().unwrap();
    for semantic in [false, true] {
        for (uri, expected) in [("MV2://WANTED/", 2), ("MV2://WANTED/PARENT#ONE", 1)] {
            let mut cursor = None;
            let mut seen = std::collections::HashSet::new();
            for page in 0..expected {
                let mut req = request(if semantic { "absenttoken" } else { "needle" }, 1, cursor);
                req.uri = Some(uri.into());
                req.scope = Some("mv2://other".into());
                if semantic {
                    req.query_embedding = Some(vec![1.0, 0.0]);
                }
                let response = mem.search(req).unwrap();
                assert_eq!(response.hits.len(), 1);
                assert!(response.hits[0].uri.starts_with("mv2://Wanted/"));
                assert!(seen.insert(response.hits[0].uri.clone()));
                cursor = response.next_cursor;
                assert_eq!(cursor.is_none(), page + 1 == expected);
            }
        }
    }
}

#[test]
fn zero_pages_are_rejected_without_nonadvancing_cursors() {
    let dir = TempDir::new().unwrap();
    let mut mem = Memvid::create(dir.path().join("zero.mv2")).unwrap();
    mem.enable_lex().unwrap();
    mem.enable_vec().unwrap();
    mem.put_with_embedding_and_options(b"needle", vec![1.0, 0.0], search_options().build())
        .unwrap();
    mem.commit().unwrap();
    for cursor in [None, Some("0".into()), Some("1".into())] {
        for hybrid in [false, true] {
            let mut req = request("needle", 0, cursor.clone());
            if hybrid {
                req.query_embedding = Some(vec![1.0, 0.0]);
            }
            assert!(matches!(
                mem.search(req),
                Err(memvid_core::MemvidError::InvalidQuery { .. })
            ));
        }
        assert!(matches!(
            mem.vec_search_with_embedding_acl(
                "needle",
                &[1.0, 0.0],
                0,
                120,
                None,
                None,
                AclEnforcementMode::Audit,
                cursor.as_deref(),
            ),
            Err(memvid_core::MemvidError::InvalidQuery { .. })
        ));
        for mode in ["lex", "sem", "hybrid"] {
            let req: memvid_core::AskRequest = serde_json::from_value(serde_json::json!({
                "question": "needle", "top_k": 0, "snippet_chars": 120, "context_only": true,
                "mode": mode, "cursor": cursor,
            }))
            .unwrap();
            assert!(matches!(
                mem.ask::<dyn memvid_core::VecEmbedder>(req, None),
                Err(memvid_core::MemvidError::InvalidQuery { .. })
            ));
        }
    }
}
