#!/usr/bin/env python3
"""Strict version-one contracts for the static app preview slot."""

from __future__ import annotations

from dataclasses import asdict, dataclass
from enum import Enum

from app_preview_contract_validation import (
    ContractError,
    require_enum,
    require_object,
    require_optional_enum,
    require_optional_positive_int,
    require_optional_sha,
    require_optional_string,
    require_positive_int,
    require_repository,
    require_run_url,
    require_sha,
    require_string,
    require_version,
)


SOURCE_REPOSITORY = "futex-ai/firna-apps"
PLATFORM_REPOSITORY = "futex-ai/firna"
PRODUCT_URL = "https://br-apps.preview.firna.ai"
API_URL = "https://br-apps.api.preview.firna.ai"
class PreviewAction(str, Enum):
    """Actions accepted by the platform preview receiver."""

    DEPLOY = "deploy"
    RELEASE = "release"


class PreviewStatus(str, Enum):
    """Terminal results returned by the platform preview receiver."""

    READY = "ready"
    FAILED = "failed"
    BUSY = "busy"
    RELEASED = "released"
    SUPERSEDED = "superseded"


class FailureCode(str, Enum):
    """Stable failure reasons returned by the platform preview receiver."""

    INVALID_REQUEST = "invalid_request"
    STALE_REQUEST = "stale_request"
    CI_NOT_GREEN = "ci_not_green"
    SLOT_BUSY = "slot_busy"
    PLATFORM_RELEASE_UNAVAILABLE = "platform_release_unavailable"
    ENVIRONMENT_DEPLOY_FAILED = "environment_deploy_failed"
    APP_SUBMISSION_FAILED = "app_submission_failed"
    SMOKE_FAILED = "smoke_failed"


@dataclass(frozen=True)
class PreviewRequest:
    """One immutable request dispatched to the platform repository."""

    schema_version: int
    action: PreviewAction
    source_repository: str
    pr_number: int
    head_sha: str
    correlation_id: str

    @classmethod
    def create(cls, action: PreviewAction, pr_number: int, head_sha: str) -> PreviewRequest:
        """Create a validated request for the canonical source repository."""

        return parse_request(
            {
                "schema_version": 1,
                "action": action.value,
                "source_repository": SOURCE_REPOSITORY,
                "pr_number": pr_number,
                "head_sha": head_sha,
                "correlation_id": correlation_id(pr_number, head_sha),
            }
        )

    def payload(self) -> dict[str, object]:
        """Return the JSON-compatible repository-dispatch payload."""

        payload = asdict(self)
        payload["action"] = self.action.value
        return payload


@dataclass(frozen=True)
class PreviewResult:
    """One validated terminal result from the platform repository."""

    schema_version: int
    status: PreviewStatus
    source_repository: str
    pr_number: int
    head_sha: str
    correlation_id: str
    platform_sha: str | None
    product_url: str | None
    api_url: str | None
    run_url: str
    failure_code: FailureCode | None
    owner_pr_number: int | None


def correlation_id(pr_number: int, head_sha: str) -> str:
    """Build the stable correlation id for a pull-request candidate."""

    return f"{SOURCE_REPOSITORY}#{pr_number}@{head_sha}"


def parse_request(document: object) -> PreviewRequest:
    """Parse a request, rejecting unknown fields and invalid values."""

    values = require_object(
        document,
        {
            "schema_version",
            "action",
            "source_repository",
            "pr_number",
            "head_sha",
            "correlation_id",
        },
    )
    version = require_version(values)
    action = require_enum(values, "action", PreviewAction)
    repository = require_repository(values, SOURCE_REPOSITORY)
    pr_number = require_positive_int(values, "pr_number")
    head_sha = require_sha(values, "head_sha")
    correlation = require_string(values, "correlation_id")
    if correlation != correlation_id(pr_number, head_sha):
        raise ContractError("correlation_id does not match repository, PR, and SHA")
    return PreviewRequest(
        schema_version=version,
        action=action,
        source_repository=repository,
        pr_number=pr_number,
        head_sha=head_sha,
        correlation_id=correlation,
    )


def parse_result(document: object) -> PreviewResult:
    """Parse a result, enforcing status-dependent nullability."""

    values = require_object(
        document,
        {
            "schema_version",
            "status",
            "source_repository",
            "pr_number",
            "head_sha",
            "correlation_id",
            "platform_sha",
            "product_url",
            "api_url",
            "run_url",
            "failure_code",
            "owner_pr_number",
        },
    )
    version = require_version(values)
    status = require_enum(values, "status", PreviewStatus)
    repository = require_repository(values, SOURCE_REPOSITORY)
    pr_number = require_positive_int(values, "pr_number")
    head_sha = require_sha(values, "head_sha")
    correlation = require_string(values, "correlation_id")
    if correlation != correlation_id(pr_number, head_sha):
        raise ContractError("correlation_id does not match repository, PR, and SHA")
    platform_sha = require_optional_sha(values, "platform_sha")
    product_url = require_optional_string(values, "product_url")
    api_url = require_optional_string(values, "api_url")
    run_url = require_run_url(values)
    failure_code = require_optional_enum(values, "failure_code", FailureCode)
    owner = require_optional_positive_int(values, "owner_pr_number")
    validate_result_state(
        status, pr_number, platform_sha, product_url, api_url, failure_code, owner
    )
    return PreviewResult(
        schema_version=version,
        status=status,
        source_repository=repository,
        pr_number=pr_number,
        head_sha=head_sha,
        correlation_id=correlation,
        platform_sha=platform_sha,
        product_url=product_url,
        api_url=api_url,
        run_url=run_url,
        failure_code=failure_code,
        owner_pr_number=owner,
    )


def validate_result_state(
    status: PreviewStatus,
    pr_number: int,
    platform_sha: str | None,
    product_url: str | None,
    api_url: str | None,
    failure_code: FailureCode | None,
    owner: int | None,
) -> None:
    """Enforce the closed cross-field result invariants."""

    if status is PreviewStatus.READY:
        if platform_sha is None:
            raise ContractError("ready requires platform_sha")
        if product_url != PRODUCT_URL or api_url != API_URL:
            raise ContractError("ready requires the fixed br-apps URLs")
        if owner != pr_number:
            raise ContractError("ready owner_pr_number must equal pr_number")
    elif product_url is not None or api_url is not None:
        raise ContractError("product_url and api_url are only allowed for ready")
    if status is PreviewStatus.BUSY:
        if owner is None or owner == pr_number:
            raise ContractError("busy requires a different owner_pr_number")
    elif status is not PreviewStatus.READY and owner is not None:
        raise ContractError("owner_pr_number is only allowed for ready and busy")
    if status is PreviewStatus.FAILED:
        if failure_code is None:
            raise ContractError("failed requires failure_code")
    elif failure_code is not None:
        raise ContractError("failure_code is only allowed for failed")
