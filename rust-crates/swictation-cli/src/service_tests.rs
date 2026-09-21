use super::*;
#[test]
fn service_escaping_rejects_directive_injection() {
    assert!(systemd_quote("safe\nExecStart=/bin/evil").is_err());
    assert_eq!(
        systemd_quote("/home/a b/100%/$HOME").unwrap(),
        "\"/home/a b/100%%/$$HOME\""
    );
}
#[test]
fn foreign_service_is_not_claimed() {
    let temp = tempfile::tempdir().unwrap();
    let paths = Paths {
        home: temp.path().into(),
        data: temp.path().join("data"),
        config: temp.path().join("config"),
        bin: temp.path().join("bin"),
        root: temp.path().join("root"),
    };
    let unit = unit_path(&paths, "daemon");
    fs::create_dir_all(unit.parent().unwrap()).unwrap();
    fs::write(unit, "[Service]\nExecStart=/some/custom/daemon\n").unwrap();
    assert!(owned(&paths, "daemon").is_err());
    let unit = unit_path(&paths, "daemon");
    fs::remove_file(&unit).unwrap();
    std::os::unix::fs::symlink(paths.home.join("missing"), &unit).unwrap();
    assert!(owned(&paths, "daemon").is_err());
    assert!(fs::symlink_metadata(unit).unwrap().file_type().is_symlink());
}
