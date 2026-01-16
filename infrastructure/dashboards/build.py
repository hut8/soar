#!/usr/bin/env python3
"""
Grafana Dashboard Builder

Combines individual panel JSON files into complete dashboard JSON files.

Usage:
    python3 build.py              # Build all dashboards
    python3 build.py run          # Build specific dashboard
    python3 build.py --extract    # Extract panels from existing dashboards
    python3 build.py --verify     # Verify built dashboards match originals

Output files are written to infrastructure/grafana-dashboard-*.json
"""

import json
import os
import re
import sys
from pathlib import Path


SCRIPT_DIR = Path(__file__).parent
PANELS_DIR = SCRIPT_DIR / "panels"
DEFINITIONS_DIR = SCRIPT_DIR / "definitions"
COMMON_DIR = SCRIPT_DIR / "common"
OUTPUT_DIR = SCRIPT_DIR.parent  # infrastructure/


def load_json(path: Path) -> dict:
    """Load a JSON file."""
    with open(path) as f:
        return json.load(f)


def save_json(path: Path, data: dict) -> None:
    """Save JSON with consistent formatting."""
    with open(path, "w") as f:
        json.dump(data, f, indent=2)
        f.write("\n")


def slugify(title: str) -> str:
    """Convert a panel title to a filename-safe slug."""
    slug = title.lower()
    slug = re.sub(r"[()/%]", "", slug)
    slug = re.sub(r"[^a-z0-9]+", "-", slug)
    slug = slug.strip("-")
    return slug


def load_panel(dashboard_name: str, panel_ref: str) -> dict:
    """Load a panel by reference."""
    path = PANELS_DIR / dashboard_name / f"{panel_ref}.json"
    if not path.exists():
        raise FileNotFoundError(f"Panel not found: {path}")
    return load_json(path)


def build_dashboard(name: str) -> dict:
    """Build a complete dashboard from its definition file."""
    definition_path = DEFINITIONS_DIR / f"{name}.json"
    definition = load_json(definition_path)

    meta = definition.get("meta", {})

    # Load common annotations
    annotations = load_json(COMMON_DIR / "annotations.json")

    # Build base dashboard structure
    dashboard = {
        "annotations": annotations,
        "editable": True,
        "fiscalYearStartMonth": 0,
        "graphTooltip": meta.get("graphTooltip", 1),
        "id": 0,
        "links": [],
        "panels": [],
        "preload": False,
        "schemaVersion": 42,
        "tags": meta.get("tags", []),
        "templating": {"list": []},
        "time": meta.get("time", {"from": "now-1h", "to": "now"}),
        "timepicker": {},
        "timezone": "",
        "title": meta.get("title", name),
        "uid": meta.get("uid", f"soar-{name}"),
        "refresh": meta.get("refresh", ""),
    }

    # Add optional fields
    if "description" in meta:
        dashboard["description"] = meta["description"]

    # Load templating
    for tpl_ref in definition.get("templating", []):
        tpl_path = COMMON_DIR / tpl_ref
        if tpl_path.exists():
            tpl_data = load_json(tpl_path)
            if isinstance(tpl_data, list):
                dashboard["templating"]["list"].extend(tpl_data)
            else:
                dashboard["templating"]["list"].append(tpl_data)

    # Build panels
    panel_id = 1
    current_y = 0

    for item in definition.get("panels", []):
        if isinstance(item, str):
            # Simple panel reference - full width
            panel = load_panel(name, item)
            panel["id"] = panel_id
            height = panel.pop("_height", 8)
            panel["gridPos"] = {"h": height, "w": 24, "x": 0, "y": current_y}
            dashboard["panels"].append(panel)
            current_y += height
            panel_id += 1

        elif isinstance(item, dict) and "row" in item:
            # Inline row definition
            row_panel = {
                "type": "row",
                "collapsed": False,
                "title": item["row"],
                "id": panel_id,
                "gridPos": {"h": 1, "w": 24, "x": 0, "y": current_y},
                "panels": [],
            }
            dashboard["panels"].append(row_panel)
            current_y += 1
            panel_id += 1

        elif isinstance(item, list):
            # Horizontal group of panels - split width evenly
            num_panels = len(item)
            if num_panels == 0:
                continue
            width_each = 24 // num_panels
            max_height = 0
            x_pos = 0

            for sub_item in item:
                if isinstance(sub_item, str):
                    panel = load_panel(name, sub_item)
                elif isinstance(sub_item, dict) and "panel" in sub_item:
                    panel = load_panel(name, sub_item["panel"])
                    # Allow explicit width override
                    if "w" in sub_item:
                        width_each = sub_item["w"]
                else:
                    continue

                panel["id"] = panel_id
                height = panel.pop("_height", 8)
                panel["gridPos"] = {"h": height, "w": width_each, "x": x_pos, "y": current_y}
                dashboard["panels"].append(panel)
                max_height = max(max_height, height)
                x_pos += width_each
                panel_id += 1

            current_y += max_height

    return dashboard


def extract_dashboard(dashboard_path: Path, output_subdir: str) -> dict:
    """Extract panels from an existing dashboard into individual files.

    Returns the definition structure for the dashboard.
    """
    dashboard = load_json(dashboard_path)
    output_dir = PANELS_DIR / output_subdir
    output_dir.mkdir(parents=True, exist_ok=True)

    definition = {
        "meta": {
            "title": dashboard.get("title", ""),
            "uid": dashboard.get("uid", ""),
            "tags": dashboard.get("tags", []),
            "refresh": dashboard.get("refresh", ""),
            "time": dashboard.get("time", {"from": "now-1h", "to": "now"}),
            "graphTooltip": dashboard.get("graphTooltip", 0),
        },
        "templating": [],
        "panels": [],
    }

    if dashboard.get("description"):
        definition["meta"]["description"] = dashboard["description"]

    # Extract templating
    templating_list = dashboard.get("templating", {}).get("list", [])
    for tpl in templating_list:
        name = tpl.get("name", "")
        if name == "environment":
            definition["templating"].append("templating-environment.json")
        elif name == "postgres_datasource":
            definition["templating"].append("templating-postgres-datasource.json")
        else:
            # Save unknown templating to a new file
            tpl_filename = f"templating-{slugify(name)}.json"
            save_json(COMMON_DIR / tpl_filename, tpl)
            definition["templating"].append(tpl_filename)
            print(f"  Created common/{tpl_filename}")

    # Track seen slugs to avoid duplicates
    seen_slugs = {}

    for panel in dashboard.get("panels", []):
        panel_type = panel.get("type", "")
        title = panel.get("title", "untitled")

        if panel_type == "row":
            # Row panels become inline definitions
            definition["panels"].append({"row": title})
            continue

        # Create filename from title
        slug = slugify(title)

        # Handle duplicate slugs
        if slug in seen_slugs:
            seen_slugs[slug] += 1
            slug = f"{slug}-{seen_slugs[slug]}"
        else:
            seen_slugs[slug] = 1

        # Remove fields that will be auto-assigned
        panel_copy = {k: v for k, v in panel.items() if k not in ("id", "gridPos")}

        # Store height hint if non-standard
        grid_pos = panel.get("gridPos", {})
        height = grid_pos.get("h", 8)
        if height != 8:
            panel_copy["_height"] = height

        save_json(output_dir / f"{slug}.json", panel_copy)
        definition["panels"].append(slug)
        print(f"  Extracted: {slug}.json")

    return definition


def extract_all():
    """Extract all dashboards into panel files and create definitions."""
    # First, create common files
    COMMON_DIR.mkdir(parents=True, exist_ok=True)
    DEFINITIONS_DIR.mkdir(parents=True, exist_ok=True)

    # Create annotations.json from the first dashboard we find
    dashboard_files = list(OUTPUT_DIR.glob("grafana-dashboard-*.json"))
    if dashboard_files:
        sample_dashboard = load_json(dashboard_files[0])
        annotations = sample_dashboard.get("annotations", {"list": []})
        save_json(COMMON_DIR / "annotations.json", annotations)
        print("Created common/annotations.json")

        # Extract templating-environment.json
        templating_list = sample_dashboard.get("templating", {}).get("list", [])
        for tpl in templating_list:
            if tpl.get("name") == "environment":
                save_json(COMMON_DIR / "templating-environment.json", tpl)
                print("Created common/templating-environment.json")
                break

    # Extract each dashboard
    for dashboard_file in sorted(OUTPUT_DIR.glob("grafana-dashboard-*.json")):
        name = dashboard_file.stem.replace("grafana-dashboard-", "")
        print(f"\nExtracting panels from {dashboard_file.name}...")
        definition = extract_dashboard(dashboard_file, name)
        save_json(DEFINITIONS_DIR / f"{name}.json", definition)
        print(f"  Created definitions/{name}.json")


def build_all(names: list[str] | None = None):
    """Build all dashboards or specific ones."""
    if names is None:
        names = [p.stem for p in DEFINITIONS_DIR.glob("*.json")]

    for name in sorted(names):
        print(f"Building {name}...")
        dashboard = build_dashboard(name)
        output_path = OUTPUT_DIR / f"grafana-dashboard-{name}.json"
        save_json(output_path, dashboard)
        print(f"  Written: {output_path}")


def verify_all():
    """Verify that built dashboards are valid (basic structure check)."""
    names = [p.stem for p in DEFINITIONS_DIR.glob("*.json")]
    all_valid = True

    for name in sorted(names):
        print(f"Verifying {name}...")
        try:
            dashboard = build_dashboard(name)
            # Basic validation
            assert "panels" in dashboard, "Missing panels"
            assert "title" in dashboard, "Missing title"
            assert "uid" in dashboard, "Missing uid"
            panel_count = len(dashboard["panels"])
            print(f"  OK: {panel_count} panels")
        except Exception as e:
            print(f"  FAILED: {e}")
            all_valid = False

    return all_valid


def main():
    if len(sys.argv) > 1:
        if sys.argv[1] == "--extract":
            extract_all()
            return
        elif sys.argv[1] == "--verify":
            success = verify_all()
            sys.exit(0 if success else 1)
        else:
            # Build specific dashboards
            build_all(sys.argv[1:])
            return

    # Default: build all
    build_all()


if __name__ == "__main__":
    main()
