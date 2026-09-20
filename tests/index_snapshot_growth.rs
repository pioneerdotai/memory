#![cfg(feature = "lex")]

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

#[cfg(feature = "parallel_segments")]
use memvid_core::BuildOpts;
use memvid_core::constants::HEADER_SIZE;
use memvid_core::io::header::HeaderCodec;
use memvid_core::memvid::sketch::SketchSearchOptions;
#[cfg(feature = "parallel_segments")]
use memvid_core::read_sketch_track;
use memvid_core::{
    FrameStatus, Memvid, PutManyOpts, PutOptions, SearchRequest, SketchTrack, SketchVariant,
};
use tempfile::TempDir;

const DIMENSION: usize = 1024;

fn document(i: usize) -> String {
    format!(
        "Synthetic document {i} contains searchable token group{}",
        i % 5
    )
}

fn embedding(i: usize) -> Vec<f32> {
    let mut value = vec![0.0; DIMENSION];
    value[i % DIMENSION] = 1.0;
    value
}

fn options(i: usize) -> PutOptions {
    PutOptions::builder()
        .uri(format!("mv2://growth/{i}"))
        .search_text(document(i))
        .auto_tag(false)
        .extract_dates(false)
        .extract_triplets(false)
        .instant_index(false)
        .extraction_budget_ms(0)
        .build()
}

fn add(mem: &mut Memvid, i: usize) {
    let text = document(i);
    mem.put_with_embedding_and_options(text.as_bytes(), embedding(i), options(i))
        .unwrap();
}

fn build_commit_each(path: &Path, count: usize, reopen_each: bool) -> u64 {
    let mut mem = Memvid::create(path).unwrap();
    mem.enable_vec().unwrap();
    mem.set_vec_model("growth-regression-1024").unwrap();
    for i in 0..count {
        add(&mut mem, i);
        mem.commit().unwrap();
        if reopen_each {
            drop(mem);
            mem = Memvid::open(path).unwrap();
        }
    }
    let stats = mem.stats().unwrap();
    let size = File::open(path).unwrap().metadata().unwrap().len();

    // This is deliberately a generous structural bound, not an exact byte
    // assertion. Before the fix, N=32 retains every 1..N vector/sketch
    // snapshot and exceeds it by several MiB. A compact current generation is
    // well below it even with WAL, Tantivy, TOC, and allocator variation.
    let live_allowance = stats
        .wal_bytes
        .saturating_add(stats.payload_bytes)
        .saturating_add(stats.vec_index_bytes.saturating_mul(4))
        .saturating_add(stats.lex_index_bytes.saturating_mul(4))
        .saturating_add(1_000_000);
    assert!(
        size <= live_allowance,
        "file retained derived snapshots: size={size}, allowance={live_allowance}, stats={stats:?}"
    );
    assert_eq!(stats.frame_count, count as u64);
    assert_eq!(stats.vector_count, count as u64);
    size
}

fn lexical_uris(mem: &mut Memvid, query: &str) -> Vec<String> {
    let mut uris = mem
        .search(SearchRequest {
            query: query.to_string(),
            top_k: 64,
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
            acl_enforcement_mode: memvid_core::types::AclEnforcementMode::Audit,
        })
        .unwrap()
        .hits
        .into_iter()
        .map(|hit| hit.uri)
        .collect::<Vec<_>>();
    uris.sort();
    uris
}

#[test]
fn repeated_commits_have_linear_physical_growth_with_and_without_reopen() {
    for reopen_each in [false, true] {
        let dir = TempDir::new().unwrap();
        let mut sizes = Vec::new();
        for count in [8, 16, 32] {
            let path = dir.path().join(format!("n-{count}.mv2"));
            sizes.push(build_commit_each(&path, count, reopen_each));
        }

        // Removing the fixed WAL/format overhead would make this closer to 2x.
        // 3x leaves room for index implementation changes while rejecting the
        // approximately 4x step produced by accumulated full snapshots.
        assert!(
            sizes[2] < sizes[1].saturating_mul(3),
            "growth is super-linear: {sizes:?} (reopen_each={reopen_each})"
        );
    }
}

#[test]
fn repeated_commits_match_batch_search_and_survive_update_delete_reopen() {
    let dir = TempDir::new().unwrap();
    let sequential_path = dir.path().join("sequential.mv2");
    let batch_path = dir.path().join("batch.mv2");

    build_commit_each(&sequential_path, 32, false);

    let mut batch = Memvid::create(&batch_path).unwrap();
    batch.enable_vec().unwrap();
    batch.set_vec_model("growth-regression-1024").unwrap();
    batch
        .begin_batch(PutManyOpts {
            enable_enrichment: false,
            ..Default::default()
        })
        .unwrap();
    for i in 0..32 {
        add(&mut batch, i);
    }
    batch.end_batch().unwrap();
    batch.commit().unwrap();

    let mut sequential = Memvid::open(&sequential_path).unwrap();
    assert_eq!(
        lexical_uris(&mut sequential, "group3"),
        lexical_uris(&mut batch, "group3")
    );
    assert_eq!(
        sequential
            .search_vec(&embedding(19), 1)
            .unwrap()
            .into_iter()
            .map(|hit| hit.frame_id)
            .collect::<Vec<_>>(),
        batch
            .search_vec(&embedding(19), 1)
            .unwrap()
            .into_iter()
            .map(|hit| hit.frame_id)
            .collect::<Vec<_>>()
    );

    let replacement = b"Replacement text with unique updatedneedle".to_vec();
    sequential
        .update_frame(
            7,
            Some(replacement),
            PutOptions::builder()
                .search_text("Replacement text with unique updatedneedle")
                .auto_tag(false)
                .extract_dates(false)
                .extract_triplets(false)
                .build(),
            Some(embedding(700)),
        )
        .unwrap();
    sequential.delete_frame(11).unwrap();
    sequential.commit().unwrap();
    drop(sequential);

    let mut reopened = Memvid::open(&sequential_path).unwrap();
    assert!(reopened.frame_by_id(7).unwrap().superseded_by.is_some());
    assert_eq!(
        reopened.frame_by_id(11).unwrap().status,
        FrameStatus::Deleted
    );
    assert_eq!(lexical_uris(&mut reopened, "updatedneedle").len(), 1);
    assert!(
        reopened
            .search_vec(&embedding(700), 1)
            .unwrap()
            .first()
            .is_some_and(|hit| hit.frame_id == 32)
    );
}

fn inflate_with_legacy_orphaned_tail(path: &Path, padding: usize) -> u64 {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    let mut header_bytes = [0; HEADER_SIZE];
    file.read_exact(&mut header_bytes).unwrap();
    let mut header = HeaderCodec::decode(&header_bytes).unwrap();

    file.seek(SeekFrom::Start(header.footer_offset)).unwrap();
    let mut toc_and_footer = Vec::new();
    file.read_to_end(&mut toc_and_footer).unwrap();
    let old_footer_offset = header.footer_offset;
    header.footer_offset = old_footer_offset + padding as u64;
    file.seek(SeekFrom::Start(old_footer_offset)).unwrap();
    file.write_all(&vec![0xA5; padding]).unwrap();
    file.write_all(&toc_and_footer).unwrap();
    HeaderCodec::write(&mut file, &header).unwrap();
    file.sync_all().unwrap();
    file.metadata().unwrap().len()
}

#[test]
fn next_commit_reads_and_reclaims_a_legacy_bloated_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("legacy-bloated.mv2");
    build_commit_each(&path, 12, false);
    let compact_size = File::open(&path).unwrap().metadata().unwrap().len();
    let bloated_size = inflate_with_legacy_orphaned_tail(&path, 3 * 1024 * 1024);
    assert!(bloated_size > compact_size + 3_000_000);

    let mut old_file = Memvid::open(&path).unwrap();
    assert_eq!(old_file.stats().unwrap().frame_count, 12);
    assert!(!old_file.search_vec(&embedding(3), 1).unwrap().is_empty());
    add(&mut old_file, 12);
    old_file.commit().unwrap();
    drop(old_file);

    let reclaimed_size = File::open(&path).unwrap().metadata().unwrap().len();
    assert!(
        reclaimed_size < bloated_size - 2 * 1024 * 1024,
        "legacy orphaned tail was not reclaimed: before={bloated_size}, after={reclaimed_size}"
    );
    let mut reopened = Memvid::open(&path).unwrap();
    assert_eq!(reopened.stats().unwrap().frame_count, 13);
    assert!(!lexical_uris(&mut reopened, "group2").is_empty());
    assert!(!reopened.search_vec(&embedding(12), 1).unwrap().is_empty());
}

#[test]
fn vacuum_preserves_nonempty_sketch_track_and_search_indexes() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("vacuum-sketch.mv2");
    build_commit_each(&path, 8, false);

    let expected = {
        let mut mem = Memvid::open(&path).unwrap();
        let expected = mem.sketches().get(3).cloned().expect("sketch for frame 3");
        assert!(
            mem.find_sketch_candidates(
                &document(3),
                Some(SketchSearchOptions {
                    hamming_threshold: 64,
                    max_candidates: 100,
                    min_score: 0.0,
                }),
            )
            .iter()
            .any(|candidate| candidate.frame_id == 3)
        );
        mem.vacuum().unwrap();
        expected
    };

    let mut reopened = Memvid::open_read_only(&path).unwrap();
    assert_eq!(reopened.stats().unwrap().frame_count, 8);
    assert_eq!(reopened.sketches().get(3), Some(&expected));
    assert!(
        reopened
            .find_sketch_candidates(
                &document(3),
                Some(SketchSearchOptions {
                    hamming_threshold: 64,
                    max_candidates: 100,
                    min_score: 0.0,
                }),
            )
            .iter()
            .any(|candidate| candidate.frame_id == 3)
    );
    assert!(!lexical_uris(&mut reopened, "group3").is_empty());
    assert_eq!(
        reopened.search_vec(&embedding(3), 1).unwrap()[0].frame_id,
        3
    );
}

#[test]
fn compact_delete_clears_manifest_for_an_explicitly_empty_sketch_track() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("empty-sketch.mv2");
    build_commit_each(&path, 4, false);

    {
        let mut mem = Memvid::open(&path).unwrap();
        assert!(mem.has_sketches());
        *mem.sketches_mut() = SketchTrack::default();
        mem.delete_frame(1).unwrap();
        mem.commit().unwrap();
    }

    let mut reopened = Memvid::open_read_only(&path).unwrap();
    assert!(!reopened.has_sketches());
    assert_eq!(reopened.sketch_stats().entry_count, 0);
    assert_eq!(
        reopened.frame_by_id(1).unwrap().status,
        FrameStatus::Deleted
    );
    assert!(!lexical_uris(&mut reopened, "group2").is_empty());
    assert_eq!(
        reopened.search_vec(&embedding(2), 1).unwrap()[0].frame_id,
        2
    );
}

#[test]
fn sketch_only_commits_replace_the_snapshot_in_bounded_space() {
    const DOCUMENTS: usize = 32;
    const UPDATES: usize = 12;

    let dir = TempDir::new().unwrap();
    let path = dir.path().join("sketch-only.mv2");
    let mut mem = Memvid::create(&path).unwrap();
    mem.enable_vec().unwrap();
    mem.set_vec_model("sketch-only-1024").unwrap();
    for i in 0..DOCUMENTS {
        add(&mut mem, i);
    }
    mem.commit().unwrap();

    *mem.sketches_mut() = SketchTrack::new(SketchVariant::Medium);
    for i in 0..DOCUMENTS {
        mem.insert_sketch(i as u64, &document(i), SketchVariant::Medium);
    }
    mem.commit().unwrap();
    let baseline_size = std::fs::metadata(&path).unwrap().len();
    let live_sketch_bytes = mem.sketch_stats().size_bytes;

    let mut expected = None;
    let mut sizes = Vec::new();
    for update in 0..UPDATES {
        let text = format!("latest sketch revision {update} unique sketch token");
        expected = Some(mem.insert_sketch(0, &text, SketchVariant::Medium));
        mem.commit().unwrap();
        sizes.push(std::fs::metadata(&path).unwrap().len());
    }
    let final_size = *sizes.last().unwrap();
    let allowance = baseline_size
        .saturating_add(live_sketch_bytes.saturating_mul(3))
        .saturating_add(16 * 1024);
    assert!(
        final_size <= allowance,
        "sketch-only snapshots accumulated: baseline={baseline_size}, final={final_size}, live_sketch={live_sketch_bytes}, sizes={sizes:?}"
    );
    println!(
        "sketch-only sizes: baseline={baseline_size}, final={final_size}, live_sketch={live_sketch_bytes}, series={sizes:?}"
    );
    assert_eq!(mem.sketches().get(0), expected.as_ref());
    assert_eq!(mem.search_vec(&embedding(19), 1).unwrap()[0].frame_id, 19);
    assert!(!lexical_uris(&mut mem, "group4").is_empty());
    drop(mem);

    let mut reopened = Memvid::open(&path).unwrap();
    assert_eq!(reopened.sketches().get(0), expected.as_ref());
    assert_eq!(reopened.sketch_stats().entry_count, DOCUMENTS as u64);
    assert_eq!(
        reopened.search_vec(&embedding(19), 1).unwrap()[0].frame_id,
        19
    );
    assert!(!lexical_uris(&mut reopened, "group4").is_empty());
}

#[cfg(feature = "parallel_segments")]
#[test]
fn sketch_only_commit_preserves_cold_parallel_vector_segments() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("parallel-vectors-sketch-only.mv2");

    let mut mem = Memvid::create(&path).unwrap();
    mem.enable_vec().unwrap();
    add(&mut mem, 0);
    add(&mut mem, 1);
    let mut build_opts = BuildOpts::default();
    build_opts.segment_tokens = 1;
    mem.commit_parallel(build_opts).unwrap();
    drop(mem);

    // Prove that the segment-backed vectors are initially searchable, but do
    // so in a separate instance. The writable instance below must not warm its
    // vector cache before the destructive compact rebuild.
    let mut reader = Memvid::open_read_only(&path).unwrap();
    assert_eq!(reader.search_vec(&embedding(0), 1).unwrap()[0].frame_id, 0);
    assert_eq!(reader.search_vec(&embedding(1), 1).unwrap()[0].frame_id, 1);
    drop(reader);

    let mut writer = Memvid::open(&path).unwrap();
    writer.insert_sketch(0, "cold segment vector preservation", SketchVariant::Small);
    writer.commit().unwrap();
    drop(writer);

    let mut reopened = Memvid::open_read_only(&path).unwrap();
    assert_eq!(
        reopened.search_vec(&embedding(0), 1).unwrap()[0].frame_id,
        0
    );
    assert_eq!(
        reopened.search_vec(&embedding(1), 1).unwrap()[0].frame_id,
        1
    );
    assert!(reopened.sketches().get(0).is_some());
}

#[cfg(feature = "parallel_segments")]
#[test]
fn parallel_fallback_clears_empty_sketch_manifest() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("parallel-empty-sketch.mv2");
    build_commit_each(&path, 2, false);

    let mut mem = Memvid::open(&path).unwrap();
    assert!(mem.has_sketches());
    *mem.sketches_mut() = SketchTrack::default();
    mem.delete_frame(0).unwrap();
    mem.commit_parallel(BuildOpts::default()).unwrap();
    drop(mem);

    let mut reopened = Memvid::open_read_only(&path).unwrap();
    assert!(!reopened.has_sketches());
    assert_eq!(reopened.sketch_stats().entry_count, 0);
    assert_eq!(
        reopened.frame_by_id(0).unwrap().status,
        FrameStatus::Deleted
    );
    assert_eq!(
        reopened.search_vec(&embedding(1), 1).unwrap()[0].frame_id,
        1
    );
    assert_eq!(
        lexical_uris(&mut reopened, "group1"),
        vec!["mv2://growth/1"]
    );
}

#[cfg(feature = "parallel_segments")]
fn overlap_payload() -> Vec<u8> {
    let mut state = 17_u64;
    (0..16 * 1024)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
}

#[cfg(feature = "parallel_segments")]
fn create_parallel_vector_file(path: &Path) {
    let mut mem = Memvid::create(path).unwrap();
    mem.enable_vec().unwrap();
    for frame_id in 0..2 {
        let text = document(frame_id);
        mem.put_with_embedding_and_options(
            text.as_bytes(),
            overlap_embedding(frame_id),
            options(frame_id),
        )
        .unwrap();
    }
    mem.insert_sketch(0, "persisted recovery sketch", SketchVariant::Small);
    let mut build_opts = BuildOpts::default();
    build_opts.segment_tokens = 1;
    mem.commit_parallel(build_opts).unwrap();
}

#[cfg(feature = "parallel_segments")]
fn overlap_embedding(frame_id: usize) -> Vec<f32> {
    let mut embedding = vec![0.0; 4];
    embedding[frame_id] = 1.0;
    embedding
}

#[cfg(feature = "parallel_segments")]
fn enqueue_overlapping_payload(mem: &mut Memvid) {
    mem.begin_batch(PutManyOpts {
        wal_pre_size_bytes: 0,
        disable_auto_checkpoint: true,
        skip_sync: false,
        ..Default::default()
    })
    .unwrap();
    mem.put_with_embedding_and_options(
        &overlap_payload(),
        overlap_embedding(2),
        PutOptions::builder()
            .uri("mv2://growth/2")
            .search_text("new binary overlap payload")
            .auto_tag(false)
            .extract_dates(false)
            .extract_triplets(false)
            .build(),
    )
    .unwrap();
    mem.end_batch().unwrap();
}

#[cfg(feature = "parallel_segments")]
fn assert_three_vectors_and_sketch(mem: &mut Memvid) {
    for frame_id in 0..3 {
        assert_eq!(
            mem.search_vec(&overlap_embedding(frame_id), 1).unwrap()[0].frame_id,
            frame_id as u64
        );
        assert!(
            mem.frame_by_uri(&format!("mv2://growth/{frame_id}"))
                .is_ok()
        );
    }
    assert!(mem.sketches().get(0).is_some());
}

#[cfg(feature = "parallel_segments")]
fn append_over_parallel_segments_after_reopen(warm_vector_cache: bool) {
    let dir = TempDir::new().unwrap();
    let path = dir
        .path()
        .join(format!("parallel-overlap-{warm_vector_cache}.mv2"));
    create_parallel_vector_file(&path);

    let mut reader = Memvid::open_read_only(&path).unwrap();
    assert_eq!(
        reader.search_vec(&overlap_embedding(0), 1).unwrap()[0].frame_id,
        0
    );
    assert_eq!(
        reader.search_vec(&overlap_embedding(1), 1).unwrap()[0].frame_id,
        1
    );
    drop(reader);

    let mut writer = Memvid::open(&path).unwrap();
    if warm_vector_cache {
        assert_eq!(
            writer.search_vec(&overlap_embedding(1), 1).unwrap()[0].frame_id,
            1
        );
    }
    enqueue_overlapping_payload(&mut writer);
    writer.commit().unwrap();
    assert!(
        writer
            .frame_by_uri("mv2://growth/2")
            .unwrap()
            .payload_length
            > 12 * 1024,
        "the deterministic payload must remain large enough to overwrite the old derived tail"
    );
    drop(writer);

    let mut reopened = Memvid::open_read_only(&path).unwrap();
    assert_three_vectors_and_sketch(&mut reopened);
}

#[cfg(feature = "parallel_segments")]
#[test]
fn payload_overlap_preserves_cold_parallel_vector_segments() {
    append_over_parallel_segments_after_reopen(false);
}

#[cfg(feature = "parallel_segments")]
#[test]
fn payload_overlap_preserves_warm_parallel_vector_segments() {
    append_over_parallel_segments_after_reopen(true);
}

#[cfg(feature = "parallel_segments")]
#[test]
fn wal_recovery_materializes_vectors_before_overlapping_payload_write() {
    let dir = TempDir::new().unwrap();
    let source_path = dir.path().join("parallel-pending-wal-source.mv2");
    let recovery_path = dir.path().join("parallel-pending-wal-recovery.mv2");
    create_parallel_vector_file(&source_path);

    let mut writer = Memvid::open(&source_path).unwrap();
    enqueue_overlapping_payload(&mut writer);
    std::fs::copy(&source_path, &recovery_path).unwrap();

    // Do not let Drop retry a normal commit and replace the pending-WAL state
    // that the copied file is intended to exercise.
    std::mem::forget(writer);

    // The committed state in the copied file is intact before recovery; a
    // read-only open deliberately leaves the pending WAL unapplied.
    let pending = Memvid::open_read_only(&recovery_path).unwrap();
    assert!(pending.sketches().get(0).is_some());
    drop(pending);

    let mut recovered = Memvid::open(&recovery_path).unwrap();
    assert!(
        recovered
            .frame_by_uri("mv2://growth/2")
            .unwrap()
            .payload_length
            > 12 * 1024
    );
    assert_three_vectors_and_sketch(&mut recovered);
    drop(recovered);

    let mut reopened = Memvid::open_read_only(&recovery_path).unwrap();
    assert_three_vectors_and_sketch(&mut reopened);
}

#[cfg(feature = "parallel_segments")]
fn parallel_embedding(frame_id: usize) -> Vec<f32> {
    let mut embedding = vec![0.0; 64];
    embedding[frame_id % 64] = 1.0;
    embedding
}

#[cfg(feature = "parallel_segments")]
fn parallel_put(mem: &mut Memvid, frame_id: usize) {
    let text = format!("parallel cache document {frame_id}");
    mem.put_with_embedding_and_options(
        text.as_bytes(),
        parallel_embedding(frame_id),
        PutOptions::builder()
            .uri(format!("mv2://parallel/{frame_id}"))
            .search_text(text.clone())
            .auto_tag(false)
            .extract_dates(false)
            .extract_triplets(false)
            .build(),
    )
    .unwrap();
}

#[cfg(feature = "parallel_segments")]
fn parallel_commit(mem: &mut Memvid) {
    let mut opts = BuildOpts::default();
    opts.threads = 1;
    opts.segment_tokens = 1;
    mem.commit_parallel(opts).unwrap();
}

#[cfg(feature = "parallel_segments")]
fn assert_parallel_hit(mem: &mut Memvid, frame_id: usize) {
    let hit = mem.search_vec(&parallel_embedding(frame_id), 1).unwrap()[0].clone();
    assert_eq!(hit.frame_id, frame_id as u64);
    assert_eq!(hit.distance, 0.0);
}

#[cfg(feature = "parallel_segments")]
fn append_parallel_after_reopen(warm_cache: bool) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("parallel-cache-{warm_cache}.mv2"));
    let mut mem = Memvid::create(&path).unwrap();
    mem.enable_vec().unwrap();
    parallel_put(&mut mem, 0);
    parallel_commit(&mut mem);
    drop(mem);

    let mut writer = Memvid::open(&path).unwrap();
    if warm_cache {
        assert_parallel_hit(&mut writer, 0);
    }
    parallel_put(&mut writer, 1);
    parallel_commit(&mut writer);
    assert_parallel_hit(&mut writer, 1);
    drop(writer);

    let mut reopened = Memvid::open_read_only(&path).unwrap();
    assert_parallel_hit(&mut reopened, 0);
    assert_parallel_hit(&mut reopened, 1);
}

#[cfg(feature = "parallel_segments")]
#[test]
fn parallel_commit_refreshes_cold_vector_cache() {
    append_parallel_after_reopen(false);
}

#[cfg(feature = "parallel_segments")]
#[test]
fn parallel_commit_refreshes_warm_vector_cache() {
    append_parallel_after_reopen(true);
}

#[cfg(feature = "parallel_segments")]
#[test]
fn sequential_parallel_commits_refresh_vector_cache() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("parallel-cache-sequential.mv2");
    let mut mem = Memvid::create(&path).unwrap();
    mem.enable_vec().unwrap();
    for frame_id in 0..4 {
        parallel_put(&mut mem, frame_id);
        parallel_commit(&mut mem);
        assert_parallel_hit(&mut mem, frame_id);
    }
    drop(mem);

    let mut reopened = Memvid::open_read_only(&path).unwrap();
    for frame_id in 0..4 {
        assert_parallel_hit(&mut reopened, frame_id);
    }
}

#[cfg(feature = "parallel_segments")]
fn assert_skip_parallel_contents(mem: &mut Memvid) {
    for (frame_id, expected) in [
        (0_u64, b"first vector".as_slice()),
        (1, b"no embedding".as_slice()),
        (2, b"second vector".as_slice()),
    ] {
        assert_eq!(mem.frame_canonical_payload(frame_id).unwrap(), expected);
    }

    let old = mem.search_vec(&[1.0, 0.0, 0.0, 0.0], 3).unwrap();
    assert_eq!(old[0].frame_id, 0);
    assert_eq!(old[0].distance, 0.0);
    let new = mem.search_vec(&[0.0, 1.0, 0.0, 0.0], 3).unwrap();
    assert_eq!(new[0].frame_id, 2);
    assert_eq!(new[0].distance, 0.0);
}

#[cfg(feature = "parallel_segments")]
#[test]
fn skip_then_parallel_commit_persists_cache_only_vectors() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("skip-then-parallel.mv2");
    let mut mem = Memvid::create(&path).unwrap();
    mem.enable_vec().unwrap();
    mem.put_with_embedding_and_options(
        b"first vector",
        vec![1.0, 0.0, 0.0, 0.0],
        PutOptions::builder()
            .uri("mv2://skip-parallel/0")
            .search_text("first vector")
            .auto_tag(false)
            .extract_dates(false)
            .extract_triplets(false)
            .build(),
    )
    .unwrap();
    mem.commit().unwrap();

    mem.put_bytes_with_options(
        b"no embedding",
        PutOptions::builder()
            .uri("mv2://skip-parallel/1")
            .search_text("no embedding")
            .auto_tag(false)
            .extract_dates(false)
            .extract_triplets(false)
            .build(),
    )
    .unwrap();
    mem.commit_skip_indexes().unwrap();

    mem.put_with_embedding_and_options(
        b"second vector",
        vec![0.0, 1.0, 0.0, 0.0],
        PutOptions::builder()
            .uri("mv2://skip-parallel/2")
            .search_text("second vector")
            .auto_tag(false)
            .extract_dates(false)
            .extract_triplets(false)
            .build(),
    )
    .unwrap();
    parallel_commit(&mut mem);
    assert_skip_parallel_contents(&mut mem);

    let mut reopened_before_finalize = Memvid::open_read_only(&path).unwrap();
    assert_skip_parallel_contents(&mut reopened_before_finalize);
    drop(reopened_before_finalize);

    mem.finalize_indexes().unwrap();
    assert_skip_parallel_contents(&mut mem);
    drop(mem);

    let mut reopened = Memvid::open_read_only(&path).unwrap();
    assert_skip_parallel_contents(&mut reopened);
}

#[cfg(feature = "parallel_segments")]
fn assert_exact_vector(mem: &mut Memvid, query: &[f32], frame_id: u64) {
    let hit = mem.search_vec(query, 3).unwrap()[0].clone();
    assert_eq!(hit.frame_id, frame_id);
    assert_eq!(hit.distance, 0.0);
}

#[cfg(feature = "parallel_segments")]
#[test]
fn pending_manifest_vectors_survive_parallel_commit_and_finalize() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("pending-manifest-parallel.mv2");
    let mut mem = Memvid::create(&path).unwrap();
    mem.enable_vec().unwrap();
    mem.put_bytes_with_options(b"pending embedding frame", options(0))
        .unwrap();
    mem.commit_skip_indexes().unwrap();
    mem.add_embeddings(vec![(0, vec![1.0, 0.0, 0.0, 0.0])])
        .unwrap();
    mem.put_with_embedding_and_options(
        b"parallel embedding frame",
        vec![0.0, 1.0, 0.0, 0.0],
        options(1),
    )
    .unwrap();
    parallel_commit(&mut mem);

    assert_exact_vector(&mut mem, &[1.0, 0.0, 0.0, 0.0], 0);
    assert_exact_vector(&mut mem, &[0.0, 1.0, 0.0, 0.0], 1);
    let mut reopened_before_finalize = Memvid::open_read_only(&path).unwrap();
    assert_exact_vector(&mut reopened_before_finalize, &[1.0, 0.0, 0.0, 0.0], 0);
    assert_exact_vector(&mut reopened_before_finalize, &[0.0, 1.0, 0.0, 0.0], 1);
    drop(reopened_before_finalize);

    mem.finalize_indexes().unwrap();
    assert_exact_vector(&mut mem, &[1.0, 0.0, 0.0, 0.0], 0);
    assert_exact_vector(&mut mem, &[0.0, 1.0, 0.0, 0.0], 1);
    drop(mem);

    let mut reopened = Memvid::open_read_only(&path).unwrap();
    assert_exact_vector(&mut reopened, &[1.0, 0.0, 0.0, 0.0], 0);
    assert_exact_vector(&mut reopened, &[0.0, 1.0, 0.0, 0.0], 1);
    assert_eq!(
        reopened.frame_canonical_payload(0).unwrap(),
        b"pending embedding frame"
    );
    assert_eq!(
        reopened.frame_canonical_payload(1).unwrap(),
        b"parallel embedding frame"
    );
}

#[cfg(feature = "parallel_segments")]
#[test]
fn pending_replacement_overrides_segments_across_commit_modes() {
    for warm_cache in [false, true] {
        for parallel in [false, true] {
            for append in [false, true] {
                let dir = TempDir::new().unwrap();
                let path = dir
                    .path()
                    .join(format!("replacement-{warm_cache}-{parallel}-{append}.mv2"));
                let mut mem = Memvid::create(&path).unwrap();
                mem.enable_vec().unwrap();
                mem.put_with_embedding_and_options(b"replace me", vec![1.0, 0.0, 0.0], options(0))
                    .unwrap();
                mem.put_with_embedding_and_options(b"untouched", vec![0.0, 0.0, 1.0], options(1))
                    .unwrap();
                parallel_commit(&mut mem);
                drop(mem);

                let mut mem = Memvid::open(&path).unwrap();
                if warm_cache {
                    assert_exact_vector(&mut mem, &[1.0, 0.0, 0.0], 0);
                }
                mem.add_embeddings(vec![(0, vec![0.0, 1.0, 0.0])]).unwrap();
                if append {
                    mem.put_with_embedding_and_options(
                        b"appended",
                        vec![1.0, 1.0, 0.0],
                        options(2),
                    )
                    .unwrap();
                }
                if parallel {
                    parallel_commit(&mut mem);
                } else {
                    mem.commit().unwrap();
                }
                assert_exact_vector(&mut mem, &[0.0, 1.0, 0.0], 0);
                assert_exact_vector(&mut mem, &[0.0, 0.0, 1.0], 1);
                if append {
                    assert_exact_vector(&mut mem, &[1.0, 1.0, 0.0], 2);
                }
                drop(mem);

                let mut reopened = Memvid::open_read_only(&path).unwrap();
                assert_exact_vector(&mut reopened, &[0.0, 1.0, 0.0], 0);
                assert_exact_vector(&mut reopened, &[0.0, 0.0, 1.0], 1);
                if append {
                    assert_exact_vector(&mut reopened, &[1.0, 1.0, 0.0], 2);
                }
            }
        }
    }
}

#[cfg(all(feature = "parallel_segments", feature = "replay"))]
#[test]
fn replay_adjacent_pending_replacement_survives_parallel_commit() {
    for initial_parallel in [false, true] {
        let dir = TempDir::new().unwrap();
        let path = dir
            .path()
            .join(format!("replay-pending-{initial_parallel}.mv2"));
        let mut mem = Memvid::create(&path).unwrap();
        mem.enable_vec().unwrap();
        mem.put_with_embedding_and_options(b"original", vec![1.0, 0.0, 0.0], options(0))
            .unwrap();
        if initial_parallel {
            parallel_commit(&mut mem);
        } else {
            mem.commit().unwrap();
        }
        mem.start_session(Some("pending vectors".to_string()), None)
            .unwrap();
        mem.end_session().unwrap();
        mem.save_replay_sessions().unwrap();
        mem.add_embeddings(vec![(0, vec![0.0, 1.0, 0.0])]).unwrap();
        mem.put_with_embedding_and_options(b"new", vec![0.0, 0.0, 1.0], options(1))
            .unwrap();
        parallel_commit(&mut mem);
        assert_exact_vector(&mut mem, &[0.0, 1.0, 0.0], 0);
        assert_exact_vector(&mut mem, &[0.0, 0.0, 1.0], 1);
        mem.finalize_indexes().unwrap();
        drop(mem);

        let mut reopened = Memvid::open_read_only(&path).unwrap();
        assert_exact_vector(&mut reopened, &[0.0, 1.0, 0.0], 0);
        assert_exact_vector(&mut reopened, &[0.0, 0.0, 1.0], 1);
        reopened.load_replay_sessions().unwrap();
        assert_eq!(reopened.list_sessions().len(), 1);
    }
}

#[cfg(feature = "parallel_segments")]
fn decoded_sketch_snapshots(path: &Path) -> Vec<(u64, u64)> {
    let bytes = std::fs::read(path).unwrap();
    let mut snapshots = Vec::new();
    for (offset, window) in bytes.windows(4).enumerate() {
        if window != b"MVSK" || offset + 24 > bytes.len() {
            continue;
        }
        let entry_size = u16::from_le_bytes(bytes[offset + 6..offset + 8].try_into().unwrap());
        let entry_count = u64::from_le_bytes(bytes[offset + 8..offset + 16].try_into().unwrap());
        let length = 24_u64.saturating_add(u64::from(entry_size).saturating_mul(entry_count));
        let mut cursor = std::io::Cursor::new(&bytes);
        if read_sketch_track(&mut cursor, offset as u64, length).is_ok() {
            snapshots.push((entry_count, length));
        }
    }
    snapshots
}

#[cfg(feature = "parallel_segments")]
#[test]
fn parallel_to_regular_commit_does_not_pin_old_sketch_snapshots() {
    for document_count in [8_usize, 16, 32] {
        let dir = TempDir::new().unwrap();
        let path = dir
            .path()
            .join(format!("parallel-snapshot-growth-{document_count}.mv2"));
        let mut mem = Memvid::create(&path).unwrap();
        mem.enable_vec().unwrap();
        for frame_id in 0..document_count {
            parallel_put(&mut mem, frame_id);
            mem.insert_sketch(
                frame_id as u64,
                "sketch words unique tokens",
                SketchVariant::Small,
            );
            parallel_commit(&mut mem);
        }
        let before = std::fs::metadata(&path).unwrap().len();
        mem.insert_sketch(0, "changed current sketch", SketchVariant::Small);
        mem.commit().unwrap();
        let after = std::fs::metadata(&path).unwrap().len();
        drop(mem);

        let snapshots = decoded_sketch_snapshots(&path);
        assert_eq!(
            snapshots,
            vec![(document_count as u64, 24 + 32 * document_count as u64)],
            "obsolete full sketch generations remained for N={document_count}"
        );

        let mut reopened = Memvid::open_read_only(&path).unwrap();
        assert_eq!(reopened.stats().unwrap().frame_count, document_count as u64);
        assert_eq!(reopened.sketch_stats().entry_count, document_count as u64);
        for frame_id in [0, document_count / 2, document_count - 1] {
            assert_parallel_hit(&mut reopened, frame_id);
            assert!(
                reopened
                    .frame_by_uri(&format!("mv2://parallel/{frame_id}"))
                    .is_ok()
            );
        }
        let stats = reopened.stats().unwrap();
        let bounded_size = stats
            .wal_bytes
            .saturating_add(stats.payload_bytes)
            .saturating_add(stats.vec_index_bytes.saturating_mul(3))
            .saturating_add(stats.lex_index_bytes.saturating_mul(3))
            .saturating_add(reopened.sketch_stats().size_bytes.saturating_mul(3))
            .saturating_add(128 * 1024);
        assert!(
            after <= bounded_size,
            "parallel snapshots remained pinned: N={document_count}, before={before}, after={after}, bound={bounded_size}, stats={stats:?}"
        );
        println!(
            "parallel compact N={document_count}: before={before}, after={after}, snapshots={snapshots:?}, bound={bounded_size}"
        );
    }
}
