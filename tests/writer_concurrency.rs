use memvid_core::{FileLock, Memvid, PutOptions};
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

fn options(name: &str) -> PutOptions {
    PutOptions::builder()
        .uri(format!("mv2://writer/{name}"))
        .search_text(name)
        .auto_tag(false)
        .extract_dates(false)
        .extract_triplets(false)
        .instant_index(false)
        .extraction_budget_ms(0)
        .build()
}

fn append(path: &Path, name: &str, vector: Vec<f32>) -> Result<(), String> {
    let mut mem = Memvid::open(path).map_err(|error| error.to_string())?;
    mem.put_with_embedding_and_options(name.as_bytes(), vector, options(name))
        .map_err(|error| error.to_string())?;
    mem.commit().map_err(|error| error.to_string())
}

fn assert_exclusive_lock_contended(path: &Path) {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .expect("open lock probe");
    assert!(
        FileLock::try_acquire(&file, path)
            .expect("probe destination lock")
            .is_none(),
        "active writer must physically exclude another exclusive lock"
    );
}

fn assert_documents(path: &Path, expected: &[(&str, &[f32])]) {
    let mut mem = Memvid::open_read_only(path).expect("reopen committed capsule");
    for (name, vector) in expected {
        let uri = format!("mv2://writer/{name}");
        let frame = mem.frame_by_uri(&uri).expect("committed URI must exist");
        assert_eq!(
            mem.frame_canonical_payload(frame.id)
                .expect("committed payload"),
            name.as_bytes()
        );
        let nearest = mem
            .search_vec(vector, 1)
            .expect("search committed vector")
            .into_iter()
            .next()
            .expect("vector result");
        assert_eq!(nearest.frame_id, frame.id, "vector for {uri} must survive");
    }
}

#[test]
fn writers_before_and_after_replacement_serialize_and_preserve_all_commits() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("writers.mv2");
    let mut first = Memvid::create(&path).expect("create capsule");
    first.enable_vec().expect("enable vectors");
    first
        .put_with_embedding_and_options(b"seed", vec![1.0, 0.0, 0.0, 0.0], options("seed"))
        .expect("put seed");
    first.commit().expect("commit seed");

    let (started_tx, started_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    let before_path = path.clone();
    let before = std::thread::spawn(move || {
        assert_exclusive_lock_contended(&before_path);
        started_tx.send(()).expect("signal old inode contention");
        done_tx
            .send(append(&before_path, "before", vec![0.0, 0.0, 1.0, 0.0]))
            .expect("send writer result");
    });
    started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("writer started");
    assert!(
        done_rx.recv_timeout(Duration::from_millis(150)).is_err(),
        "second writer must not acquire while first handle is alive"
    );

    first
        .put_with_embedding_and_options(b"alpha", vec![0.0, 1.0, 0.0, 0.0], options("alpha"))
        .expect("put alpha");
    first.commit().expect("second commit on first handle");
    assert!(
        done_rx.recv_timeout(Duration::from_millis(150)).is_err(),
        "writer waiting on the replaced inode must retry and remain excluded"
    );

    let (after_started_tx, after_started_rx) = mpsc::channel();
    let (after_done_tx, after_done_rx) = mpsc::channel();
    let after_path = path.clone();
    let after = std::thread::spawn(move || {
        assert_exclusive_lock_contended(&after_path);
        after_started_tx
            .send(())
            .expect("signal current inode contention");
        after_done_tx
            .send(append(&after_path, "after", vec![0.0, 0.0, 0.0, 1.0]))
            .expect("send writer result");
    });
    after_started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("post-replacement writer started");
    assert!(
        after_done_rx
            .recv_timeout(Duration::from_millis(150))
            .is_err(),
        "writer opened after replacement must remain excluded"
    );

    drop(first);
    done_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("pre-replacement waiter completed")
        .expect("pre-replacement waiter committed");
    after_done_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("post-replacement waiter completed")
        .expect("post-replacement waiter committed");
    before.join().expect("pre-replacement writer thread");
    after.join().expect("post-replacement writer thread");

    assert_documents(
        &path,
        &[
            ("seed", &[1.0, 0.0, 0.0, 0.0]),
            ("alpha", &[0.0, 1.0, 0.0, 0.0]),
            ("before", &[0.0, 0.0, 1.0, 0.0]),
            ("after", &[0.0, 0.0, 0.0, 1.0]),
        ],
    );
}

#[test]
fn subprocess_writer() {
    let Some(path) = std::env::var_os("MEMVID_WRITER_CHILD_PATH") else {
        return;
    };
    let old_contended = PathBuf::from(
        std::env::var_os("MEMVID_WRITER_CHILD_OLD_CONTENDED").expect("old contention path"),
    );
    let current_contended = PathBuf::from(
        std::env::var_os("MEMVID_WRITER_CHILD_CURRENT_CONTENDED").expect("current contention path"),
    );
    let go = PathBuf::from(std::env::var_os("MEMVID_WRITER_CHILD_GO").expect("child go path"));

    let old_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .expect("open old inode");
    assert!(
        FileLock::try_acquire(&old_file, Path::new(&path))
            .expect("probe old inode lock")
            .is_none(),
        "parent must physically lock the old destination inode"
    );
    std::fs::write(&old_contended, b"contended").expect("signal old contention");

    let deadline = Instant::now() + Duration::from_secs(5);
    while !go.exists() {
        assert!(
            Instant::now() < deadline,
            "parent did not release child gate"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let current_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .expect("open current inode");
    assert!(
        FileLock::try_acquire(&current_file, Path::new(&path))
            .expect("probe current inode lock")
            .is_none(),
        "parent must physically lock the atomically replaced destination inode"
    );
    std::fs::write(&current_contended, b"contended").expect("signal current contention");
    append(Path::new(&path), "process", vec![0.0, 0.0, 1.0, 0.0]).expect("child process commit");
}

struct ChildGuard(Option<Child>);

impl ChildGuard {
    fn child_mut(&mut self) -> &mut Child {
        self.0.as_mut().expect("child already reaped")
    }

    fn wait_with_timeout(mut self, timeout: Duration) -> ExitStatus {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(status) = self.child_mut().try_wait().expect("poll child") {
                self.0.take();
                return status;
            }
            assert!(Instant::now() < deadline, "writer subprocess timed out");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn wait_for_child_marker(child: &mut ChildGuard, marker: &Path, label: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if marker.exists() {
            return;
        }
        if let Some(status) = child.child_mut().try_wait().expect("poll child") {
            panic!("child exited before {label}: {status}");
        }
        assert!(Instant::now() < deadline, "child did not report {label}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn writer_lock_survives_atomic_replacement_across_processes() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("process.mv2");
    let old_contended = dir.path().join("child.old-contended");
    let current_contended = dir.path().join("child.current-contended");
    let go = dir.path().join("child.go");
    let mut parent = Memvid::create(&path).expect("create capsule");
    parent.enable_vec().expect("enable vectors");
    parent
        .put_with_embedding_and_options(b"seed", vec![1.0, 0.0, 0.0, 0.0], options("seed"))
        .expect("put seed");
    parent.commit().expect("commit seed");

    let child = Command::new(std::env::current_exe().expect("test executable"))
        .arg("--exact")
        .arg("subprocess_writer")
        .arg("--nocapture")
        .env("MEMVID_WRITER_CHILD_PATH", &path)
        .env("MEMVID_WRITER_CHILD_OLD_CONTENDED", &old_contended)
        .env("MEMVID_WRITER_CHILD_CURRENT_CONTENDED", &current_contended)
        .env("MEMVID_WRITER_CHILD_GO", &go)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn writer process");
    let mut child = ChildGuard(Some(child));

    wait_for_child_marker(
        &mut child,
        &old_contended,
        "physical contention on the old inode",
    );
    parent
        .put_with_embedding_and_options(b"parent", vec![0.0, 1.0, 0.0, 0.0], options("parent"))
        .expect("put parent document");
    parent.commit().expect("parent replacement commit");
    std::fs::write(&go, b"go").expect("release child after replacement");
    wait_for_child_marker(
        &mut child,
        &current_contended,
        "physical contention on the current inode",
    );
    assert!(
        child.child_mut().try_wait().expect("poll child").is_none(),
        "child writer must remain blocked while parent handle lives"
    );
    drop(parent);

    let status = child.wait_with_timeout(Duration::from_secs(10));
    assert!(status.success(), "writer subprocess failed: {status}");
    assert_documents(
        &path,
        &[
            ("seed", &[1.0, 0.0, 0.0, 0.0]),
            ("parent", &[0.0, 1.0, 0.0, 0.0]),
            ("process", &[0.0, 0.0, 1.0, 0.0]),
        ],
    );
}
