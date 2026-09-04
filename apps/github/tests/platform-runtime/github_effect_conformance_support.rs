//! Records and envelopes for cross-repository effect conformance.

use std::collections::BTreeMap;

use fna_apps_interface::manifest::RuntimeKind;
use fna_apps_interface::runtime::{WebhookEnvelope, WebhookHeader};
use fna_apps_store_interface::{
    AppEventAcceptanceRequest, AppEventInboxRecord, AppInstallStatus, AppInstallationRecord,
    AppPlatformEffectRecord, AppVersionRecord, AppVisibility, ProviderInstallationRecord,
};
use fna_db_enums::{
    AppEventInboxOutcome, AppPlatformEffectState, ExternalRepoProvider, ProviderInstallationState,
    RepoSectionOrigin,
};
use fna_store_interface::workspace::{
    WorkstreamPrChecks, WorkstreamPrMergeable, WorkstreamPrReview, WorkstreamPrState,
    WorkstreamRecord, WorkstreamRepository,
};
use uuid::Uuid;

use crate::manifest;

pub(crate) const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub(crate) const DELIVERY: &str = "123e4567-e89b-12d3-a456-426614174000";

pub(crate) struct ConformanceRecords {
    pub(crate) installation: AppInstallationRecord,
    pub(crate) provider: ProviderInstallationRecord,
    pub(crate) version: AppVersionRecord,
}

pub(crate) fn records(envelope: &WebhookEnvelope) -> ConformanceRecords {
    let workspace_id = Uuid::now_v7();
    let installation_id = Uuid::now_v7();
    let version_id = Uuid::now_v7();
    let installed_at = envelope.received_at;
    let installation = AppInstallationRecord {
        id: installation_id,
        workspace_id,
        app_id: String::from("github"),
        app_version_id: Some(version_id),
        manifest_version: Some(String::from("2.1.0")),
        installed_by_user_id: Some(Uuid::now_v7()),
        provider_account_id: Some(String::from("2001")),
        provider_account_label: Some(String::from("octo-org")),
        status: AppInstallStatus::Active,
        granted_scopes: manifest().auth_requirements[0].scopes.clone(),
        available_to_all_agents: true,
        authorization_revision: 7,
        created_at: installed_at,
        updated_at: installed_at,
    };
    let provider = ProviderInstallationRecord {
        id: installation_id,
        app_id: String::from("github"),
        provider: String::from("github"),
        provider_installation_id: String::from("1001"),
        provider_account_id: String::from("2001"),
        provider_account_label: String::from("octo-org"),
        workspace_id: Some(workspace_id),
        state: ProviderInstallationState::Active,
        claimed_by_user_id: installation.installed_by_user_id,
        claimed_at: Some(installed_at),
        unclaimed_expires_at: None,
        created_at: installed_at,
        updated_at: installed_at,
    };
    let manifest = manifest();
    let version = AppVersionRecord {
        id: version_id,
        app_id: String::from("github"),
        version: manifest.version.clone(),
        manifest,
        manifest_hash: String::from("manifest-hash"),
        component_ref: Some(String::from("component/github.wasm")),
        component_hash: Some(String::from("component-hash")),
        runtime_kind: RuntimeKind::WasmComponent,
        abi: Some(String::from("firna-app-component-v1")),
        source_package: None,
        visibility: AppVisibility::External,
        owning_workspace_id: None,
        created_at: installed_at,
    };
    ConformanceRecords {
        installation,
        provider,
        version,
    }
}

pub(crate) fn envelope() -> WebhookEnvelope {
    envelope_for(
        "pull_request_review",
        include_bytes!("../fixtures/webhooks/pull_request_review.json"),
    )
}

pub(crate) fn envelope_for(event_type: &str, body: &[u8]) -> WebhookEnvelope {
    WebhookEnvelope {
        app_id: String::from("github"),
        ingress_id: String::from("github_events"),
        headers: vec![
            header("x-hub-signature-256", &format!("sha256={DIGEST}")),
            header("x-github-delivery", DELIVERY),
            header("x-github-event", event_type),
        ],
        query: BTreeMap::new(),
        body: body.to_vec(),
        received_at: "2026-08-03T12:00:00Z".parse().expect("fixture time"),
    }
}

pub(crate) fn changed_workstream(
    workspace_id: Uuid,
    agent_id: Uuid,
    envelope: &WebhookEnvelope,
) -> WorkstreamRecord {
    let now = envelope.received_at;
    WorkstreamRecord {
        agent_id,
        workspace_id,
        section_id: Some(Uuid::now_v7()),
        agent_archived: false,
        repository: WorkstreamRepository {
            origin: RepoSectionOrigin::External,
            git_repo_id: None,
            external_repo_id: Some(Uuid::now_v7()),
            name: String::from("repo"),
            default_branch: String::from("main"),
            branch_prefix: None,
            provider: Some(ExternalRepoProvider::Github),
            provider_repository_id: Some(String::from("3001")),
            app_installation_id: Some(Uuid::now_v7()),
        },
        branch_name: String::from("events"),
        pr_state: WorkstreamPrState::Open,
        pr_checks: WorkstreamPrChecks::Passing,
        pr_review: WorkstreamPrReview::Approved,
        pr_mergeable: WorkstreamPrMergeable::Mergeable,
        pr_url: Some(String::from("https://github.com/octo-org/repo/pull/17")),
        pr_number: Some(17),
        wrap_up_at: None,
        provision_failed_at: None,
        provision_failure_reason: None,
        merge_failed_at: None,
        merge_failure_reason: None,
        created_at: now,
        updated_at: now,
    }
}

pub(crate) fn accepted_records(
    request: &AppEventAcceptanceRequest,
) -> (AppEventInboxRecord, AppPlatformEffectRecord) {
    let inbox_id = request.inbox.id;
    let now = request.accepted_at;
    let inbox = AppEventInboxRecord {
        id: inbox_id,
        workspace_id: request.inbox.workspace_id,
        installation_id: request.inbox.installation_id,
        ingress_id: request.inbox.ingress_id.clone(),
        provider: request.inbox.provider.clone(),
        provider_workspace_id: request.inbox.provider_workspace_id.clone(),
        provider_event_id: request.inbox.provider_event_id.clone(),
        provider_event_type: request.inbox.provider_event_type.clone(),
        event_id: request.inbox.event_id.clone(),
        event_fingerprint: request.inbox.event_fingerprint.clone(),
        verifier_app_version_id: request.inbox.verifier_app_version_id,
        normalizer_app_version_id: request.inbox.normalizer_app_version_id,
        received_at: request.inbox.received_at,
        source_metadata_json: request.inbox.source_metadata_json.clone(),
        redacted_payload_json: request.inbox.redacted_payload_json.clone(),
        outcome: AppEventInboxOutcome::Accepted,
        purge_at: request.inbox.purge_at,
        created_at: now,
        updated_at: now,
    };
    let effect = AppPlatformEffectRecord {
        id: Uuid::now_v7(),
        workspace_id: request.inbox.workspace_id,
        inbox_id,
        effect_ordinal: 1,
        provider: request.inbox.provider.clone(),
        effect: request.platform_effects[0].clone(),
        state: AppPlatformEffectState::Pending,
        claim_owner: None,
        claim_token: None,
        lease_expires_at: None,
        attempt_count: 0,
        available_at: now,
        last_error_code: None,
        dispatched_at: None,
        dead_lettered_at: None,
        created_at: now,
        updated_at: now,
    };
    (inbox, effect)
}

fn header(name: &str, value: &str) -> WebhookHeader {
    WebhookHeader {
        name: name.to_owned(),
        value: value.as_bytes().to_vec(),
    }
}
