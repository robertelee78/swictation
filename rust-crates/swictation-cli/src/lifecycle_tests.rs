use super::*;
use std::{fs, os::unix::fs::PermissionsExt, path::Path};

fn sandbox() -> (tempfile::TempDir, paths::Paths) {
    let temp = tempfile::tempdir().unwrap();
    let home = fs::canonicalize(temp.path()).unwrap();
    let data = home.join("data");
    let paths = paths::Paths {
        home: home.clone(),
        root: data.join("install"),
        data,
        config: home.join("config"),
        bin: home.join("bin"),
    };
    (temp, paths)
}

fn archive(
    dir: &Path,
    version: &str,
    extra: Option<(&str, &[u8])>,
) -> (std::path::PathBuf, String) {
    let file = dir.join(format!("fixture-{version}.tar.gz"));
    let output = fs::File::create(&file).unwrap();
    let encoder = flate2::write::GzEncoder::new(output, flate2::Compression::default());
    let mut archive = tar::Builder::new(encoder);
    let cli = format!("#!/bin/sh\nprintf 'swictation {version}\\n'\n");
    let release = serde_json::to_vec(&package::Release {
        schema_version: 1,
        version: version.into(),
        target: package::target().unwrap().into(),
        source_sha: "a".repeat(40),
    })
    .unwrap();
    let lib = if cfg!(target_os = "macos") {
        "lib/libonnxruntime.dylib"
    } else {
        "lib/libonnxruntime.so"
    };
    let entries: Vec<(&str, &[u8], u32)> = vec![
        ("bin/swictation", cli.as_bytes(), 0o755),
        ("bin/swictation-daemon", b"daemon", 0o755),
        ("bin/swictation-ui", b"ui", 0o755),
        ("release.json", &release, 0o644),
        (lib, b"library", 0o644),
        ("share/models.manifest.json", b"{}", 0o644),
        ("share/config.example.toml", b"[audio]\n", 0o644),
    ];
    for (name, bytes, mode) in entries.into_iter().chain(extra.map(|(n, b)| (n, b, 0o644))) {
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(mode);
        header.set_cksum();
        archive.append_data(&mut header, name, bytes).unwrap();
    }
    archive.into_inner().unwrap().finish().unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
    let digest = package::hash(&file).unwrap();
    (file, digest)
}

#[test]
fn install_update_rollback_uninstall_preserves_user_state() {
    let (_temp, p) = sandbox();
    fs::create_dir_all(p.models()).unwrap();
    fs::create_dir_all(&p.config).unwrap();
    fs::write(p.config_file(), "# custom setting\n").unwrap();
    fs::write(p.models().join("weights"), b"valuable").unwrap();
    let (a, hash_a) = archive(&p.home, "0.7.37", None);
    lifecycle::install(&p, &a, &hash_a, true).unwrap();
    let (b, hash_b) = archive(&p.home, "0.7.38", None);
    lifecycle::install(&p, &b, &hash_b, true).unwrap();
    assert_eq!(lifecycle::current(&p).unwrap().release.version, "0.7.38");
    lifecycle::rollback(&p).unwrap();
    assert_eq!(lifecycle::current(&p).unwrap().archive_sha256, hash_a);
    lifecycle::rollback(&p).unwrap();
    assert_eq!(lifecycle::current(&p).unwrap().archive_sha256, hash_b);
    assert!(lifecycle::uninstall(&p, false, false, false).is_err());
    lifecycle::uninstall(&p, true, false, false).unwrap();
    assert!(fs::symlink_metadata(p.launcher()).is_err());
    assert_eq!(fs::read(p.config_file()).unwrap(), b"# custom setting\n");
    assert_eq!(fs::read(p.models().join("weights")).unwrap(), b"valuable");
}

#[test]
fn wrong_digest_and_downgrade_leave_current_unchanged() {
    let (_temp, p) = sandbox();
    let (a, hash_a) = archive(&p.home, "0.7.37", None);
    lifecycle::install(&p, &a, &hash_a, true).unwrap();
    let (b, _) = archive(&p.home, "0.7.38", None);
    assert!(lifecycle::install(&p, &b, &"0".repeat(64), true).is_err());
    let (old, old_hash) = archive(&p.home, "0.7.36", None);
    assert!(lifecycle::install(&p, &old, &old_hash, true).is_err());
    assert_eq!(lifecycle::current(&p).unwrap().archive_sha256, hash_a);
    assert_eq!(
        fs::read_link(p.root.join("current")).unwrap(),
        PathBuf::from(format!("releases/0.7.37-{hash_a}"))
    );
}

#[test]
fn foreign_launcher_is_never_overwritten() {
    let (_temp, p) = sandbox();
    fs::create_dir_all(&p.bin).unwrap();
    fs::write(p.launcher(), "foreign").unwrap();
    let (archive, hash) = archive(&p.home, "0.7.37", None);
    assert!(lifecycle::install(&p, &archive, &hash, true).is_err());
    assert_eq!(fs::read(p.launcher()).unwrap(), b"foreign");
}

#[test]
fn lock_rejects_concurrent_mutations() {
    let (_temp, p) = sandbox();
    let _guard = lifecycle::lock(&p).unwrap();
    assert!(lifecycle::lock(&p).is_err());
}

#[test]
fn modified_retained_release_refuses_rollback() {
    let (_temp, p) = sandbox();
    let (a, ah) = archive(&p.home, "0.7.37", None);
    lifecycle::install(&p, &a, &ah, true).unwrap();
    let (b, bh) = archive(&p.home, "0.7.38", None);
    lifecycle::install(&p, &b, &bh, true).unwrap();
    let previous = p.root.join(fs::read_link(p.root.join("previous")).unwrap());
    fs::write(
        previous.join("lib").join(if cfg!(target_os = "macos") {
            "libonnxruntime.dylib"
        } else {
            "libonnxruntime.so"
        }),
        "tampered",
    )
    .unwrap();
    assert!(lifecycle::rollback(&p).is_err());
    assert_eq!(lifecycle::current(&p).unwrap().archive_sha256, bh);
}

#[test]
fn unsafe_members_and_symlinked_owner_fail_closed() {
    let (_temp, p) = sandbox();
    let (a, ah) = archive(&p.home, "0.7.37", Some((".receipt.json", b"{}")));
    assert!(lifecycle::install(&p, &a, &ah, true).is_err());
    assert!(!p.current().exists());
    fs::remove_file(p.root.join(".owner")).unwrap();
    let foreign = p.home.join("foreign");
    fs::write(&foreign, "safe").unwrap();
    std::os::unix::fs::symlink(&foreign, p.root.join(".owner")).unwrap();
    assert!(lifecycle::lock(&p).is_err());
    assert_eq!(fs::read(foreign).unwrap(), b"safe");
}

#[test]
fn parser_rejects_conflicting_modes_and_missing_confirmation() {
    assert!(Cli::try_parse_from(["swictation", "update", "--check", "--rollback"]).is_err());
    assert!(
        Cli::try_parse_from(["swictation", "update", "--version", "0.7.37", "--rollback"]).is_err()
    );
    assert!(Cli::try_parse_from(["swictation", "uninstall", "--purge-cache"]).is_ok());
    assert!(network::stable_version("0.7.37-rc.1").is_err());
    assert!(network::stable_version("../../evil").is_err());
    assert!(paths::safe_relative(Path::new("bin/../../escape")).is_err());
}

#[test]
fn start_defaults_to_app_and_preserves_explicit_ui_compatibility() {
    for args in [
        vec!["swictation", "start"],
        vec!["swictation", "start", "--ui"],
    ] {
        let cli = Cli::try_parse_from(args).unwrap();
        assert!(matches!(
            cli.command,
            Command::Start {
                daemon_only: false,
                ..
            }
        ));
    }
    let cli = Cli::try_parse_from(["swictation", "start", "--daemon-only"]).unwrap();
    assert!(matches!(
        cli.command,
        Command::Start {
            daemon_only: true,
            ..
        }
    ));
    assert!(Cli::try_parse_from(["swictation", "start", "--ui", "--daemon-only"]).is_err());
    let help = Cli::try_parse_from(["swictation", "start", "--help"])
        .err()
        .unwrap();
    assert_eq!(help.kind(), clap::error::ErrorKind::DisplayHelp);
    assert!(help
        .to_string()
        .contains("Start the dictation daemon and desktop app"));
    assert!(help.to_string().contains("--daemon-only"));
    assert!(help.to_string().contains("already the default"));
}

#[test]
fn archive_symlink_is_rejected() {
    let (_temp, p) = sandbox();
    let file = p.home.join("link.tar.gz");
    let mut archive = tar::Builder::new(flate2::write::GzEncoder::new(
        fs::File::create(&file).unwrap(),
        flate2::Compression::default(),
    ));
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Symlink);
    header.set_mode(0o777);
    header.set_size(0);
    archive
        .append_link(&mut header, "bin/swictation", "/etc/passwd")
        .unwrap();
    archive.into_inner().unwrap().finish().unwrap();
    let digest = package::hash(&file).unwrap();
    assert!(lifecycle::install(&p, &file, &digest, true).is_err());
    assert!(!p.current().exists());
}

#[test]
fn first_install_leaves_existing_services_untouched() {
    let (_temp, p) = sandbox();
    let unit = services::unit_path(&p, "daemon");
    fs::create_dir_all(unit.parent().unwrap()).unwrap();
    fs::write(&unit, "foreign service must remain untouched").unwrap();
    let (a, hash) = archive(&p.home, "0.8.0", None);
    lifecycle::install(&p, &a, &hash, true).unwrap();
    assert_eq!(
        fs::read_to_string(unit).unwrap(),
        "foreign service must remain untouched"
    );
    assert_eq!(lifecycle::current(&p).unwrap().release.version, "0.8.0");
}

#[test]
fn damaged_same_version_is_repaired_without_overwriting_old_files() {
    let (_temp, p) = sandbox();
    let (a, hash) = archive(&p.home, "0.8.0", None);
    lifecycle::install(&p, &a, &hash, true).unwrap();
    let old = fs::canonicalize(p.current()).unwrap();
    fs::write(old.join("bin/swictation-ui"), "damaged").unwrap();
    lifecycle::install(&p, &a, &hash, true).unwrap();
    assert_ne!(fs::canonicalize(p.current()).unwrap(), old);
    assert_eq!(fs::read(old.join("bin/swictation-ui")).unwrap(), b"damaged");
    assert_eq!(
        fs::read(p.current().join("bin/swictation-ui")).unwrap(),
        b"ui"
    );
}

#[test]
fn invalid_previous_and_symlinked_purge_preserve_installation() {
    let (_temp, p) = sandbox();
    let (a, hash) = archive(&p.home, "0.8.0", None);
    lifecycle::install(&p, &a, &hash, true).unwrap();
    std::os::unix::fs::symlink("../foreign", p.root.join("previous")).unwrap();
    assert!(lifecycle::uninstall(&p, true, false, false).is_err());
    assert!(p.launcher().exists());
    assert!(p.current().exists());
    fs::remove_file(p.root.join("previous")).unwrap();
    fs::create_dir_all(p.models()).unwrap();
    let outside = p.home.join("important");
    fs::write(&outside, "keep").unwrap();
    std::os::unix::fs::symlink(&outside, p.models().join("foreign")).unwrap();
    assert!(lifecycle::uninstall(&p, true, false, true).is_err());
    assert!(p.launcher().exists());
    assert_eq!(fs::read_to_string(outside).unwrap(), "keep");
}
