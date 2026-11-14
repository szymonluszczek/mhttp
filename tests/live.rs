//! Requests against real servers, so they need the engine and the network:
//!
//! ```text
//! cargo test --test live -- --ignored
//! ```
#![cfg(mhttp_engine)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use mhttp::{Connection, Error, HttpVersion, Request};

fn url(s: &str) -> mhttp::url::Url {
    s.parse().unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn http2_requests_run_concurrently_on_one_connection() {
    let connection = Arc::new(
        Connection::builder(url("https://www.google.com"))
            .connect()
            .await
            .unwrap(),
    );

    let started = Instant::now();
    let tasks: Vec<_> = (0..40)
        .map(|i| {
            let connection = connection.clone();
            tokio::spawn(async move {
                let request = Request::get(&format!("/robots.txt?n={i}"))
                    .header("accept", "*/*")
                    .build();
                connection.send(request).await
            })
        })
        .collect();

    for task in tasks {
        let response = task.await.unwrap().unwrap();
        assert_eq!(response.status(), 200);
        assert!(response.text().contains("User-agent"));
    }
    println!("40 concurrent requests in {:?}", started.elapsed());
    assert!(connection.is_open());
}

#[tokio::test]
#[ignore]
async fn http2_sends_a_body_larger_than_one_frame() {
    let connection = Connection::builder(url("https://httpbin.org"))
        .connect()
        .await
        .unwrap();

    let body = "x".repeat(200_000);
    let request = Request::post("/post")
        .header("content-type", "text/plain")
        .body(body.clone())
        .build();
    let response = connection.send(request).await.unwrap();

    assert_eq!(response.status(), 200);
    assert!(response.text().contains(&body[..1000]));
    let echoed: serde_json::Value = serde_json::from_slice(response.body()).unwrap();
    assert_eq!(echoed["data"].as_str().unwrap().len(), body.len());
}

#[tokio::test]
#[ignore]
async fn http1_keeps_the_connection_for_sequential_requests() {
    let connection = Connection::builder(url("https://httpbin.org"))
        .version(HttpVersion::Http1)
        .connect()
        .await
        .unwrap();

    for i in 0..3 {
        let request = Request::get(&format!("/get?n={i}")).build();
        let response = connection.send(request).await.unwrap();
        assert_eq!(response.status(), 200);
        assert!(response.text().contains(&format!("\"n\": \"{i}\"")));
    }
}

#[tokio::test]
#[ignore]
async fn a_slow_http2_request_times_out_alone() {
    let connection = Connection::builder(url("https://httpbin.org"))
        .timeout(Duration::from_secs(3))
        .connect()
        .await
        .unwrap();

    let slow = connection.send(Request::get("/delay/6").build());
    let fast = connection.send(Request::get("/get").build());
    let (slow, fast) = tokio::join!(slow, fast);

    assert!(
        matches!(slow, Err(Error::Native { code: -502, .. })),
        "{slow:?}"
    );
    assert_eq!(fast.unwrap().status(), 200);
    assert!(connection.is_open());
}

#[tokio::test]
#[ignore]
async fn dropping_a_connection_with_requests_in_flight_is_clean() {
    for _ in 0..5 {
        let connection = Connection::builder(url("https://httpbin.org"))
            .connect()
            .await
            .unwrap();
        let pending = connection.send(Request::get("/delay/5").build());
        // Abandon the request, then the connection.
        let _ = tokio::time::timeout(Duration::from_millis(200), pending).await;
        drop(connection);
    }
}
