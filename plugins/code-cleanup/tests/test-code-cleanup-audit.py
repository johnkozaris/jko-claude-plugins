#!/usr/bin/env python3
"""Regression tests for the repository-agnostic cleanup ledger."""

from __future__ import annotations

import csv
import subprocess
import sys
import tempfile
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "scripts" / "code-cleanup-audit.py"


def run(*args: str, success: bool = True) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        [sys.executable, str(SCRIPT), *args],
        check=False,
        capture_output=True,
        text=True,
    )
    if success and result.returncode != 0:
        raise AssertionError(result.stdout + result.stderr)
    if not success and result.returncode == 0:
        raise AssertionError("command unexpectedly succeeded")
    return result


def git(root: Path, *args: str) -> None:
    subprocess.run(["git", *args], cwd=root, check=True, capture_output=True)


def read_rows(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as source:
        return list(csv.DictReader(source, delimiter="\t"))


def write_rows(path: Path, rows: list[dict[str, str]]) -> None:
    with path.open("w", newline="", encoding="utf-8") as output:
        writer = csv.DictWriter(output, fieldnames=rows[0], delimiter="\t")
        writer.writeheader()
        writer.writerows(rows)


def resolve(row: dict[str, str], decision: str, outcome: str) -> None:
    row["product_role"] = "current product or cleanup outcome"
    row["decision"] = decision
    row["outcome"] = outcome
    row["authority"] = "surviving owner"
    row["evidence"] = "traced from product responsibility through production entry point"
    row["validation"] = "shipping boundary exercised"


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="code-cleanup-audit-") as temporary:
        workspace = Path(temporary)
        repository = workspace / "sample"
        repository.mkdir()
        git(repository, "init", "--quiet")
        git(repository, "config", "user.email", "cleanup@example.invalid")
        git(repository, "config", "user.name", "Cleanup Test")
        (repository / "nested").mkdir()
        (repository / "nested" / "inside.txt").write_text("inside\n")
        (repository / "live.txt").write_text("live\n")
        (repository / "remove.txt").write_text("remove\n")
        git(repository, "add", ".")
        git(repository, "commit", "--quiet", "-m", "fixture")
        (repository / "untracked.txt").write_text("untracked\n")

        ledger = workspace / "audit.tsv"
        run(
            "init",
            "--repo",
            str(repository / "nested"),
            "--ledger",
            str(ledger),
        )
        rows = read_rows(ledger)
        assert {row["path"] for row in rows} == {
            "live.txt",
            "nested/inside.txt",
            "remove.txt",
            "untracked.txt",
        }
        run(
            "check",
            "--repo",
            str(repository),
            "--ledger",
            str(ledger),
            "--require-resolved",
            success=False,
        )

        for row in rows:
            resolve(row, "keep", "verified")
        write_rows(ledger, rows)
        run(
            "check",
            "--repo",
            str(repository),
            "--ledger",
            str(ledger),
            "--require-resolved",
        )

        rows = read_rows(ledger)
        remove_row = next(row for row in rows if row["path"] == "remove.txt")
        resolve(remove_row, "delete", "implemented")
        write_rows(ledger, rows)
        (repository / "remove.txt").unlink()
        git(repository, "add", "-u")
        run("refresh", "--repo", str(repository), "--ledger", str(ledger))
        rows = read_rows(ledger)
        removed = next(row for row in rows if row["path"] == "remove.txt")
        assert removed["lifecycle"] == "deletion-pending"
        run(
            "check",
            "--repo",
            str(repository),
            "--ledger",
            str(ledger),
            "--require-resolved",
        )

        git(repository, "commit", "--quiet", "-m", "remove file")
        run("refresh", "--repo", str(repository), "--ledger", str(ledger))
        rows = read_rows(ledger)
        removed = next(row for row in rows if row["path"] == "remove.txt")
        assert removed["lifecycle"] == "historical"

        (repository / "live.txt").write_text("changed\n")
        stale = run(
            "check",
            "--repo",
            str(repository),
            "--ledger",
            str(ledger),
            success=False,
        )
        assert "identity is stale" in stale.stdout
        run("refresh", "--repo", str(repository), "--ledger", str(ledger))
        changed = next(
            row for row in read_rows(ledger) if row["path"] == "live.txt"
        )
        assert changed["decision"] == "pending"
        assert changed["outcome"] == "pending"

    print("Verified code-cleanup audit inventory, refresh, and resolution checks.")


if __name__ == "__main__":
    main()
