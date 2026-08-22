use tokio::net::TcpListener;

use super::*;

const A_REFERENCE_TARGET: [(Ipv4Addr, u16); 1] = [(Ipv4Addr::new(1, 1, 1, 1), 443)];

async fn an_exit_that(answer: &'static [u8]) -> String {
    let exit = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .expect("a local port for the exit");
    let address = exit.local_addr().expect("the port just bound").to_string();
    tokio::spawn(async move {
        let (mut served, _) = exit.accept().await.expect("the probe below");
        let mut asked = [0u8; 13];
        let _ = served.read_exact(&mut asked).await;
        let _ = served.write_all(answer).await;
    });
    address
}

#[tokio::test]
async fn an_exit_on_this_host_is_timed_without_a_leg_through_the_mesh() {
    let target = an_exit_that(&[5, 0, 5, 0, 0, 1, 0, 0, 0, 0, 0, 0]).await;

    let probe = over_a_local_socket(&target, &A_REFERENCE_TARGET)
        .await
        .expect("a measurement");

    assert_eq!(probe.link_rtt, NO_MESH_LEG);
    assert!(probe.e2e < 1.0, "a loopback exchange took {}s", probe.e2e);
}

#[tokio::test]
async fn an_exit_that_hangs_up_mid_answer_is_not_a_measurement() {
    let target = an_exit_that(&[5, 0]).await;

    assert!(
        over_a_local_socket(&target, &A_REFERENCE_TARGET)
            .await
            .is_none()
    );
}

#[tokio::test]
async fn an_exit_that_answers_quickly_but_refuses_is_not_a_working_exit() {
    const REFUSED: [u8; 12] = [5, 0, 5, 4, 0, 1, 0, 0, 0, 0, 0, 0];

    let target = an_exit_that(&REFUSED).await;

    assert!(
        over_a_local_socket(&target, &A_REFERENCE_TARGET)
            .await
            .is_none(),
        "a fast refusal is the fastest answer there is, and would win every ranking"
    );
}

#[tokio::test]
async fn an_exit_with_nothing_listening_is_not_a_measurement() {
    assert!(
        over_a_local_socket("127.0.0.1:1", &A_REFERENCE_TARGET)
            .await
            .is_none()
    );
}
