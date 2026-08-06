#!/usr/bin/env python3
"""Audit app layout, compatibility pins, versions, docs, and file lengths."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

import deploy_config


APP_MANIFEST_NAMES = ("manifest.yaml", "manifest.json")
APPS_REPOSITORY = "https://github.com/futex-ai/firna-apps.git"
REQUIRED_PLATFORM_DEPENDENCIES = ("fna-apps-interface", "fna-apps-wasm")
SEMVER_RE = re.compile(
    r"^(0|[1-9][0-9]*)\."
    r"(0|[1-9][0-9]*)\."
    r"(0|[1-9][0-9]*)"
    r"(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?"
    r"(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$"
)
MARKDOWN_LINK_RE = re.compile(r"\[[^\]]+\]\(([^)]+)\)")
STATIC_RUST_INCLUDE_RE = re.compile(
    r'include_(?:str|bytes)!\(\s*"([^"]+)"\s*\)'
)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", default="origin/main")
    parser.add_argument("--root", type=Path)
    args = parser.parse_args()
    root = args.root or Path(__file__).resolve().parents[1]
    failures = audit_repository(root.resolve(), args.base)
    for failure in failures:
        print(f"audit: {failure}", file=sys.stderr)
    return int(bool(failures))


def audit_repository(root: Path, base_ref: str) -> list[str]:
    failures = []
    failures.extend(audit_workspace_metadata(root))
    failures.extend(audit_app_layout(root))
    failures.extend(audit_deploy_config(root))
    failures.extend(audit_platform_pins(root))
    failures.extend(audit_static_rust_includes(root))
    failures.extend(audit_changed_versions(root, base_ref))
    failures.extend(audit_rust_file_lengths(root))
    failures.extend(audit_markdown_links(root))
    return failures


def audit_deploy_config(root: Path) -> list[str]:
    return deploy_config.validate_repository(root)


def audit_workspace_metadata(root: Path) -> list[str]:
    try:
        contents = (root / "Cargo.toml").read_text(encoding="utf-8")
    except OSError as error:
        return [f"cannot read Cargo.toml: {error}"]
    section = re.search(
        r"^\[workspace\.package\]\s*(.*?)(?=^\[|\Z)",
        contents,
        re.MULTILINE | re.DOTALL,
    )
    fields = {} if section is None else toml_string_fields(section.group(1))
    if fields.get("repository") != APPS_REPOSITORY:
        return ["Cargo.toml workspace repository must be the firna-apps repository"]
    return []


def audit_app_layout(root: Path) -> list[str]:
    failures = []
    manifests = sorted(root.glob("apps/*/manifest.*"))
    app_roots = {manifest.parent for manifest in manifests}
    for app_root in sorted(app_roots):
        manifest_paths = [app_root / name for name in APP_MANIFEST_NAMES]
        present = [path for path in manifest_paths if path.is_file()]
        if len(present) != 1:
            failures.append(f"{app_root.relative_to(root)} must contain one manifest")
            continue
        manifest_path = present[0]
        contents = manifest_path.read_text(encoding="utf-8")
        fields = manifest_fields_from_text(contents)
        app_id = fields.get("id")
        version = fields.get("version")
        if app_id != app_root.name:
            failures.append(
                f"{present[0].relative_to(root)} id `{app_id}` must match `{app_root.name}`"
            )
        if version is None or SEMVER_RE.fullmatch(version) is None:
            failures.append(
                f"{manifest_path.relative_to(root)} version `{version}` is not valid semver"
            )
        if manifest_declares_top_level_events(manifest_path, contents):
            failures.append(
                f"{manifest_path.relative_to(root)} must not declare top-level events; "
                "nest events under their owning ingress or omit the key"
            )
        for relative in (
            "README.md",
            "component/Cargo.toml",
            "component/Cargo.lock",
            "component/README.md",
            "tests/platform-runtime/Cargo.toml",
            "tests/platform-runtime/Cargo.lock",
            "tests/platform-runtime/README.md",
        ):
            if not (app_root / relative).is_file():
                failures.append(f"{app_root.relative_to(root)}/{relative} is required")
    return failures


def audit_platform_pins(root: Path) -> list[str]:
    config_path = root / "platform.toml"
    try:
        config = toml_string_fields(config_path.read_text(encoding="utf-8"))
    except OSError as error:
        return [f"cannot read platform.toml: {error}"]
    repository = config.get("repository")
    revision = config.get("revision")
    failures = []
    if not isinstance(repository, str) or not repository.startswith("https://"):
        failures.append("platform.toml repository must be an HTTPS Git URL")
    if not isinstance(revision, str) or re.fullmatch(r"[0-9a-f]{40}", revision) is None:
        failures.append("platform.toml revision must be a full Git commit")
    if failures:
        return failures
    workflow_path = root / ".github/workflows/deploy-apps.yml"
    try:
        workflow = workflow_path.read_text(encoding="utf-8")
    except OSError as error:
        return [f"cannot read {workflow_path.relative_to(root)}: {error}"]
    for name, expected in (
        ("FIRNA_PLATFORM_REPOSITORY", repository),
        ("FIRNA_PLATFORM_REVISION", revision),
    ):
        if workflow_environment_value(workflow, name) != expected:
            failures.append(
                f"{workflow_path.relative_to(root)} {name} must match platform.toml"
            )
    for manifest_path in sorted(root.glob("apps/*/tests/platform-runtime/Cargo.toml")):
        manifest = manifest_path.read_text(encoding="utf-8")
        dependency_names = set(REQUIRED_PLATFORM_DEPENDENCIES)
        dependency_names.update(platform_dependency_names(manifest))
        for name in sorted(dependency_names):
            dependency = inline_dependency_fields(manifest, name)
            relative = manifest_path.relative_to(root)
            if not isinstance(dependency, dict):
                failures.append(f"{relative} must declare Git dependency {name}")
                continue
            if dependency.get("git") != repository or dependency.get("rev") != revision:
                failures.append(f"{relative} {name} must match platform.toml")
            if "path" in dependency:
                failures.append(f"{relative} {name} must not use a local path")
        lock_path = manifest_path.with_name("Cargo.lock")
        lock_relative = lock_path.relative_to(root)
        if not lock_path.is_file():
            failures.append(f"{lock_relative} is required")
            continue
        try:
            lock_text = lock_path.read_text(encoding="utf-8")
        except OSError as error:
            failures.append(f"cannot read {lock_relative}: {error}")
            continue
        expected_source = f"git+{repository}?rev={revision}#{revision}"
        if expected_source not in lock_text:
            failures.append(f"{lock_relative} does not resolve platform.toml")
    return failures


def workflow_environment_value(contents: str, name: str) -> str | None:
    match = re.search(
        rf"^  {re.escape(name)}:[ \t]*(.+?)[ \t]*$",
        contents,
        re.MULTILINE,
    )
    if match is None:
        return None
    return match.group(1).split(" #", maxsplit=1)[0].strip().strip("\"'")


def audit_changed_versions(root: Path, base_ref: str) -> list[str]:
    verify = run_git(root, "rev-parse", "--verify", f"{base_ref}^{{commit}}")
    if verify.returncode != 0:
        return [f"base ref `{base_ref}` is unavailable"]
    changed_paths = set(
        git_paths(root, "diff", "--name-only", "-z", "--diff-filter=ACMRTD", f"{base_ref}...HEAD", "--", "apps")
    )
    changed_paths.update(
        git_paths(root, "diff", "--cached", "--name-only", "-z", "--diff-filter=ACMRTD", "--", "apps")
    )
    changed_paths.update(
        git_paths(root, "ls-files", "-z", "--modified", "--deleted", "--others", "--exclude-standard", "--", "apps")
    )
    app_ids = sorted(
        components[1]
        for path in changed_paths
        if len(components := Path(path).parts) > 2
        and components[0] == "apps"
        and components[2:] != ("deploy.toml",)
    )
    failures = []
    for app_id in sorted(set(app_ids)):
        current_path = current_manifest(root / "apps" / app_id)
        base_path = base_manifest(root, base_ref, app_id)
        if base_path is None:
            continue
        if current_path is None:
            failures.append(f"apps/{app_id} was removed relative to {base_ref}")
            continue
        current_version = manifest_fields(current_path).get("version")
        base_contents = run_git(root, "show", f"{base_ref}:{base_path.as_posix()}")
        if base_contents.returncode != 0:
            failures.append(f"cannot read {base_ref}:{base_path}")
            continue
        base_version = manifest_fields_from_text(base_contents.stdout).get("version")
        if not newer_semver(current_version, base_version):
            failures.append(
                f"apps/{app_id} changed but version `{current_version}` is not above `{base_version}`"
            )
    return failures


def audit_rust_file_lengths(root: Path) -> list[str]:
    failures = []
    for path in rust_source_files(root):
        lines = len(path.read_text(encoding="utf-8").splitlines())
        if lines > 300:
            failures.append(f"{path.relative_to(root)} has {lines} lines; maximum is 300")
    return failures


def audit_static_rust_includes(root: Path) -> list[str]:
    failures = []
    for path in rust_source_files(root):
        contents = path.read_text(encoding="utf-8")
        for target in STATIC_RUST_INCLUDE_RE.findall(contents):
            resolved = (path.parent / target).resolve()
            if not resolved.is_relative_to(root) or not resolved.is_file():
                failures.append(
                    f"{path.relative_to(root)} includes missing file `{target}`"
                )
    return failures


def rust_source_files(root: Path) -> list[Path]:
    paths = []
    for directory in (root / "apps", root / "xtask"):
        paths.extend(
            path
            for path in directory.rglob("*.rs")
            if "target" not in path.relative_to(directory).parts
        )
    return sorted(paths)


def audit_markdown_links(root: Path) -> list[str]:
    failures = []
    for path in sorted([root / "README.md", *root.glob("apps/**/*.md"), *root.glob("xtask/*.md")]):
        for target in MARKDOWN_LINK_RE.findall(path.read_text(encoding="utf-8")):
            target = target.strip().strip("<>").split("#", 1)[0]
            if not target or "://" in target or target.startswith("mailto:"):
                continue
            if not (path.parent / target).resolve().exists():
                failures.append(f"{path.relative_to(root)} has missing link `{target}`")
    return failures


def manifest_fields(path: Path) -> dict[str, str]:
    return manifest_fields_from_text(path.read_text(encoding="utf-8"))


def manifest_declares_top_level_events(path: Path, contents: str) -> bool:
    if path.suffix == ".json":
        try:
            document = json.loads(contents)
        except json.JSONDecodeError:
            return False
        return isinstance(document, dict) and "events" in document
    return re.search(
        r'''^(?:events|["']events["'])[ \t]*:''', contents, re.MULTILINE
    ) is not None


def manifest_fields_from_text(contents: str) -> dict[str, str]:
    fields = {}
    for line in contents.splitlines():
        if line.startswith(("id:", "version:")):
            key, value = line.split(":", 1)
            fields[key] = value.strip().strip('"\'')
    return fields


def toml_string_fields(contents: str) -> dict[str, str]:
    return dict(
        re.findall(r'^([a-z][a-z0-9_]*)\s*=\s*"([^"]*)"\s*$', contents, re.MULTILINE)
    )


def inline_dependency_fields(contents: str, name: str) -> dict[str, str] | None:
    match = re.search(
        rf"^{re.escape(name)}\s*=\s*\{{([^}}]+)\}}\s*$", contents, re.MULTILINE
    )
    if match is None:
        return None
    return dict(re.findall(r'([a-z]+)\s*=\s*"([^"]*)"', match.group(1)))


def platform_dependency_names(contents: str) -> set[str]:
    return set(re.findall(r"^(fna-[a-z0-9-]+)\s*=", contents, re.MULTILINE))


def current_manifest(app_root: Path) -> Path | None:
    return next((app_root / name for name in APP_MANIFEST_NAMES if (app_root / name).is_file()), None)


def base_manifest(root: Path, base_ref: str, app_id: str) -> Path | None:
    for name in APP_MANIFEST_NAMES:
        path = Path("apps") / app_id / name
        if run_git(root, "cat-file", "-e", f"{base_ref}:{path.as_posix()}").returncode == 0:
            return path
    return None


def newer_semver(current: str | None, base: str | None) -> bool:
    if current is None or base is None:
        return False
    if SEMVER_RE.fullmatch(current) is None or SEMVER_RE.fullmatch(base) is None:
        return False
    return semver_key(current) > semver_key(base)


def semver_key(value: str) -> tuple[int, int, int, tuple[tuple[int, object], ...]]:
    core_and_pre = value.split("+", 1)[0].split("-", 1)
    core = tuple(int(part) for part in core_and_pre[0].split("."))
    if len(core_and_pre) == 1:
        prerelease = ((2, ""),)
    else:
        prerelease = tuple(
            (0, int(part)) if part.isdigit() else (1, part)
            for part in core_and_pre[1].split(".")
        )
    return (*core, prerelease)


def git_paths(root: Path, *args: str) -> list[str]:
    result = subprocess.run(
        ["git", *args], cwd=root, check=True, capture_output=True
    )
    return [path.decode() for path in result.stdout.split(b"\0") if path]


def run_git(root: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args], cwd=root, check=False, capture_output=True, text=True
    )


if __name__ == "__main__":
    raise SystemExit(main())
