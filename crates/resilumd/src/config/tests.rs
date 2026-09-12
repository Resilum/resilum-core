use super::*;

const A_CONFIG_WITHOUT_A_PATH: &str = "instance_name: a-node\nlxmf: {}\n";

fn written(dir: &Path, body: &str) -> PathBuf {
    let file = dir.join("resilumd.yaml");
    resilum_store::write_text(&file, body).expect("a temp dir to write into");
    file
}

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("a temporary directory")
}

#[test]
fn state_lands_beside_the_config_when_none_is_named() {
    let dir = temp_dir();
    let file = written(dir.path(), A_CONFIG_WITHOUT_A_PATH);

    let cfg = load(&file).expect("the config just written");

    assert_eq!(
        cfg.storage_path.as_deref(),
        Some(dir.path().join("state").as_path())
    );
}

#[test]
fn a_named_path_is_left_alone() {
    let dir = temp_dir();
    let file = written(
        dir.path(),
        "instance_name: a-node\nstorage_path: /var/lib/resilum\nlxmf: {}\n",
    );

    let cfg = load(&file).expect("the config just written");

    assert_eq!(
        cfg.storage_path.as_deref(),
        Some(Path::new("/var/lib/resilum"))
    );
}

#[test]
fn a_config_in_the_working_directory_keeps_its_state_there() {
    assert_eq!(beside(Path::new("node.yaml")), Path::new("./state"));
}
