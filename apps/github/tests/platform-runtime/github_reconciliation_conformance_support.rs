//! Real provider-backed workstream service for package conformance.

use std::sync::Arc;

use fna_db_enums::ExternalRepoState;
use fna_external_repos_interface::{
    ExternalPullRequestChecks, ExternalPullRequestMergeable, ExternalPullRequestReview,
    ExternalPullRequestState, ExternalPullRequestStatus, ExternalRepoDefaults,
    ExternalRepoProviderClientMock, ExternalRepoRecord, ExternalRepoRemote,
    ExternalRepoServiceMock, RepoCredential, RepoCredentialOperation, RepoCredentialSource,
    RepoCredentialSourceServiceMock,
};
use fna_store_interface::workspace::{
    WorkstreamPrChecks, WorkstreamPrMergeable, WorkstreamPrReview, WorkstreamPrState,
    WorkstreamRecord, WorkstreamSnapshot, WorkstreamSnapshotStoreMock, WorkstreamStoreMock,
};
use fna_workstreams::{ConfiguredWorkstreamService, WorkstreamUpdatePublisherMock};
use unimock::{MockFn as _, Unimock, matching};

pub(crate) fn configured_workstream_service(
    refreshed: WorkstreamRecord,
) -> Arc<ConfiguredWorkstreamService> {
    let workspace_id = refreshed.workspace_id;
    let agent_id = refreshed.agent_id;
    let external_repo_id = refreshed
        .repository
        .external_repo_id
        .expect("external repository id");
    let provider_repository_id = refreshed
        .repository
        .provider_repository_id
        .clone()
        .expect("provider repository id");
    let branch_name = refreshed.branch_name.clone();
    let mut binding = refreshed.clone();
    binding.pr_state = WorkstreamPrState::None;
    binding.pr_checks = WorkstreamPrChecks::Unknown;
    binding.pr_review = WorkstreamPrReview::None;
    binding.pr_mergeable = WorkstreamPrMergeable::Unknown;
    binding.pr_url = None;
    binding.pr_number = None;
    let expected_snapshot = WorkstreamSnapshot {
        state: refreshed.pr_state,
        checks: refreshed.pr_checks,
        review: refreshed.pr_review,
        mergeable: refreshed.pr_mergeable,
        url: refreshed.pr_url.clone(),
        number: refreshed.pr_number,
    };
    let repository = ExternalRepoRecord {
        id: external_repo_id,
        workspace_id,
        name: refreshed.repository.name.clone(),
        provider: refreshed.repository.provider.expect("repository provider"),
        remote: ExternalRepoRemote {
            host: String::from("github.com"),
            path: String::from("octo-org/repo"),
        },
        default_branch: refreshed.repository.default_branch.clone(),
        provider_repository_id: provider_repository_id.clone(),
        credential_source: RepoCredentialSource::WorkspaceSecret {
            name: String::from("github-token"),
        },
        state: ExternalRepoState::Active,
        defaults: ExternalRepoDefaults::default(),
        validated_at: refreshed.updated_at,
        archived_at: None,
        created_at: refreshed.created_at,
        updated_at: refreshed.updated_at,
    };
    let status = ExternalPullRequestStatus {
        state: ExternalPullRequestState::Open,
        draft: false,
        review: ExternalPullRequestReview::Approved,
        checks: ExternalPullRequestChecks::Passing,
        mergeable: ExternalPullRequestMergeable::Mergeable,
        url: refreshed.pr_url.clone(),
        number: refreshed.pr_number,
        head_sha: Some(String::from("0123456789abcdef")),
    };
    let workstreams = Arc::new(Unimock::new(
        WorkstreamStoreMock::get_workstream
            .next_call(matching!(_, _))
            .answers_arc(Arc::new(move |_, actual_workspace_id, actual_agent_id| {
                assert_eq!(actual_workspace_id, workspace_id);
                assert_eq!(actual_agent_id, agent_id);
                Ok(binding.clone())
            })),
    ));
    let snapshots = Arc::new(Unimock::new(
        WorkstreamSnapshotStoreMock::replace_workstream_snapshot
            .next_call(matching!(_, _, _, _))
            .answers_arc(Arc::new(
                move |_, actual_workspace_id, actual_agent_id, actual, _| {
                    assert_eq!(actual_workspace_id, workspace_id);
                    assert_eq!(actual_agent_id, agent_id);
                    assert_eq!(actual, expected_snapshot);
                    Ok(refreshed.clone())
                },
            )),
    ));
    let repositories = Arc::new(Unimock::new(
        ExternalRepoServiceMock::get_by_id
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, request| {
                assert_eq!(request.external_repo_id, external_repo_id);
                Ok(repository.clone())
            })),
    ));
    let credentials = Arc::new(Unimock::new(
        RepoCredentialSourceServiceMock::mint
            .next_call(matching!(_, _))
            .answers_arc(Arc::new(move |_, actual_repo_id, operation| {
                assert_eq!(actual_repo_id, external_repo_id);
                assert_eq!(operation, RepoCredentialOperation::PullRequestStatusRead);
                Ok(RepoCredential::new(
                    String::from("x-access-token"),
                    String::from("secret"),
                    None,
                ))
            })),
    ));
    let provider = Arc::new(Unimock::new(
        ExternalRepoProviderClientMock::read_pull_request_status
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, request| {
                assert_eq!(request.repo.provider_repository_id, provider_repository_id);
                assert_eq!(request.head_branch, branch_name);
                assert_eq!(request.credential.username(), "x-access-token");
                Ok(status.clone())
            })),
    ));
    let updates = Arc::new(Unimock::new(
        WorkstreamUpdatePublisherMock::publish_workspace_agents
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, actual_workspace_id| {
                assert_eq!(actual_workspace_id, workspace_id);
            })),
    ));

    Arc::new(
        ConfiguredWorkstreamService::new(
            workstreams,
            snapshots,
            repositories,
            credentials,
            provider,
            None,
        )
        .with_update_publisher(updates),
    )
}
