/* This Source Code Form is subject to the terms of the Mozilla Public
* License, v. 2.0. If a copy of the MPL was not distributed with this
* file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::{http_client::*, config::ClientConfig, error::ClientError};
use mockito::mock;
use std::cell::Cell;
use std::time::{Duration, Instant};

#[test]
fn test_backoff() {
    viaduct_reqwest::use_reqwest_backend();
    let m = mock(
        "GET",
        "/v1/buckets/the-bucket/collections/the-collection/records",
    )
    .with_body(response_body())
    .with_status(200)
    .with_header("content-type", "application/json")
    .with_header("Backoff", "60")
    .create();
    let config = ClientConfig {
        server_url: Some(mockito::server_url()),
        collection_name: String::from("the-collection"),
        bucket_name: Some(String::from("the-bucket")),
    };
    let http_client = Client::new(config).unwrap();

    // let url = Url::parse(&format!("{}/{}", &base_url, path)).unwrap();
    assert!(http_client.get().is_ok());
    let second_resp = http_client.get();
    assert!(matches!(second_resp, Err(ClientError::BackoffError(_))));
    m.expect(1).assert();
}

#[test]
fn test_500_retry_after() {
    viaduct_reqwest::use_reqwest_backend();
    let m = mock(
        "GET",
        "/v1/buckets/the-bucket/collections/the-collection/records",
    )
    .with_body("Boom!")
    .with_status(500)
    .with_header("Retry-After", "60")
    .create();
    let config = ClientConfig {
        server_url: Some(mockito::server_url()),
        collection_name: String::from("the-collection"),
        bucket_name: Some(String::from("the-bucket")),
    };
    let http_client = Client::new(config).unwrap();
    assert!(http_client.get().is_err());
    let second_request = http_client.get();
    assert!(matches!(second_request, Err(ClientError::BackoffError(_))));
    m.expect(1).assert();
}

#[test]
fn test_backoff_recovery() {
    viaduct_reqwest::use_reqwest_backend();
    let m = mock(
        "GET",
        "/v1/buckets/the-bucket/collections/the-collection/records",
    )
    .with_body(response_body())
    .with_status(200)
    .with_header("content-type", "application/json")
    .create();
    let config = ClientConfig {
        server_url: Some(mockito::server_url()),
        collection_name: String::from("the-collection"),
        bucket_name: Some(String::from("the-bucket")),
    };
    let mut http_client = Client::new(config).unwrap();
    // First, sanity check that manipulating the remote state does something.
    http_client.remote_state.replace(RemoteState::Backoff {
        observed_at: Instant::now(),
        duration: Duration::from_secs(30),
    });
    assert!(matches!(
        http_client.get(),
        Err(ClientError::BackoffError(_))
    ));
    // Then do the actual test.
    http_client.remote_state = Cell::new(RemoteState::Backoff {
        observed_at: Instant::now() - Duration::from_secs(31),
        duration: Duration::from_secs(30),
    });
    assert!(http_client.get().is_ok());
    m.expect(1).assert();
}

fn response_body() -> String {
    format!(
        r#"
    {{ "data": [
        {{
            "empty_field": null,
            "bool_field": true,
            "int_field": 123,
            "string_field": "value"
        }}
    ]}}"#
    )
}