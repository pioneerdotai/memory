#![cfg(all(feature = "lex", feature = "hnsw_bench"))]

#[cfg(feature = "parallel_segments")]
use memvid_core::BuildOpts;
use memvid_core::{Memvid, PutManyOpts, PutOptions, SketchVariant};
use tempfile::TempDir;

fn embedding(frame_id: usize) -> Vec<f32> {
    vec![frame_id as f32, 0.5, 1.0, 2.0]
}

fn options(frame_id: usize) -> PutOptions {
    PutOptions::builder()
        .uri(format!("mv2://encoding/{frame_id}"))
        .search_text(format!("vector encoding document {frame_id}"))
        .auto_tag(false)
        .extract_dates(false)
        .extract_triplets(false)
        .instant_index(false)
        .extraction_budget_ms(0)
        .build()
}

fn populate(mem: &mut Memvid, count: usize) {
    mem.begin_batch(PutManyOpts {
        wal_pre_size_bytes: 2 * 1024 * 1024,
        disable_auto_checkpoint: true,
        enable_enrichment: false,
        skip_sync: true,
        ..Default::default()
    })
    .unwrap();
    for frame_id in 0..count {
        let payload = format!("vector encoding document {frame_id}");
        mem.put_with_embedding_and_options(
            payload.as_bytes(),
            embedding(frame_id),
            options(frame_id),
        )
        .unwrap();
    }
    mem.end_batch().unwrap();
}

fn assert_exact(mem: &mut Memvid, frame_id: usize) {
    let hit = mem.search_vec(&embedding(frame_id), 1).unwrap()[0].clone();
    assert_eq!(hit.frame_id, frame_id as u64);
    assert_eq!(hit.distance, 0.0);
}

fn sketch_only_rebuild(count: usize, parallel_initial: bool, warm_cache: bool) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!(
        "encoding-{count}-{parallel_initial}-{warm_cache}.mv2"
    ));
    let mut mem = Memvid::create(&path).unwrap();
    mem.enable_vec().unwrap();
    populate(&mut mem, count);
    if parallel_initial {
        #[cfg(feature = "parallel_segments")]
        mem.commit_parallel(BuildOpts {
            threads: 1,
            segment_tokens: 100,
            segment_pages: 25,
            ..Default::default()
        })
        .unwrap();
        #[cfg(not(feature = "parallel_segments"))]
        panic!("parallel fixture requires parallel_segments");
    } else {
        mem.commit().unwrap();
    }
    drop(mem);

    let probe = count / 2;
    let mut before = Memvid::open_read_only(&path).unwrap();
    assert_exact(&mut before, probe);
    drop(before);

    let mut writer = Memvid::open(&path).unwrap();
    if warm_cache {
        assert_exact(&mut writer, probe);
    }
    writer.insert_sketch(
        probe as u64,
        "encoding-preserving sketch",
        SketchVariant::Small,
    );
    writer.commit().unwrap();
    assert_exact(&mut writer, probe);
    drop(writer);

    let mut reopened = Memvid::open_read_only(&path).unwrap();
    assert_exact(&mut reopened, probe);
    assert_eq!(
        reopened.frame_canonical_payload(probe as u64).unwrap(),
        format!("vector encoding document {probe}").as_bytes()
    );
}

#[test]
fn monolithic_hnsw_survives_cold_and_warm_sketch_rebuild() {
    sketch_only_rebuild(1000, false, false);
    sketch_only_rebuild(1000, false, true);
}

#[test]
fn below_hnsw_threshold_survives_sketch_rebuild() {
    sketch_only_rebuild(999, false, false);
}

#[cfg(feature = "parallel_segments")]
#[test]
fn segment_backed_vectors_survive_hnsw_threshold_materialization() {
    sketch_only_rebuild(1000, true, false);
    sketch_only_rebuild(1000, true, true);
}

#[cfg(feature = "parallel_segments")]
#[test]
fn pending_replacements_survive_large_monolithic_and_segment_sources() {
    for (initial_parallel, warm_cache) in [(false, true), (true, false)] {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!(
            "large-replacement-{initial_parallel}-{warm_cache}.mv2"
        ));
        let mut mem = Memvid::create(&path).unwrap();
        mem.enable_vec().unwrap();
        populate(&mut mem, 1000);
        if initial_parallel {
            mem.commit_parallel(BuildOpts {
                threads: 1,
                segment_tokens: 100,
                segment_pages: 25,
                ..Default::default()
            })
            .unwrap();
        } else {
            mem.commit().unwrap();
        }
        drop(mem);

        let replacement = vec![-500.0, 0.25, 0.75, 1.25];
        let appended = vec![-1000.0, 0.25, 0.75, 1.25];
        let mut writer = Memvid::open(&path).unwrap();
        if warm_cache {
            assert_exact(&mut writer, 500);
        }
        writer
            .add_embeddings(vec![(500, replacement.clone())])
            .unwrap();
        writer
            .put_with_embedding_and_options(
                b"appended large vector",
                appended.clone(),
                options(1000),
            )
            .unwrap();
        writer
            .commit_parallel(BuildOpts {
                threads: 1,
                segment_tokens: 100,
                segment_pages: 25,
                ..Default::default()
            })
            .unwrap();
        let replaced = writer.search_vec(&replacement, 1).unwrap()[0].clone();
        assert_eq!(replaced.frame_id, 500);
        assert_eq!(replaced.distance, 0.0);
        let appended_hit = writer.search_vec(&appended, 1).unwrap()[0].clone();
        assert_eq!(appended_hit.frame_id, 1000);
        assert_eq!(appended_hit.distance, 0.0);
        assert_exact(&mut writer, 501);
        drop(writer);

        let mut reopened = Memvid::open_read_only(&path).unwrap();
        let replaced = reopened.search_vec(&replacement, 1).unwrap()[0].clone();
        assert_eq!(replaced.frame_id, 500);
        assert_eq!(replaced.distance, 0.0);
        let appended_hit = reopened.search_vec(&appended, 1).unwrap()[0].clone();
        assert_eq!(appended_hit.frame_id, 1000);
        assert_eq!(appended_hit.distance, 0.0);
        assert_exact(&mut reopened, 501);
    }
}
