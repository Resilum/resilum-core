use super::*;

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("a temporary directory")
}

#[test]
fn messages_leave_in_the_order_they_arrived() {
    let inbox = Inbox::ephemeral();
    inbox.push("first".into());
    inbox.push("second".into());

    assert_eq!(inbox.pop().as_deref(), Some("first"));
    assert_eq!(inbox.pop().as_deref(), Some("second"));
    assert_eq!(inbox.pop(), None);
}

#[test]
fn a_message_outlives_the_process_that_received_it() {
    let dir = temp_dir();
    let path = dir.path().join("inbox");

    let inbox = Inbox::open(path.clone());
    inbox.push("kept".into());
    drop(inbox);

    let reopened = Inbox::open(path);
    assert_eq!(reopened.pop().as_deref(), Some("kept"));
}

#[test]
fn a_taken_message_does_not_come_back_after_a_restart() {
    let dir = temp_dir();
    let path = dir.path().join("inbox");

    let inbox = Inbox::open(path.clone());
    inbox.push("taken".into());
    assert_eq!(inbox.pop().as_deref(), Some("taken"));
    drop(inbox);

    assert_eq!(Inbox::open(path).pop(), None);
}

#[test]
fn refusals_past_the_ceiling_are_counted_once() {
    let inbox = Inbox::ephemeral();
    for i in 0..MAX_HELD + 3 {
        inbox.push(format!("{i}"));
    }
    assert_eq!(inbox.take_dropped(), 3);
    assert_eq!(inbox.take_dropped(), 0);
}
