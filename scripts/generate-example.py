#!/usr/bin/env python3
"""
Generate example files by copying one file from each provider's default directory.
This script scans the default locations for Claude Code, Gemini CLI, Codex, and Cursor Agent,
and copies one representative file from each to the examples/ directory.
For JSON/JSONL files, only the top 20 rows are copied.
"""

import os
import shutil
import glob
import json
import argparse
from pathlib import Path
from typing import List, Optional, Dict


# Provider configurations matching src/models/provider/config/*.rs
PROVIDERS = {
    "claude-code": {
        "name": "Claude Code",
        "default_dir": "~/.claude/projects",
        "patterns": ["*.jsonl", "*claude-code*.json*"],
        "example_name": "claude.jsonl",
    },
    "gemini": {
        "name": "Gemini CLI",
        "default_dir": "~/.gemini/tmp",
        "patterns": ["session-*.json"],
        "example_name": "gemini.json",
    },
    "codex": {
        "name": "Codex",
        "default_dir": "~/.codex/sessions",
        "patterns": ["*.jsonl"],
        "example_name": "codex.jsonl",
    },
    "cursor": {
        "name": "Cursor Agent",
        "default_dir": "~/.cursor/chats",
        "patterns": ["store.db", "*cursor*.db"],
        "example_name": "cursor.db",
    },
}


def find_largest_matching_file(directory: Path, patterns: List[str]) -> Optional[Path]:
    """Find the largest file matching any of the given patterns in the directory."""
    if not directory.exists():
        return None

    all_matches = []
    for pattern in patterns:
        # Search recursively for matching files
        matches = list(directory.glob(f"**/{pattern}"))
        all_matches.extend(matches)

    if not all_matches:
        return None

    # Sort by file size (largest first) and return the largest
    all_matches.sort(key=lambda p: p.stat().st_size, reverse=True)
    return all_matches[0]


def filter_json_file(source: Path, dest: Path, max_lines: int = 20) -> bool:
    """Filter JSON file to include only the first N items (if array) or lines."""
    try:
        with open(source, 'r', encoding='utf-8') as f:
            data = json.load(f)

        # If it's an array, take only the first N items
        if isinstance(data, list):
            filtered_data = data[:max_lines]
            with open(dest, 'w', encoding='utf-8') as f:
                json.dump(filtered_data, f, indent=2, ensure_ascii=False)
        else:
            # If it's not an array, just copy the entire object
            with open(dest, 'w', encoding='utf-8') as f:
                json.dump(data, f, indent=2, ensure_ascii=False)

        return True
    except Exception as e:
        print(f"  Warning: Could not parse as JSON: {e}")
        return False


def filter_jsonl_file(source: Path, dest: Path, max_lines: int = 20) -> bool:
    """Filter JSONL file to include only the first N lines."""
    try:
        with open(source, 'r', encoding='utf-8') as src_f:
            with open(dest, 'w', encoding='utf-8') as dest_f:
                for i, line in enumerate(src_f):
                    if i >= max_lines:
                        break
                    dest_f.write(line)
        return True
    except Exception as e:
        print(f"  Warning: Could not filter JSONL: {e}")
        return False


def copy_example_file(source: Path, dest_dir: Path, example_name: str) -> bool:
    """Copy a file to the examples directory with the given name.
    For JSON/JSONL files, filter to top 20 rows."""
    dest_dir.mkdir(parents=True, exist_ok=True)
    dest_path = dest_dir / example_name

    try:
        # Check file extension and filter if needed
        suffix = source.suffix.lower()

        if suffix == '.jsonl':
            # Filter JSONL to top 20 lines
            if filter_jsonl_file(source, dest_path, max_lines=20):
                print(f"✓ Filtered and copied {source} -> {dest_path} (top 20 lines)")
                return True
            else:
                # Fallback to regular copy if filtering fails
                shutil.copy2(source, dest_path)
                print(f"✓ Copied {source} -> {dest_path} (filtering failed, copied all)")
                return True

        elif suffix == '.json':
            # Filter JSON to top 20 items if array
            if filter_json_file(source, dest_path, max_lines=20):
                print(f"✓ Filtered and copied {source} -> {dest_path} (top 20 items if array)")
                return True
            else:
                # Fallback to regular copy if filtering fails
                shutil.copy2(source, dest_path)
                print(f"✓ Copied {source} -> {dest_path} (filtering failed, copied all)")
                return True

        else:
            # For non-JSON files (like .db), just copy directly
            shutil.copy2(source, dest_path)
            print(f"✓ Copied {source} -> {dest_path}")
            return True

    except Exception as e:
        print(f"✗ Failed to copy {source}: {e}")
        return False


def main():
    """Main function to generate example files."""
    # Parse command-line arguments
    parser = argparse.ArgumentParser(
        description="Generate example files from provider directories"
    )
    parser.add_argument(
        "--prefix",
        type=str,
        default="local",
        help="Prefix for generated example files (default: local)",
    )
    args = parser.parse_args()

    # Get the project root directory (parent of scripts/)
    script_dir = Path(__file__).parent
    project_root = script_dir.parent
    examples_dir = project_root / "examples"

    print("Generating example files...")
    print(f"Examples directory: {examples_dir}")
    print(f"Prefix: {args.prefix}")
    print()

    success_count = 0
    total_count = len(PROVIDERS)

    for provider_id, config in PROVIDERS.items():
        name = config["name"]
        default_dir = Path(config["default_dir"]).expanduser()
        patterns = config["patterns"]
        base_example_name = config["example_name"]

        # Add prefix to example name
        name_parts = base_example_name.rsplit(".", 1)
        if len(name_parts) == 2:
            example_name = f"{args.prefix}_{name_parts[0]}.{name_parts[1]}"
        else:
            example_name = f"{args.prefix}_{base_example_name}"

        print(f"[{name}]")
        print(f"  Scanning: {default_dir}")

        if not default_dir.exists():
            print(f"  ✗ Directory not found")
            print()
            continue

        # Find largest matching file
        source_file = find_largest_matching_file(default_dir, patterns)

        if source_file:
            print(f"  Found: {source_file}")
            if copy_example_file(source_file, examples_dir, example_name):
                success_count += 1
        else:
            print(f"  ✗ No matching files found (patterns: {', '.join(patterns)})")

        print()

    # Summary
    print("=" * 60)
    print(f"Summary: {success_count}/{total_count} example files generated")
    print()

    if success_count > 0:
        print("Generated examples:")
        for example_file in sorted(examples_dir.glob(f"{args.prefix}_*")):
            if example_file.is_file():
                size_kb = example_file.stat().st_size / 1024
                print(f"  - {example_file.name} ({size_kb:.1f} KB)")

    return 0 if success_count == total_count else 1


if __name__ == "__main__":
    exit(main())
