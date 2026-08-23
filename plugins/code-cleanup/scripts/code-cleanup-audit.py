#!/usr/bin/env python3
"""Create and verify a hash-bound every-file code-cleanup ledger."""

from __future__ import annotations

import argparse
import csv
import hashlib
import os
import stat
import subprocess
import sys
import tempfile
from dataclasses import asdict, dataclass
from pathlib import Path


HEADER = (
    "repository",
    "path",
    "sha256",
    "source_state",
    "source_mode",
    "source_type",
    "cohort",
    "lifecycle",
    "product_role",
    "decision",
    "outcome",
    "authority",
    "evidence",
    "validation",
)
DECISIONS = {"pending", "keep", "simplify", "move", "delete"}
OUTCOMES = {"pending", "verified", "implemented"}
IMPLEMENTED_DECISIONS = {"simplify", "move", "delete"}
SUPPORTED_MODES = {"100644", "100755", "120000", "160000"}


class AuditError(RuntimeError):
    pass


@dataclass(frozen=True)
class GitObject:
    mode: str
    object_id: str


@dataclass(frozen=True)
class Repository:
    name: str
    root: Path


@dataclass(frozen=True)
class Entry:
    repository: str
    path: str
    sha256: str
    source_state: str
    source_mode: str
    source_type: str
    cohort: str
    lifecycle: str

    @property
    def identity(self) -> tuple[str, str, str, str, str]:
        return (
            self.repository,
            self.path,
            self.sha256,
            self.source_mode,
            self.source_type,
        )


@dataclass
class Row:
    repository: str
    path: str
    sha256: str
    source_state: str
    source_mode: str
    source_type: str
    cohort: str
    lifecycle: str
    product_role: str
    decision: str
    outcome: str
    authority: str
    evidence: str
    validation: str

    @classmethod
    def pending(cls, entry: Entry) -> "Row":
        return cls(
            **asdict(entry),
            product_role="",
            decision="pending",
            outcome="pending",
            authority="",
            evidence="",
            validation="",
        )

    @property
    def identity(self) -> tuple[str, str, str, str, str]:
        return (
            self.repository,
            self.path,
            self.sha256,
            self.source_mode,
            self.source_type,
        )


def run_git(root: Path, *args: str, check: bool = True) -> bytes:
    result = subprocess.run(
        ["git", *args],
        cwd=root,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if check and result.returncode != 0:
        detail = result.stderr.decode("utf-8", "replace").strip()
        raise AuditError(detail or f"git {' '.join(args)} failed in {root}")
    return result.stdout


def parse_repository(value: str) -> Repository:
    if "=" in value:
        name, raw_path = value.split("=", 1)
        if not name:
            raise AuditError(f"repository name is empty: {value!r}")
    else:
        raw_path = value
        name = ""
    requested = Path(raw_path).expanduser().resolve()
    if not requested.is_dir():
        raise AuditError(f"repository directory does not exist: {requested}")
    if run_git(requested, "rev-parse", "--is-inside-work-tree").strip() != b"true":
        raise AuditError(f"not a Git worktree: {requested}")
    root = Path(
        run_git(requested, "rev-parse", "--show-toplevel")
        .decode("utf-8", "surrogateescape")
        .strip()
    ).resolve()
    if not name:
        name = root.name
    return Repository(name=name, root=root)


def parse_repositories(values: list[str]) -> list[Repository]:
    repositories = [parse_repository(value) for value in values]
    names = [repository.name for repository in repositories]
    roots = [repository.root for repository in repositories]
    if len(names) != len(set(names)):
        raise AuditError("repository names must be unique; use NAME=PATH")
    if len(roots) != len(set(roots)):
        raise AuditError("repository paths must be unique")
    return repositories


def validate_path(path: str) -> None:
    candidate = Path(path)
    if candidate.is_absolute() or not candidate.parts:
        raise AuditError(f"unsafe repository path: {path!r}")
    if any(part in {"", ".", ".."} for part in candidate.parts):
        raise AuditError(f"unsafe repository path: {path!r}")
    if "\t" in path or "\n" in path or "\r" in path:
        raise AuditError(f"path cannot be represented safely in TSV: {path!r}")


def object_map(root: Path, revision: str) -> dict[str, GitObject]:
    exists = subprocess.run(
        ["git", "rev-parse", "--verify", revision],
        cwd=root,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    if exists.returncode != 0:
        return {}
    objects: dict[str, GitObject] = {}
    for record in run_git(root, "ls-tree", "-rz", "--full-tree", revision).split(b"\0"):
        if not record:
            continue
        metadata, raw_path = record.split(b"\t", 1)
        mode, _object_type, object_id = metadata.decode().split(" ", 2)
        path = raw_path.decode("utf-8", "surrogateescape")
        validate_path(path)
        if mode not in SUPPORTED_MODES:
            raise AuditError(f"unsupported Git mode {mode}: {path}")
        objects[path] = GitObject(mode, object_id)
    return objects


def index_map(root: Path) -> dict[str, GitObject]:
    objects: dict[str, GitObject] = {}
    for record in run_git(root, "ls-files", "--stage", "-z").split(b"\0"):
        if not record:
            continue
        metadata, raw_path = record.split(b"\t", 1)
        mode, object_id, stage = metadata.decode().split(" ", 2)
        path = raw_path.decode("utf-8", "surrogateescape")
        validate_path(path)
        if stage != "0":
            raise AuditError(f"unmerged index entry is not auditable: {path}")
        if object_id.strip("0") == "":
            raise AuditError(f"intent-to-add entry is not auditable: {path}")
        if mode not in SUPPORTED_MODES:
            raise AuditError(f"unsupported Git mode {mode}: {path}")
        objects[path] = GitObject(mode, object_id)
    return objects


def untracked_paths(root: Path) -> set[str]:
    paths = set()
    for raw_path in run_git(
        root, "ls-files", "--others", "--exclude-standard", "-z"
    ).split(b"\0"):
        if not raw_path:
            continue
        path = raw_path.decode("utf-8", "surrogateescape")
        validate_path(path)
        paths.add(path)
    return paths


def object_bytes(root: Path, value: GitObject) -> bytes:
    if value.mode == "160000":
        return value.object_id.encode()
    return run_git(root, "cat-file", "blob", value.object_id)


def object_type(mode: str) -> str:
    if mode == "120000":
        return "symlink"
    if mode == "160000":
        return "gitlink"
    return "regular"


def live_bytes(root: Path, path: str) -> tuple[bytes, str, str] | None:
    target = root / path
    try:
        metadata = target.lstat()
    except FileNotFoundError:
        return None
    if stat.S_ISLNK(metadata.st_mode):
        return os.fsencode(os.readlink(target)), "120000", "symlink"
    if stat.S_ISDIR(metadata.st_mode):
        return None
    if not stat.S_ISREG(metadata.st_mode):
        raise AuditError(f"unsupported worktree file type: {path}")
    before = target.stat()
    content = target.read_bytes()
    after = target.stat()
    stable = ("st_dev", "st_ino", "st_mode", "st_size", "st_mtime_ns")
    if any(getattr(before, field) != getattr(after, field) for field in stable):
        raise AuditError(f"file changed while being read: {path}")
    mode = "100755" if before.st_mode & 0o111 else "100644"
    return content, mode, "regular"


def sha256(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def cohort(path: str) -> str:
    parts = Path(path).parts
    return parts[0] if len(parts) > 1 else "root"


def source_state(
    content_hash: str,
    mode: str,
    source_type: str,
    head: GitObject | None,
    index: GitObject | None,
    root: Path,
    in_worktree: bool,
) -> str:
    values: list[str] = []
    for label, value in (("head", head), ("index", index)):
        if value is None:
            continue
        same = (
            sha256(object_bytes(root, value)) == content_hash
            and value.mode == mode
            and object_type(value.mode) == source_type
        )
        values.append(f"{label}:{'same' if same else 'different'}")
    if in_worktree:
        values.append("worktree")
    return "+".join(values)


def excluded_path(repository: Repository, ledger: Path) -> str | None:
    try:
        relative = ledger.resolve().relative_to(repository.root)
    except ValueError:
        return None
    return relative.as_posix()


def snapshot_repository(repository: Repository, ledger: Path) -> list[Entry]:
    head = object_map(repository.root, "HEAD")
    index = index_map(repository.root)
    paths = set(head) | set(index) | untracked_paths(repository.root)
    excluded = excluded_path(repository, ledger)
    if excluded is not None:
        paths.discard(excluded)

    entries: list[Entry] = []
    for path in sorted(paths, key=os.fsencode):
        selected = index.get(path) or head.get(path)
        if selected is not None and selected.mode == "160000":
            content = object_bytes(repository.root, selected)
            mode = selected.mode
            source_type = "gitlink"
            in_worktree = (repository.root / path).is_dir()
            lifecycle = "current" if in_worktree else "deletion-pending"
        elif (live := live_bytes(repository.root, path)) is not None:
            content, mode, source_type = live
            in_worktree = True
            lifecycle = "current"
        else:
            if selected is None:
                continue
            content = object_bytes(repository.root, selected)
            mode = selected.mode
            source_type = object_type(mode)
            in_worktree = False
            lifecycle = "deletion-pending"
        content_hash = sha256(content)
        entries.append(
            Entry(
                repository=repository.name,
                path=path,
                sha256=content_hash,
                source_state=source_state(
                    content_hash,
                    mode,
                    source_type,
                    head.get(path),
                    index.get(path),
                    repository.root,
                    in_worktree,
                ),
                source_mode=mode,
                source_type=source_type,
                cohort=cohort(path),
                lifecycle=lifecycle,
            )
        )
    return entries


def snapshot(repositories: list[Repository], ledger: Path) -> list[Entry]:
    entries = [
        entry
        for repository in repositories
        for entry in snapshot_repository(repository, ledger)
    ]
    return sorted(entries, key=lambda entry: (entry.repository, os.fsencode(entry.path)))


def read_rows(path: Path) -> list[Row]:
    if not path.is_file():
        raise AuditError(f"ledger does not exist: {path}")
    with path.open(newline="", encoding="utf-8") as source:
        reader = csv.DictReader(source, delimiter="\t")
        if tuple(reader.fieldnames or ()) != HEADER:
            raise AuditError("ledger columns do not match the current schema")
        return [Row(**row) for row in reader]


def write_rows(path: Path, rows: list[Row]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{path.name}.", suffix=".tmp", dir=path.parent
    )
    try:
        with os.fdopen(descriptor, "w", newline="", encoding="utf-8") as output:
            writer = csv.DictWriter(output, fieldnames=HEADER, delimiter="\t")
            writer.writeheader()
            writer.writerows(asdict(row) for row in rows)
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary_name, path)
    finally:
        try:
            os.unlink(temporary_name)
        except FileNotFoundError:
            pass


def repository_names(rows: list[Row]) -> set[str]:
    return {row.repository for row in rows}


def refresh_rows(entries: list[Entry], existing: list[Row]) -> list[Row]:
    expected_names = {entry.repository for entry in entries}
    existing_names = repository_names(existing)
    if existing_names and expected_names != existing_names:
        raise AuditError(
            "ledger repository names differ from --repo arguments: "
            f"ledger={sorted(existing_names)}, requested={sorted(expected_names)}"
        )

    by_identity: dict[tuple[str, str, str, str, str], Row] = {}
    for row in existing:
        if row.identity in by_identity:
            raise AuditError(
                f"duplicate ledger identity: {row.repository}:{row.path}:{row.sha256}"
            )
        by_identity[row.identity] = row

    current_paths = {(entry.repository, entry.path) for entry in entries}
    refreshed: list[Row] = []
    for entry in entries:
        prior = by_identity.get(entry.identity)
        if prior is None:
            refreshed.append(Row.pending(entry))
            continue
        prior.source_state = entry.source_state
        prior.cohort = entry.cohort
        prior.lifecycle = entry.lifecycle
        refreshed.append(prior)

    for row in existing:
        key = (row.repository, row.path)
        if key in current_paths:
            continue
        row.lifecycle = "historical"
        refreshed.append(row)

    return sorted(
        refreshed,
        key=lambda row: (
            row.repository,
            os.fsencode(row.path),
            row.lifecycle == "historical",
            row.sha256,
        ),
    )


def validate_rows(entries: list[Entry], rows: list[Row], require_resolved: bool) -> list[str]:
    errors: list[str] = []
    identities: set[tuple[str, str, str, str, str, str]] = set()
    current_rows: dict[tuple[str, str], list[Row]] = {}
    for row in rows:
        identity = (*row.identity, row.lifecycle)
        if identity in identities:
            errors.append(f"duplicate ledger row: {row.repository}:{row.path}")
        identities.add(identity)
        if row.decision not in DECISIONS:
            errors.append(f"invalid decision for {row.repository}:{row.path}: {row.decision}")
        if row.outcome not in OUTCOMES:
            errors.append(f"invalid outcome for {row.repository}:{row.path}: {row.outcome}")
        if row.lifecycle != "historical":
            current_rows.setdefault((row.repository, row.path), []).append(row)

        if not require_resolved:
            continue
        label = f"{row.repository}:{row.path}"
        if row.decision == "pending" or row.outcome == "pending":
            errors.append(f"pending cleanup decision: {label}")
            continue
        for field in ("product_role", "authority", "evidence", "validation"):
            if not getattr(row, field).strip():
                errors.append(f"resolved row lacks {field}: {label}")
        if row.lifecycle == "historical":
            if row.decision not in IMPLEMENTED_DECISIONS or row.outcome != "implemented":
                errors.append(f"historical row lacks implemented removal outcome: {label}")
        elif row.decision == "keep" and row.outcome != "verified":
            errors.append(f"kept row must be verified: {label}")
        elif row.decision in IMPLEMENTED_DECISIONS and row.outcome != "implemented":
            errors.append(f"cleanup decision is not implemented: {label}")
        if row.lifecycle == "current" and row.decision == "delete":
            errors.append(f"file marked deleted is still present: {label}")
        if row.lifecycle == "deletion-pending" and row.decision != "delete":
            errors.append(f"pending deletion lacks delete decision: {label}")

    expected = {(entry.repository, entry.path): entry for entry in entries}
    for key, entry in expected.items():
        matches = current_rows.get(key, [])
        label = f"{entry.repository}:{entry.path}"
        if not matches:
            errors.append(f"current file is missing from ledger: {label}")
            continue
        if len(matches) != 1:
            errors.append(f"current file has multiple ledger rows: {label}")
            continue
        row = matches[0]
        actual = (
            row.sha256,
            row.source_state,
            row.source_mode,
            row.source_type,
            row.cohort,
            row.lifecycle,
        )
        wanted = (
            entry.sha256,
            entry.source_state,
            entry.source_mode,
            entry.source_type,
            entry.cohort,
            entry.lifecycle,
        )
        if actual != wanted:
            errors.append(f"current file identity is stale: {label}")

    for key in sorted(set(current_rows) - set(expected)):
        errors.append(f"stale current ledger row: {key[0]}:{key[1]}")
    return errors


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(
        description="Create or verify a hash-bound every-file code-cleanup ledger."
    )
    commands = result.add_subparsers(dest="command", required=True)
    for name in ("init", "refresh", "check"):
        command = commands.add_parser(name)
        command.add_argument(
            "--repo",
            action="append",
            required=True,
            metavar="[NAME=]PATH",
            help="Git repository to audit; repeat for cooperating repositories",
        )
        command.add_argument("--ledger", type=Path, required=True)
        if name == "init":
            command.add_argument("--force", action="store_true")
        if name == "check":
            command.add_argument("--require-resolved", action="store_true")
    return result


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    try:
        repositories = parse_repositories(args.repo)
        ledger = args.ledger.expanduser().resolve()
        entries = snapshot(repositories, ledger)

        if args.command == "init":
            if ledger.exists() and not args.force:
                raise AuditError(f"ledger already exists; use --force to replace it: {ledger}")
            rows = [Row.pending(entry) for entry in entries]
            write_rows(ledger, rows)
            print(f"Initialized {len(rows)} cleanup rows in {ledger}")
            return 0

        rows = read_rows(ledger)
        requested_names = {repository.name for repository in repositories}
        if repository_names(rows) != requested_names:
            raise AuditError(
                "ledger repository names differ from --repo arguments: "
                f"ledger={sorted(repository_names(rows))}, "
                f"requested={sorted(requested_names)}"
            )
        if args.command == "refresh":
            refreshed = refresh_rows(entries, rows)
            write_rows(ledger, refreshed)
            pending = sum(
                row.decision == "pending" or row.outcome == "pending"
                for row in refreshed
            )
            print(f"Refreshed {len(refreshed)} cleanup rows ({pending} unresolved)")
            return 0

        errors = validate_rows(entries, rows, args.require_resolved)
        if errors:
            for error in errors:
                print(error)
            return 1
        print(f"Verified {len(entries)} current files against {len(rows)} cleanup rows")
        return 0
    except (AuditError, OSError, csv.Error) as error:
        print(f"code-cleanup audit failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
