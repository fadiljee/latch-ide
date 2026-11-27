#![allow(clippy::unwrap_used, clippy::expect_used)]

#[cfg(target_os = "linux")]
#[test]
fn landlock_enforces_worktree_jail_and_denies_sensitive_paths() {
    use std::path::PathBuf;
    use std::process::Command;

    use latch_core::sandbox::{SandboxPolicy, SandboxProvider};
    use latch_sandbox::{preset_ro_paths, LandlockProvider};
    use tempfile::TempDir;

    let Some(provider) = LandlockProvider::probe() else {
        // Runner tanpa kernel Landlock (mis. container terbatas)
        return;
    };

    let root = TempDir::new().unwrap();
    let worktree = root.path().join("worktree");
    let tmp_dir = root.path().join("tmp");
    let secret_dir = root.path().join("secret_ssh");
    let outside_dir = root.path().join("outside");
    std::fs::create_dir_all(&worktree).unwrap();
    std::fs::create_dir_all(&tmp_dir).unwrap();
    std::fs::create_dir_all(&secret_dir).unwrap();
    std::fs::create_dir_all(&outside_dir).unwrap();

    let secret_file = secret_dir.join("id_rsa");
    std::fs::write(&secret_file, "PRIVATE-KEY-CANARY").unwrap();
    let worktree_file = worktree.join("hello.txt");
    std::fs::write(&worktree_file, "ok").unwrap();

    let mut ro_paths = preset_ro_paths("system");
    ro_paths.push(PathBuf::from("/etc"));
    let policy = SandboxPolicy {
        rw_paths: vec![
            worktree.clone(),
            tmp_dir.clone(),
            PathBuf::from("/dev/null"),
        ],
        ro_paths,
        deny_hints: vec![secret_dir.clone()],
        allow_network: Vec::new(),
    };

    // 1. Baca & tulis di dalam worktree HARUS berhasil
    let mut ok_cmd = Command::new("/bin/sh");
    ok_cmd.args([
        "-c",
        &format!(
            "cat '{}' >/dev/null && echo written > '{}/out.txt'",
            worktree_file.display(),
            worktree.display()
        ),
    ]);
    provider.prepare(&policy, &mut ok_cmd).unwrap();
    let ok_status = ok_cmd.status().unwrap();
    assert!(
        ok_status.success(),
        "worktree read/write should succeed under Landlock"
    );
    assert!(worktree.join("out.txt").exists());

    // 2. Membaca berkas rahasia di luar allowlist HARUS ditolak (EACCES)
    let mut read_secret = Command::new("/bin/cat");
    read_secret.arg(&secret_file);
    provider.prepare(&policy, &mut read_secret).unwrap();
    let read_out = read_secret.output().unwrap();
    assert!(
        !read_out.status.success(),
        "reading sensitive path outside Landlock allowlist MUST fail"
    );

    // 3. Menulis ke direktori di luar worktree HARUS ditolak (EACCES)
    let mut write_outside = Command::new("/bin/sh");
    write_outside.args([
        "-c",
        &format!("echo x > '{}/escape.txt'", outside_dir.display()),
    ]);
    provider.prepare(&policy, &mut write_outside).unwrap();
    let write_out = write_outside.output().unwrap();
    assert!(
        !write_out.status.success(),
        "writing outside worktree MUST fail under Landlock"
    );
    assert!(!outside_dir.join("escape.txt").exists());
}
