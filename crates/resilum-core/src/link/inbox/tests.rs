use std::sync::Arc;

use leviculum_std::api::{DestinationHash, LinkId};

use super::*;

const CLAIMED: [u8; 16] = [1; 16];
const NOBODYS: [u8; 16] = [2; 16];

fn arrival(destination: [u8; 16]) -> Inbound {
    let (_, rx) = unbounded_channel();
    (LinkId::new([9; 16]), DestinationHash::new(destination), rx)
}

#[tokio::test]
async fn a_link_goes_to_whoever_claimed_its_destination() {
    let inbox = Arc::new(Inbox::default());
    let mut claimed = inbox.claim(CLAIMED);
    let (unclaimed_tx, mut unclaimed) = unbounded_channel();
    let (arriving_tx, arriving) = unbounded_channel();

    let sorting = tokio::spawn({
        let inbox = inbox.clone();
        async move { inbox.sort(arriving, unclaimed_tx).await }
    });
    arriving_tx.send(arrival(CLAIMED)).expect("sorter is up");
    arriving_tx.send(arrival(NOBODYS)).expect("sorter is up");
    drop(arriving_tx);
    sorting.await.expect("the sorter runs to the end");

    assert_eq!(
        claimed.recv().await.map(|(_, to, _)| to),
        Some(DestinationHash::new(CLAIMED))
    );
    assert_eq!(
        unclaimed.recv().await.map(|(_, to, _)| to),
        Some(DestinationHash::new(NOBODYS))
    );
}
