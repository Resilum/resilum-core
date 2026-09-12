use super::*;

type Lines = Vec<String>;

fn decode(bytes: &[u8]) -> Result<Lines, String> {
    let text = String::from_utf8(bytes.to_vec()).map_err(|e| e.to_string())?;
    Ok(text.lines().map(str::to_owned).collect())
}

fn encode(held: &Lines) -> Vec<u8> {
    held.join("\n").into_bytes()
}

fn open(path: PathBuf) -> Document<Lines> {
    Document::open(path, Lines::new(), decode, encode).expect("opens")
}

#[test]
fn what_one_run_wrote_the_next_one_opens() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("kept");

    let held = open(path.clone());
    held.change(|lines| lines.push("first".to_owned()));
    drop(held);

    assert_eq!(open(path).read(Vec::clone), vec!["first".to_owned()]);
}

#[test]
fn a_burst_of_changes_leaves_the_last_state_whole() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("kept");

    let held = open(path.clone());
    for nth in 0..200 {
        held.change(|lines| lines.push(format!("line {nth}")));
    }
    drop(held);

    assert_eq!(open(path).read(Vec::len), 200);
}

#[test]
fn a_file_that_cannot_be_read_is_refused_rather_than_guessed() {
    let a_directory_where_the_file_belongs = tempfile::tempdir().expect("a temporary directory");

    let refused = Document::open(
        a_directory_where_the_file_belongs.path().to_path_buf(),
        Lines::new(),
        decode,
        encode,
    );

    assert!(refused.is_err());
}

#[test]
fn a_file_that_is_not_there_yet_opens_empty() {
    let dir = tempfile::tempdir().expect("a temporary directory");

    assert!(open(dir.path().join("absent")).read(Vec::is_empty));
}

#[test]
fn a_document_with_nowhere_to_write_still_holds_its_state() {
    let held = Document::in_memory(Vec::new());

    held.change(|lines| lines.push("kept".to_owned()));

    assert_eq!(held.read(Vec::clone), vec!["kept".to_owned()]);
}
