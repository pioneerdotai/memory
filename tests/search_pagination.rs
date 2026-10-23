#![cfg(feature = "lex")]

use memvid_core::types::{AclContext, AclEnforcementMode};
use memvid_core::{Memvid, PutOptions, SearchRequest};
use tempfile::TempDir;

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
            PutOptions::builder()
                .uri(format!("mv2://other/{i}"))
                .build(),
        )
        .unwrap();
    }
    for i in 0..13 {
        let text = format!("needle {}", "filler ".repeat(100));
        mem.put_bytes_with_options(
            text.as_bytes(),
            PutOptions::builder()
                .uri(format!("mv2://wanted/{i}"))
                .build(),
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
    let id = mem
        .put_with_embedding_and_options(
            b"compare changes providers",
            vec![1.0, 0.0],
            PutOptions {
                timestamp: Some(100),
                ..Default::default()
            },
        )
        .unwrap();
    mem.put_with_embedding_and_options(
        b"unrelated recent entry",
        vec![0.0, 1.0],
        PutOptions {
            timestamp: Some(200),
            ..Default::default()
        },
    )
    .unwrap();
    mem.commit().unwrap();
    for query in ["providers", "compare changes providers"] {
        let mut req = request(query, 1, None);
        req.query_embedding = Some(vec![1.0, 0.0]);
        let response = mem.search(req).unwrap();
        assert_eq!(response.hits.len(), 1);
        assert_eq!(response.hits[0].frame_id, id);
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
            ..Default::default()
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
            PutOptions::builder().uri(format!("mv2://doc/{i}")).build(),
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
                ..Default::default()
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
    mem.put_bytes(text.as_bytes()).unwrap();
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
