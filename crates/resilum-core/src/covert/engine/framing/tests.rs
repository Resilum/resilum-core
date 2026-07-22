use super::*;

#[test]
fn recv_orders_out_of_order_chunks() {
    let mut r = RecvBuffer::new();
    assert_eq!(r.feed(2, b"BB".to_vec()), b"");
    assert_eq!(r.feed(1, b"AA".to_vec()), b"AABB");
    assert_eq!(r.feed(3, b"CC".to_vec()), b"CC");
    assert_eq!(r.ack(), 3);
}

#[test]
fn send_chunks_up_to_window() {
    let mut s = SendBuffer::new(2, 3);
    s.write(b"HELLO"); // 5 bytes → chunks of 2
    let out = s.ready(0.0);
    // Window 3 → chunks (1,"HE"), (2,"LL"), (3,"O").
    assert_eq!(
        out,
        vec![(1, b"HE".to_vec()), (2, b"LL".to_vec()), (3, b"O".to_vec())]
    );
    // Nothing new until acks come in.
    assert!(s.ready(0.0).is_empty());
}

#[test]
fn ack_frees_window_slots() {
    let mut s = SendBuffer::new(1, 2);
    s.write(b"ABC");
    let out = s.ready(0.0);
    assert_eq!(out.len(), 2); // window fills at 2
    s.ack(1, 0.1);
    let out2 = s.ready(0.2);
    assert_eq!(out2, vec![(3, b"C".to_vec())]); // window opened one slot
}

#[test]
fn ready_retransmits_after_rto() {
    let mut s = SendBuffer::new(1, 4);
    s.write(b"X");
    let first = s.ready(0.0);
    assert_eq!(first.len(), 1);
    // Below RTO — no resend.
    assert!(s.ready(0.5).is_empty());
    // After RTO — same seq re-emitted.
    let rto = s.rto().as_secs_f64();
    let out = s.ready(rto + 0.01);
    assert_eq!(out, vec![(1, b"X".to_vec())]);
}
