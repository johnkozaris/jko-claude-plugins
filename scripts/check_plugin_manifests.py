#!/usr/bin/env python3

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[1]
PLUGINS_ROOT = REPO_ROOT / "plugins"

MARKETPLACE_PATHS = [
    REPO_ROOT / ".github" / "plugin" / "marketplace.json",
    REPO_ROOT / ".claude-plugin" / "marketplace.json",
]

METADATA_KEYS = {
    "name",
    "description",
    "version",
    "license",
    "author",
    "homepage",
    "repository",
    "keywords",
    "category",
    "tags",
}

COMPONENT_KEYS = {
    "commands",
    "agents",
    "skills",
    "hooks",
    "mcpServers",
    "lspServers",
    "outputStyles",
}

KEBAB_CASE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")


def format_path(path: Path) -> str:
    try:
        return str(path.relative_to(REPO_ROOT))
    except ValueError:
        return str(path)


def load_json(path: Path, errors: list[str]) -> dict[str, Any] | None:
    if not path.is_file():
        errors.append(f"Missing JSON file: {format_path(path)}")
        return None

    try:
        data = json.loads(path.read_text())
    except json.JSONDecodeError as exc:
        errors.append(f"Invalid JSON in {format_path(path)}: {exc}")
        return None

    if not isinstance(data, dict):
        errors.append(f"Expected a JSON object in {format_path(path)}")
        return None

    return data


def ensure_kebab_case(name: str, context: str, errors: list[str]) -> None:
    if not KEBAB_CASE.match(name):
        errors.append(f"{context} must be kebab-case, got {name!r}")


def validate_relative_path(
    plugin_dir: Path,
    manifest_path: Path,
    key: str,
    value: str,
    errors: list[str],
) -> None:
    if not value.startswith("./"):
        errors.append(
            f"{format_path(manifest_path)} field {key!r} must use a ./ relative path, got {value!r}"
        )
        return

    target = plugin_dir / value[2:]
    if not target.exists():
        errors.append(
            f"{format_path(manifest_path)} field {key!r} points to missing path {value!r}"
        )


def validate_component_paths(
    plugin_dir: Path,
    manifest_path: Path,
    manifest: dict[str, Any],
    errors: list[str],
) -> None:
    for key in COMPONENT_KEYS:
        value = manifest.get(key)
        if value is None:
            continue

        if isinstance(value, str):
            validate_relative_path(plugin_dir, manifest_path, key, value, errors)
            continue

        if isinstance(value, list):
            for item in value:
                if not isinstance(item, str):
                    errors.append(
                        f"{format_path(manifest_path)} field {key!r} must contain only string paths"
                    )
                    continue
                validate_relative_path(plugin_dir, manifest_path, key, item, errors)
            continue

        if key in {"hooks", "mcpServers", "lspServers"} and isinstance(value, dict):
            continue

        errors.append(
            f"{format_path(manifest_path)} field {key!r} has unsupported value type {type(value).__name__}"
        )


def validate_skill_directory(plugin_dir: Path, errors: list[str]) -> None:
    skills_dir = plugin_dir / "skills"
    if not skills_dir.is_dir():
        return

    skill_dirs = sorted(path for path in skills_dir.iterdir() if path.is_dir())
    if not skill_dirs:
        errors.append(f"{format_path(skills_dir)} exists but contains no skill directories")
        return

    for skill_dir in skill_dirs:
        if not any(skill_dir.iterdir()):
            continue
        skill_file = skill_dir / "SKILL.md"
        if not skill_file.is_file():
            errors.append(f"Missing SKILL.md in {format_path(skill_dir)}")


def validate_commands_directory(plugin_dir: Path, errors: list[str]) -> None:
    commands_dir = plugin_dir / "commands"
    if not commands_dir.is_dir():
        return

    if not any(commands_dir.glob("*.md")):
        errors.append(
            f"{format_path(commands_dir)} exists but does not contain any command markdown files"
        )


def validate_plugin(plugin_dir: Path, errors: list[str]) -> None:
    plugin_name = plugin_dir.name
    ensure_kebab_case(plugin_name, f"Plugin directory {plugin_name!r}", errors)

    copilot_manifest_path = plugin_dir / ".github" / "plugin" / "plugin.json"
    claude_manifest_path = plugin_dir / ".claude-plugin" / "plugin.json"

    copilot_manifest = load_json(copilot_manifest_path, errors)
    claude_manifest = load_json(claude_manifest_path, errors)
    if copilot_manifest is None or claude_manifest is None:
        return

    if copilot_manifest.get("name") != plugin_name:
        errors.append(
            f"{format_path(copilot_manifest_path)} name must match plugin directory {plugin_name!r}"
        )
    if claude_manifest.get("name") != plugin_name:
        errors.append(
            f"{format_path(claude_manifest_path)} name must match plugin directory {plugin_name!r}"
        )

    ensure_kebab_case(
        str(copilot_manifest.get("name", "")),
        f"{format_path(copilot_manifest_path)} name",
        errors,
    )
    ensure_kebab_case(
        str(claude_manifest.get("name", "")),
        f"{format_path(claude_manifest_path)} name",
        errors,
    )

    copilot_metadata = {
        key: copilot_manifest[key]
        for key in sorted(METADATA_KEYS)
        if key in copilot_manifest
    }
    claude_metadata = {
        key: claude_manifest[key]
        for key in sorted(METADATA_KEYS)
        if key in claude_manifest
    }
    if copilot_metadata != claude_metadata:
        errors.append(
            f"Metadata mismatch between {format_path(copilot_manifest_path)} and {format_path(claude_manifest_path)}"
        )

    claude_component_keys = sorted(key for key in claude_manifest if key in COMPONENT_KEYS)
    if claude_component_keys:
        errors.append(
            f"{format_path(claude_manifest_path)} must stay metadata-only, found component keys: {', '.join(claude_component_keys)}"
        )

    validate_component_paths(plugin_dir, copilot_manifest_path, copilot_manifest, errors)

    has_skills = (plugin_dir / "skills").is_dir()
    has_commands = (plugin_dir / "commands").is_dir()

    if has_skills and copilot_manifest.get("skills") != "./skills":
        errors.append(
            f"{format_path(copilot_manifest_path)} must declare \"skills\": \"./skills\" when skills/ exists"
        )
    if not has_skills and "skills" in copilot_manifest:
        errors.append(
            f"{format_path(copilot_manifest_path)} declares skills but {format_path(plugin_dir / 'skills')} does not exist"
        )

    if has_commands and copilot_manifest.get("commands") != "./commands":
        errors.append(
            f"{format_path(copilot_manifest_path)} must declare \"commands\": \"./commands\" when commands/ exists"
        )
    if not has_commands and "commands" in copilot_manifest:
        errors.append(
            f"{format_path(copilot_manifest_path)} declares commands but {format_path(plugin_dir / 'commands')} does not exist"
        )

    validate_skill_directory(plugin_dir, errors)
    validate_commands_directory(plugin_dir, errors)


def validate_marketplaces(plugin_names: list[str], errors: list[str]) -> None:
    marketplace_docs = [load_json(path, errors) for path in MARKETPLACE_PATHS]
    if any(doc is None for doc in marketplace_docs):
        return

    github_marketplace, claude_marketplace = marketplace_docs
    assert github_marketplace is not None
    assert claude_marketplace is not None

    if github_marketplace != claude_marketplace:
        errors.append(
            "Root marketplace manifests must stay identical between "
            f"{format_path(MARKETPLACE_PATHS[0])} and {format_path(MARKETPLACE_PATHS[1])}"
        )

    marketplace_name = str(github_marketplace.get("name", ""))
    ensure_kebab_case(
        marketplace_name,
        f"{format_path(MARKETPLACE_PATHS[0])} name",
        errors,
    )

    plugins = github_marketplace.get("plugins")
    if not isinstance(plugins, list):
        errors.append(f"{format_path(MARKETPLACE_PATHS[0])} must contain a plugins array")
        return

    seen_names: set[str] = set()
    marketplace_plugin_names: list[str] = []
    for entry in plugins:
        if not isinstance(entry, dict):
            errors.append(
                f"{format_path(MARKETPLACE_PATHS[0])} plugins entries must be JSON objects"
            )
            continue

        name = entry.get("name")
        source = entry.get("source")
        if not isinstance(name, str):
            errors.append(
                f"{format_path(MARKETPLACE_PATHS[0])} plugin entry is missing a string name"
            )
            continue

        ensure_kebab_case(name, f"{format_path(MARKETPLACE_PATHS[0])} plugin name", errors)

        if name in seen_names:
            errors.append(
                f"{format_path(MARKETPLACE_PATHS[0])} contains duplicate plugin entry {name!r}"
            )
        seen_names.add(name)
        marketplace_plugin_names.append(name)

        expected_source = f"./plugins/{name}"
        if source != expected_source:
            errors.append(
                f"{format_path(MARKETPLACE_PATHS[0])} plugin {name!r} must use source {expected_source!r}, got {source!r}"
            )

    if sorted(plugin_names) != sorted(marketplace_plugin_names):
        errors.append(
            "Marketplace plugin list must match plugins/ directories exactly. "
            f"Dirs={sorted(plugin_names)!r}, marketplace={sorted(marketplace_plugin_names)!r}"
        )


def main() -> int:
    errors: list[str] = []

    if not PLUGINS_ROOT.is_dir():
        print("plugins/ directory not found", file=sys.stderr)
        return 1

    plugin_dirs = sorted(path for path in PLUGINS_ROOT.iterdir() if path.is_dir())
    plugin_names = [path.name for path in plugin_dirs]

    validate_marketplaces(plugin_names, errors)
    for plugin_dir in plugin_dirs:
        validate_plugin(plugin_dir, errors)

    if errors:
        print("Manifest validation failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print(
        f"Validated {len(MARKETPLACE_PATHS)} marketplace manifests and {len(plugin_dirs)} plugin layouts successfully."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
