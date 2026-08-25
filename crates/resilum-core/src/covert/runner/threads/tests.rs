use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::channel;
use std::time::Duration;

use super::*;
use crate::covert::carrier::CarrierClient;

#[derive(Default)]
struct SilentCarrier {
    stopped: AtomicBool,
    calls_after_stop: AtomicUsize,
}

impl CarrierClient for SilentCarrier {
    fn capacity(&self) -> usize {
        1400
    }
    fn send_request(&self, _wire: &[u8]) -> io::Result<()> {
        Ok(())
    }
    fn recv_response(&self, _buf: &mut [u8]) -> io::Result<Option<Vec<u8>>> {
        if self.stopped.load(Ordering::SeqCst) {
            self.calls_after_stop.fetch_add(1, Ordering::SeqCst);
            return Ok(None);
        }
        std::thread::sleep(Duration::from_millis(1));
        Ok(None)
    }
    fn stop_receiving(&self) {
        self.stopped.store(true, Ordering::SeqCst);
    }
    fn told_to_stop(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }
}

#[test]
fn a_sniffer_on_a_silent_carrier_stops_calling_recv_once_told_to_stop() {
    let carrier = Arc::new(SilentCarrier::default());
    let (out, _keep) = channel();
    spawn_client_sniffer(Arc::clone(&carrier), out);

    carrier.stop_receiving();
    std::thread::sleep(Duration::from_millis(50));
    let settled = carrier.calls_after_stop.load(Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(100));
    let later = carrier.calls_after_stop.load(Ordering::SeqCst);

    assert_eq!(
        settled, later,
        "a stopped sniffer kept calling recv_response: {settled} then {later}"
    );
}
