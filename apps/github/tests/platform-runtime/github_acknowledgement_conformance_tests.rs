//! Pinned-host conformance for authenticated acknowledge-and-drop events.

use std::sync::Arc;

use fna_apps::ingress::AppWebhookIngressService;
use fna_apps_store_interface::AppStoreMock;
use fna_apps_wasm::{HostHmacSha256Response, WasmHostMock};
use unimock::{MockFn as _, Unimock, matching};

use crate::github_effect_conformance_support::{DIGEST, envelope_for, records};
use crate::github_runtime_support::runtime_with_host;

const ACKNOWLEDGED: [&str; 29] = [
    "create",
    "delete",
    "commit_comment",
    "fork",
    "gollum",
    "release",
    "repository",
    "repository_dispatch",
    "member",
    "public",
    "star",
    "watch",
    "pull_request_review_thread",
    "merge_queue_entry",
    "issue_dependencies",
    "sub_issues",
    "label",
    "milestone",
    "workflow_dispatch",
    "deployment",
    "deployment_status",
    "deployment_review",
    "deployment_protection_rule",
    "registry_package",
    "page_build",
    "discussion",
    "discussion_comment",
    "deploy_key",
    "exemption_request_push_ruleset",
];

#[tokio::test]
async fn every_acknowledged_event_skips_normalization_and_storage() {
    for event_type in ACKNOWLEDGED {
        let results = handle(event_type, 1).await;
        assert!(results[0].accepted, "{event_type} should be acknowledged");
        assert!(!results[0].duplicate);
        assert_eq!(results[0].delivery_count, 0);
    }
}

#[tokio::test]
async fn acknowledged_provider_retries_repeat_the_same_storage_free_noop() {
    let results = handle("repository_dispatch", 2).await;
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|result| result.accepted));
    assert!(results.iter().all(|result| !result.duplicate));
    assert!(results.iter().all(|result| result.delivery_count == 0));
}

async fn handle(
    event_type: &str,
    attempts: usize,
) -> Vec<fna_apps::ingress::AppWebhookIngressResult> {
    let envelope = envelope_for(
        event_type,
        include_bytes!("../fixtures/webhooks/acknowledged.json"),
    );
    let records = records(&envelope);
    let installation = records.installation;
    let provider = records.provider;
    let version = records.version;
    let route_version = version.clone();
    let pinned_version = version.clone();
    let resolved_installation = installation.clone();
    let app_store = Arc::new(Unimock::new((
        AppStoreMock::get_current_app_version
            .each_call(matching!("github"))
            .answers_arc(Arc::new(move |_, _| Ok(Some(route_version.clone())))),
        AppStoreMock::find_app_event_inbox_candidates
            .each_call(matching!("github", "github_events", _))
            .answers(&|_, _, _, _| Ok(Vec::new())),
        AppStoreMock::get_installation
            .each_call(matching!(_))
            .answers_arc(Arc::new(move |_, _| {
                Ok(Some(resolved_installation.clone()))
            })),
        AppStoreMock::get_app_version
            .each_call(matching!(_))
            .answers_arc(Arc::new(move |_, _| Ok(Some(pinned_version.clone())))),
    )));
    let provider_store = Arc::new(Unimock::new(
        AppStoreMock::find_active_provider_installations_by_identity
            .each_call(matching!("github", "github", "1001"))
            .answers_arc(Arc::new(move |_, _, _, _| Ok(vec![provider.clone()]))),
    ));
    let runtime = Arc::new(runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::hmac_sha256
            .each_call(matching!(_))
            .returns(HostHmacSha256Response {
                ok: true,
                digest: Some(String::from(DIGEST)),
                error: None,
            }),
    ))));
    let service = AppWebhookIngressService::new(app_store, runtime, Arc::new(Unimock::new(())))
        .with_provider_installations(
            provider_store,
            Arc::new(Unimock::new(())),
            Arc::new(Unimock::new(())),
            Arc::new(Unimock::new(())),
        );

    let mut results = Vec::with_capacity(attempts);
    for _ in 0..attempts {
        results.push(
            service
                .handle_webhook(envelope.clone())
                .await
                .expect("acknowledge signed event"),
        );
    }
    results
}
