/* This Source Code Form is subject to the terms of the Mozilla Public
* License, v. 2.0. If a copy of the MPL was not distributed with this
* file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::{http_client::*, config::ClientConfig};
use mockito::mock;

#[test]
fn test_backoff() {
    viaduct_reqwest::use_reqwest_backend();
    let m = mock(
        "GET",
        "/v1/buckets/main/collections/messaging-experiments/records",
    )
    .with_body(response_body())
    .with_status(200)
    .with_header("content-type", "application/json")
    .with_header("Backoff", "60")
    .create();
    let config = ClientConfig {
        server_url: Some(mockito::server_url()),
        collection_name: "messaging-experiments".to_string(),
        bucket_name: None,
    };
    let http_client = Client::new(config).unwrap();
    todo!()
    // TODO generate request
    // assert!(http_client.make_reqyest().is_ok());
    // let second_request = http_client.fetch_experiments();
    // assert!(matches!(second_request, Err(NimbusError::BackoffError(_))));
    // m.expect(1).assert();
}

#[test]
fn test_500_retry_after() {
    viaduct_reqwest::use_reqwest_backend();
    let m = mock(
        "GET",
        "/v1/buckets/main/collections/messaging-experiments/records",
    )
    .with_body("Boom!")
    .with_status(500)
    .with_header("Retry-After", "60")
    .create();
    let config = ClientConfig {
        server_url: Some(mockito::server_url()),
        collection_name: "messaging-experiments".to_string(),
        bucket_name: None,
    };
    let http_client = Client::new(config).unwrap();
    todo!()
    // assert!(http_client.fetch_experiments().is_err());
    // let second_request = http_client.fetch_experiments();
    // assert!(matches!(second_request, Err(NimbusError::BackoffError(_))));
    // m.expect(1).assert();
}

fn response_body() -> String {
    format!(
        r#"
    {{ "data": [
        {{
            "schemaVersion": "1.0.0",
            "slug": "mobile-a-a-example",
            "appName": "reference-browser",
            "channel": "nightly",
            "userFacingName": "Mobile A/A Example",
            "userFacingDescription": "An A/A Test to validate the Rust SDK",
            "isEnrollmentPaused": false,
            "bucketConfig": {{
                "randomizationUnit": "nimbus_id",
                "namespace": "mobile-a-a-example",
                "start": 0,
                "count": 5000,
                "total": 10000
            }},
            "startDate": null,
            "endDate": null,
            "proposedEnrollment": 7,
            "referenceBranch": "control",
            "probeSets": [],
            "featureIds": ["first_switch"],
            "branches": [
                {{
                "slug": "control",
                "ratio": 1,
                "feature": {{
                    "featureId": "first_switch",
                    "enabled": false
                    }}
                }},
                {{
                "slug": "treatment-variation-b",
                "ratio": 1,
                "feature": {{
                    "featureId": "first_switch",
                    "enabled": true
                    }}
                }}
            ]
        }},
        {{
            "schemaVersion": "1.0.0",
            "slug": "mobile-a-a-example",
            "appName": "reference-browser",
            "channel": "nightly",
            "userFacingName": "Mobile A/A Example",
            "userFacingDescription": "An A/A Test to validate the Rust SDK",
            "isEnrollmentPaused": false,
            "bucketConfig": {{
                "randomizationUnit": "nimbus_id",
                "namespace": "mobile-a-a-example",
                "start": 0,
                "count": 5000,
                "total": 10000
            }},
            "startDate": null,
            "endDate": null,
            "proposedEnrollment": 7,
            "referenceBranch": "control",
            "probeSets": [],
            "featureIds": ["some_switch"],
            "branches": [
                {{
                "slug": "control",
                "ratio": 1
                }},
                {{
                "slug": "treatment-variation-b",
                "ratio": 1
                }}
            ]
        }},
        {{
            "slug": "schema-version-missing",
            "appName": "reference-browser",
            "userFacingName": "Schema Version Missing",
            "userFacingDescription": "This should be completely ignored",
            "isEnrollmentPaused": false
        }}
    ]}}"#
    )
}