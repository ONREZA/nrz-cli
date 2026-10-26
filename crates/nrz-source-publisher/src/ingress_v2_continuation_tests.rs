use super::*;
use std::time::Duration;

async fn receiver() -> Endpoint {
    Endpoint::builder(presets::Minimal)
        .clear_ip_transports()
        .bind_addr("127.0.0.1:0")
        .unwrap()
        .relay_mode(iroh::RelayMode::Disabled)
        .alpns(vec![wire::ALPN.to_vec()])
        .bind()
        .await
        .unwrap()
}
fn target(endpoint: &Endpoint) -> TransferTarget {
    TransferTarget {
        endpoint_id: endpoint.id().to_string(),
        direct_addresses: endpoint.addr().ip_addrs().copied().collect(),
        operation_id: Uuid::from_u128(1),
        session_id: Uuid::from_u128(2),
        manifest_blake3: "ab".repeat(32),
    }
}

#[tokio::test]
async fn accepted_source_session_outlives_selection_sample_but_not_operation() {
    for operation_expires_first in [false, true] {
        let receiver = receiver().await;
        let provider = TransferProvider::bind(std::iter::empty::<FrozenBlob>())
            .await
            .unwrap();
        let opening = Instant::now() + Duration::from_secs(2);
        let operation = if operation_expires_first {
            opening + Duration::from_millis(100)
        } else {
            opening + Duration::from_secs(2)
        };
        let target = target(&receiver);
        let (accepted, accepted_rx) = tokio::sync::oneshot::channel();
        let receiving = async {
            let connection = receiver.accept().await.unwrap().await.unwrap();
            let (mut send, mut receive) = connection.accept_bi().await.unwrap();
            let open: Open = read_frame(&mut receive, wire::MAX_OPEN_BYTES)
                .await
                .unwrap();
            assert_eq!(open.transfer_grant, "fixture-original-grant");
            write_frame(
                &mut send,
                &Reply::Accepted {
                    operation_id: Uuid::from_u128(1),
                    session_id: Uuid::from_u128(2),
                },
                wire::MAX_REPLY_BYTES,
            )
            .await
            .unwrap();
            accepted.send(()).unwrap();
            tokio::time::sleep_until(opening + Duration::from_millis(200)).await;
            if !operation_expires_first {
                write_frame(
                    &mut send,
                    &Reply::Published {
                        operation_id: Uuid::from_u128(1),
                        session_id: Uuid::from_u128(2),
                        manifest_blake3: "ab".repeat(32),
                    },
                    wire::MAX_REPLY_BYTES,
                )
                .await
                .unwrap();
                send.finish().unwrap();
            }
            connection.closed().await;
        };
        let publishing = async {
            let result = provider
                .publish_with_deadlines(target, "fixture-original-grant".into(), opening, operation)
                .await;
            accepted_rx.await.unwrap();
            assert_eq!(result.is_err(), operation_expires_first);
            assert!(Instant::now() > opening);
        };
        tokio::time::timeout(Duration::from_secs(6), async {
            tokio::join!(receiving, publishing);
        })
        .await
        .unwrap();
        receiver.close().await;
    }
}

#[tokio::test]
async fn expired_opening_never_connects_even_with_remaining_operation_budget() {
    let receiver = receiver().await;
    let provider = TransferProvider::bind(std::iter::empty::<FrozenBlob>())
        .await
        .unwrap();
    let now = Instant::now();
    assert!(
        provider
            .publish_with_deadlines(
                target(&receiver),
                "fixture-original-grant".into(),
                now - Duration::from_millis(1),
                now + Duration::from_secs(2)
            )
            .await
            .is_err()
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(20), receiver.accept())
            .await
            .is_err()
    );
    receiver.close().await;
}
