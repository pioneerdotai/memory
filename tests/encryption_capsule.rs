//! Encryption capsule tests (.mv2e).

#[cfg(feature = "encryption")]
use memvid_core::encryption::{EncryptionError, Mv2eHeader, lock_file, unlock_file};
#[cfg(feature = "encryption")]
use memvid_core::{Memvid, PutOptions};

#[cfg(feature = "encryption")]
use std::fs::read;
#[cfg(feature = "encryption")]
use tempfile::TempDir;

#[test]
#[cfg(feature = "encryption")]
fn mv2e_header_roundtrip() {
    let header = Mv2eHeader {
        magic: memvid_core::encryption::MV2E_MAGIC,
        version: memvid_core::encryption::MV2E_VERSION,
        kdf_algorithm: memvid_core::encryption::KdfAlgorithm::Argon2id,
        cipher_algorithm: memvid_core::encryption::CipherAlgorithm::Aes256Gcm,
        salt: [1u8; memvid_core::encryption::SALT_SIZE],
        nonce: [2u8; memvid_core::encryption::NONCE_SIZE],
        original_size: 1024,
        reserved: [0u8; 4],
    };

    let encoded = header.encode();
    let decoded = Mv2eHeader::decode(&encoded).expect("decode");

    assert_eq!(decoded.magic, header.magic);
    assert_eq!(decoded.version, header.version);
    assert_eq!(decoded.salt, header.salt);
    assert_eq!(decoded.nonce, header.nonce);
    assert_eq!(decoded.original_size, header.original_size);
}

#[test]
#[cfg(feature = "encryption")]
fn lock_unlock_roundtrip_preserves_bytes() {
    let dir = TempDir::new().expect("tmp");
    let mv2_path = dir.path().join("test.mv2");
    let mv2e_path = dir.path().join("test.mv2e");
    let restored_path = dir.path().join("restored.mv2");

    {
        let mut mem = Memvid::create(&mv2_path).expect("create");
        mem.put_bytes_with_options(
            b"hello",
            PutOptions {
                title: Some("doc".to_string()),
                labels: vec!["note".to_string()],
                ..Default::default()
            },
        )
        .expect("put");
        mem.commit().expect("commit");
    }

    lock_file(&mv2_path, Some(mv2e_path.as_path()), b"test-password-123").expect("lock");
    unlock_file(
        &mv2e_path,
        Some(restored_path.as_path()),
        b"test-password-123",
    )
    .expect("unlock");

    let original = read(&mv2_path).expect("read original");
    let restored = read(&restored_path).expect("read restored");
    assert_eq!(original, restored);
}

#[test]
#[cfg(feature = "encryption")]
fn wrong_password_fails() {
    let dir = TempDir::new().expect("tmp");
    let mv2_path = dir.path().join("test.mv2");
    let mv2e_path = dir.path().join("test.mv2e");

    {
        let mut mem = Memvid::create(&mv2_path).expect("create");
        mem.put_bytes(b"hello").expect("put");
        mem.commit().expect("commit");
    }

    lock_file(&mv2_path, Some(mv2e_path.as_path()), b"password-a").expect("lock");
    let err = unlock_file(&mv2e_path, None, b"password-b").expect_err("should fail");
    assert!(matches!(err, EncryptionError::Decryption { .. }));
}

/// Test streaming encryption with a large file (>1MB to trigger multiple chunks)
/// Note: The mv2 file format includes a 64MB WAL by default, so even small content
/// creates large files. This test focuses on verifying the streaming format works.
#[test]
#[cfg(feature = "encryption")]
fn streaming_encryption_large_file() {
    let dir = TempDir::new().expect("tmp");
    let mv2_path = dir.path().join("large.mv2");
    let mv2e_path = dir.path().join("large.mv2e");
    let restored_path = dir.path().join("large_restored.mv2");

    // Create a memory file with modest content (the file will be large due to WAL)
    {
        let mut mem = Memvid::create(&mv2_path).expect("create");

        // Add 5 entries - this should create a file >1MB due to WAL overhead
        for i in 0..5 {
            let content = format!("Entry {} with content: {}", i, "x".repeat(10_000));
            mem.put_bytes_with_options(
                content.as_bytes(),
                PutOptions {
                    title: Some(format!("Entry {}", i)),
                    labels: vec!["test".to_string()],
                    ..Default::default()
                },
            )
            .expect("put");
        }
        mem.commit().expect("commit");
    }

    // The file should be >1MB due to embedded WAL
    let original_size = std::fs::metadata(&mv2_path).expect("metadata").len();
    assert!(
        original_size > 1_000_000,
        "File should be >1MB, got {} bytes",
        original_size
    );
    println!(
        "Created test file: {} bytes ({:.2} MB)",
        original_size,
        original_size as f64 / 1_000_000.0
    );

    // Encrypt using streaming
    lock_file(
        &mv2_path,
        Some(mv2e_path.as_path()),
        b"streaming-test-password",
    )
    .expect("lock");

    // Verify encrypted file has streaming marker (reserved[0] == 0x01)
    let encrypted_bytes = read(&mv2e_path).expect("read encrypted");
    let header_bytes: [u8; Mv2eHeader::SIZE] = encrypted_bytes[..Mv2eHeader::SIZE]
        .try_into()
        .expect("slice to array");
    let header = Mv2eHeader::decode(&header_bytes).expect("decode header");
    assert_eq!(
        header.reserved[0], 0x01,
        "Should use streaming format (reserved[0] == 0x01)"
    );
    println!(
        "Encrypted file: {} bytes, streaming format confirmed",
        encrypted_bytes.len()
    );

    // Decrypt
    unlock_file(
        &mv2e_path,
        Some(restored_path.as_path()),
        b"streaming-test-password",
    )
    .expect("unlock");

    // Verify content matches
    let original = read(&mv2_path).expect("read original");
    let restored = read(&restored_path).expect("read restored");
    assert_eq!(original.len(), restored.len(), "Size mismatch");
    assert_eq!(original, restored, "Content mismatch");
    println!(
        "Decryption successful, {} bytes restored correctly",
        restored.len()
    );

    // Verify the restored file is valid and readable
    let mem = Memvid::open(&restored_path).expect("open restored");
    let stats = mem.stats().expect("stats");
    assert!(
        stats.frame_count >= 5,
        "Should have at least 5 frames, got {}",
        stats.frame_count
    );
    println!("Restored memory verified: {} frames", stats.frame_count);
}

/// Test that wrong password still fails with streaming format
#[test]
#[cfg(feature = "encryption")]
fn wrong_password_fails_streaming() {
    let dir = TempDir::new().expect("tmp");
    let mv2_path = dir.path().join("test_stream.mv2");
    let mv2e_path = dir.path().join("test_stream.mv2e");

    // Create a file (will be >1MB due to WAL overhead)
    {
        let mut mem = Memvid::create(&mv2_path).expect("create");
        for i in 0..3 {
            let content = format!("Entry {} {}", i, "data".repeat(10_000));
            mem.put_bytes(content.as_bytes()).expect("put");
        }
        mem.commit().expect("commit");
    }

    lock_file(&mv2_path, Some(mv2e_path.as_path()), b"correct-password").expect("lock");

    // Verify streaming format (files >1MB use streaming)
    let encrypted = read(&mv2e_path).expect("read");
    let header_bytes: [u8; Mv2eHeader::SIZE] = encrypted[..Mv2eHeader::SIZE]
        .try_into()
        .expect("slice to array");
    let header = Mv2eHeader::decode(&header_bytes).expect("decode");
    assert_eq!(header.reserved[0], 0x01, "Should use streaming format");

    // Wrong password should fail
    let err = unlock_file(&mv2e_path, None, b"wrong-password").expect_err("should fail");
    assert!(
        matches!(err, EncryptionError::Decryption { .. }),
        "Expected Decryption error, got {:?}",
        err
    );
    println!("Wrong password correctly rejected for streaming format");
}

/*
    This test verifies two things:
    1. Legacy format marker exists (reserved[0] == 0x00)
    2. Decryption works (new code can decrypt old format files)
*/
#[test]
#[cfg(feature = "encryption")]
fn decrypt_legacy_format_with_new_code() {
    use std::path::PathBuf;

    let mut fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fixture_dir.push("tests/fixtures");

    let original_mv2 = fixture_dir.join("legacy_test.mv2");
    let original_mv2e = fixture_dir.join("legacy_test.mv2e");

    let encrypted = read(&original_mv2e).expect("read");
    let header_bytes: [u8; Mv2eHeader::SIZE] = encrypted[..Mv2eHeader::SIZE]
        .try_into()
        .expect("header bytes");
    let header = Mv2eHeader::decode(&header_bytes).expect("decode");
    assert_eq!(header.reserved[0], 0x00, "should be legacy format");

    let dir = TempDir::new().expect("temp");
    let decrypted_path = dir.path().join("decrypted.mv2");

    unlock_file(original_mv2e, Some(decrypted_path.as_ref()), b"legacy-password").expect("unlock");

    let original = read(&original_mv2).expect("original");
    let decrypted = read(&decrypted_path).expect("decrypted");
    assert_eq!(original, decrypted);
}



/*
    This test verifies two things:
    1. Legacy format marker exists (reserved[0] == 0x00)
    2. Decryption works (new code can decrypt old format files)
*/
#[test]
#[cfg(feature = "encryption")]
#[ignore]
fn auto_detection_chooses_correct_decoder() {
    // TODO: Verify unlock_file dispatches correctly based on reserved[0]
    todo!()
}

#[test]
#[cfg(feature = "encryption")]
#[ignore]
fn corrupted_chunk_fails_gracefully() {
    // TODO: Corrupt a chunk in the middle, verify proper error
    todo!()
}

#[test]
#[cfg(feature = "encryption")]
#[ignore]
fn truncated_file_fails_gracefully() {
    // TODO: Truncate encrypted file mid-chunk, verify proper error
    todo!()
}

#[test]
#[cfg(feature = "encryption")]
#[ignore]
fn empty_mv2_file_encryption() {
    // TODO: Test encrypting an empty .mv2 file
    todo!()
}

#[test]
#[cfg(feature = "encryption")]
#[ignore]
fn exact_chunk_boundary_file() {
    // TODO: Test file size that's exactly N * CHUNK_SIZE (1MB)
    todo!()
}

#[test]
#[cfg(feature = "encryption")]
#[ignore]
fn multiple_encrypt_decrypt_cycles() {
    // TODO: Encrypt, decrypt, modify, encrypt again - verify no corruption
    todo!()
}

#[test]
#[cfg(feature = "encryption")]
#[ignore]
fn concurrent_encryption_different_files() {
    // TODO: Test encrypting multiple files in parallel
    todo!()
}

#[test]
#[cfg(feature = "encryption")]
#[ignore]
fn legacy_file_upgrade_on_reencrypt() {
    // TODO: Simulate legacy file upgrade flow:
    // 1. Create a .mv2e file with reserved[0] = 0x00 (legacy one-shot format)
    // 2. Decrypt it using unlock_file (should use oneshot path)
    // 3. Modify the .mv2 file (add some data)
    // 4. Re-encrypt using lock_file
    // 5. Verify the new .mv2e has reserved[0] = 0x01 (streaming format)
    // 6. Verify the content is still correct after round-trip
    todo!()
}
