//! Cross-repository package-to-reconciliation contract conformance.

use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};

use fna_apps::ingress::AppWebhookIngressService;
use fna_apps_interface::{NormalizedPlatformEffect, RepositoryChangeKind};
use fna_apps_store_interface::{
    AppEventAcceptanceRequest, AppEventAcceptanceResult, AppPlatformEffectRecord, AppStoreMock,
};
use fna_apps_wasm::{HostHmacSha256Response, WasmHostMock};
use fna_db_enums::AppPlatformEffectState;
use fna_store_interface::workspace::{
    ClaimedWorkstreamReconciliation, DispatchWorkstreamReconciliationInput,
    DispatchWorkstreamReconciliationOutcome, WorkstreamPrChecks,
    WorkstreamReconciliationLeaseMutation, WorkstreamReconciliationStoreMock,
};
use fna_workstreams::{
    ConfiguredPlatformEffectDispatcher, ConfiguredWorkstreamReconciliationProcessor,
    PlatformEffectDispatcher, RefreshWorkstreamRequest, WorkstreamReconciliationProcessor,
    WorkstreamServiceMock,
};
use unimock::{MockFn as _, Unimock, matching};
use uuid::Uuid;

use crate::github_effect_conformance_support::{
    DIGEST, accepted_records, changed_workstream, envelope, records,
};
use crate::github_runtime_support::runtime_with_host;

#[tokio::test]
async fn real_review_fixture_reaches_atomic_acceptance_and_refreshes_a_snapshot() {
    let envelope = envelope();
    let records = records(&envelope);
    let installation = records.installation.clone();
    let provider = records.provider.clone();
    let version = records.version.clone();
    let stored_effect = Arc::new(Mutex::new(None::<AppPlatformEffectRecord>));
    let accepted_effect = stored_effect.clone();
    let claimed_effect = stored_effect.clone();
    let app_store = Arc::new(Unimock::new((
        AppStoreMock::get_current_app_version
            .next_call(matching!("github"))
            .returns(Ok(Some(version.clone()))),
        AppStoreMock::find_app_event_inbox_candidates
            .next_call(matching!("github", "github_events", _))
            .returns(Ok(Vec::new())),
        AppStoreMock::get_installation
            .next_call(matching!(_))
            .returns(Ok(Some(installation.clone()))),
        AppStoreMock::get_app_version
            .next_call(matching!(_))
            .returns(Ok(Some(version))),
        AppStoreMock::accept_app_event_with_deliveries
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, request: AppEventAcceptanceRequest| {
                assert_eq!(request.platform_effects.len(), 1);
                let NormalizedPlatformEffect::RepositoryChange(effect) =
                    &request.platform_effects[0];
                assert_eq!(effect.provider_repository_id, "3001");
                assert_eq!(effect.change_kind, RepositoryChangeKind::Review);
                assert_eq!(effect.branches, ["events"]);
                assert_eq!(effect.pull_request_number, Some(17));
                let (inbox, platform_effect) = accepted_records(&request);
                *accepted_effect.lock().expect("effect lock") = Some(platform_effect.clone());
                Ok(AppEventAcceptanceResult {
                    inbox,
                    duplicate: false,
                    deliveries: Vec::new(),
                    platform_effects: vec![platform_effect],
                })
            })),
        AppStoreMock::claim_due_app_platform_effects
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, request| {
                let mut effect = claimed_effect
                    .lock()
                    .expect("effect lock")
                    .clone()
                    .expect("accepted effect");
                effect.state = AppPlatformEffectState::Claimed;
                effect.claim_owner = Some(request.worker_id);
                effect.claim_token = Some(request.claim_token);
                effect.lease_expires_at = Some(request.lease_expires_at);
                Ok(vec![effect])
            })),
    )));
    let provider_store = Arc::new(Unimock::new(
        AppStoreMock::find_active_provider_installations_by_identity
            .next_call(matching!("github", "github", "1001"))
            .returns(Ok(vec![provider])),
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
    let service =
        AppWebhookIngressService::new(app_store.clone(), runtime, Arc::new(Unimock::new(())))
            .with_provider_installations(
                provider_store,
                Arc::new(Unimock::new(())),
                Arc::new(Unimock::new(())),
                Arc::new(Unimock::new(())),
            );

    let ingress = service
        .handle_webhook(envelope.clone())
        .await
        .expect("accept review");
    assert!(ingress.accepted);
    assert_eq!(ingress.delivery_count, 0);

    dispatch_and_refresh(app_store, envelope).await;
}

async fn dispatch_and_refresh(
    app_store: Arc<Unimock>,
    envelope: fna_apps_interface::runtime::WebhookEnvelope,
) {
    let agent_id = Uuid::now_v7();
    let dispatch = Arc::new(Mutex::new(None::<DispatchWorkstreamReconciliationInput>));
    let dispatched = dispatch.clone();
    let claimed = dispatch.clone();
    let workspace_store = Arc::new(Unimock::new((
        WorkstreamReconciliationStoreMock::dispatch_workstream_reconciliation
            .next_call(matching!(_))
            .answers_arc(Arc::new(
                move |_, input: DispatchWorkstreamReconciliationInput| {
                    assert_eq!(input.provider_repository_id, "3001");
                    assert_eq!(input.branches, ["events"]);
                    *dispatched.lock().expect("dispatch lock") = Some(input);
                    Ok(DispatchWorkstreamReconciliationOutcome {
                        effect_dispatched: true,
                        matched_workstreams: 1,
                    })
                },
            )),
        WorkstreamReconciliationStoreMock::claim_workstream_reconciliations
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, request| {
                let input = claimed
                    .lock()
                    .expect("dispatch lock")
                    .clone()
                    .expect("dispatch");
                Ok(vec![ClaimedWorkstreamReconciliation {
                    workspace_id: input.workspace_id,
                    agent_id,
                    generation: 1,
                    attempt_count: 0,
                    provisional_reread_used: false,
                    available_at: input.available_at,
                    claim_token: request.claim_token,
                    lease_expires_at: request.lease_expires_at,
                }])
            })),
        WorkstreamReconciliationStoreMock::complete_workstream_reconciliation
            .next_call(matching!(_))
            .returns(Ok(WorkstreamReconciliationLeaseMutation::Applied)),
    )));
    let now = envelope.received_at;
    let dispatcher = ConfiguredPlatformEffectDispatcher::new(
        app_store,
        workspace_store.clone(),
        String::from("conformance-dispatcher"),
    );
    let summary = dispatcher
        .dispatch_due(now, NonZeroU32::MIN)
        .expect("dispatch effect");
    assert_eq!(summary.matched_workstreams, 1);
    let processing_at = dispatch
        .lock()
        .expect("dispatch lock")
        .as_ref()
        .expect("dispatch")
        .available_at;
    let snapshot = changed_workstream(
        dispatch
            .lock()
            .expect("dispatch lock")
            .as_ref()
            .expect("dispatch")
            .workspace_id,
        agent_id,
        &envelope,
    );
    let observed = Arc::new(Mutex::new(None));
    let observed_snapshot = observed.clone();
    let workstreams = Arc::new(Unimock::new(
        WorkstreamServiceMock::refresh
            .next_call(matching!(RefreshWorkstreamRequest { .. }))
            .answers_arc(Arc::new(move |_, _| {
                *observed_snapshot.lock().expect("snapshot lock") = Some(snapshot.clone());
                Ok(snapshot.clone())
            })),
    ));
    let processed = ConfiguredWorkstreamReconciliationProcessor::new(
        workspace_store,
        workstreams,
        String::from("conformance-reconciler"),
    )
    .process_due(processing_at, NonZeroU32::MIN)
    .await
    .expect("reconcile workstream");
    assert_eq!(processed, 1);
    assert_eq!(
        observed
            .lock()
            .expect("snapshot lock")
            .as_ref()
            .expect("snapshot")
            .pr_checks,
        WorkstreamPrChecks::Passing
    );
}
