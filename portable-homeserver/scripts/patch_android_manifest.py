#!/usr/bin/env python3
"""Ensure the generated Android manifest exposes the network capabilities required by the testnet."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import xml.etree.ElementTree as ET

NETWORK_SECURITY_CONFIG = """<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<network-security-config>\n    <domain-config cleartextTrafficPermitted=\"true\">\n        <domain>127.0.0.1</domain>\n        <domain>localhost</domain>\n    </domain-config>\n</network-security-config>\n"""

ANDROID_NS = "http://schemas.android.com/apk/res/android"
NS_ATTR = "{" + ANDROID_NS + "}"

# Make sure ElementTree preserves the android namespace prefix when writing the file.
ET.register_namespace("android", ANDROID_NS)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "manifest",
        type=Path,
        help="Path to the generated app/src/main/AndroidManifest.xml file",
    )
    return parser.parse_args()


def load_manifest(path: Path) -> ET.ElementTree:
    try:
        return ET.parse(path)
    except FileNotFoundError as exc:
        raise SystemExit(f"Android manifest not found at {path}") from exc
    except ET.ParseError as exc:
        raise SystemExit(f"Failed to parse Android manifest at {path}: {exc}") from exc


def ensure_permission(manifest: ET.Element, permission: str) -> None:
    """Add the requested uses-permission if it is not already present."""
    android_name = NS_ATTR + "name"
    for node in manifest.findall("uses-permission"):
        if node.get(android_name) == permission:
            return

    new_node = ET.Element("uses-permission")
    new_node.set(android_name, permission)

    # Keep uses-permission elements grouped before the <application> node when possible.
    children = list(manifest)
    insert_index = len(children)
    for idx, node in enumerate(children):
        if node.tag == "application":
            insert_index = idx
            break
    manifest.insert(insert_index, new_node)


def ensure_application_attribute(manifest: ET.Element, attribute: str, value: str) -> None:
    application = manifest.find("application")
    if application is None:
        raise SystemExit("Manifest does not contain an <application> element to patch")

    android_attr = NS_ATTR + attribute
    if application.get(android_attr) != value:
        application.set(android_attr, value)


def remove_application_attribute(manifest: ET.Element, attribute: str) -> None:
    application = manifest.find("application")
    if application is None:
        raise SystemExit("Manifest does not contain an <application> element to patch")

    android_attr = NS_ATTR + attribute
    if android_attr in application.attrib:
        del application.attrib[android_attr]


def ensure_network_security_config(manifest_path: Path) -> None:
    main_dir = manifest_path.parent
    res_dir = main_dir / "res" / "xml"
    res_dir.mkdir(parents=True, exist_ok=True)

    config_path = res_dir / "network_security_config.xml"
    current = None
    try:
        current = config_path.read_text(encoding="utf-8")
    except FileNotFoundError:
        current = None
    except OSError as exc:
        raise SystemExit(
            f"Failed to read existing network security config at {config_path}: {exc}"
        ) from exc

    if current == NETWORK_SECURITY_CONFIG:
        return

    try:
        config_path.write_text(NETWORK_SECURITY_CONFIG, encoding="utf-8")
    except OSError as exc:
        raise SystemExit(
            f"Failed to write network security config to {config_path}: {exc}"
        ) from exc


def main() -> None:
    args = parse_args()
    tree = load_manifest(args.manifest)
    manifest = tree.getroot()

    ensure_permission(manifest, "android.permission.INTERNET")
    ensure_permission(manifest, "android.permission.ACCESS_NETWORK_STATE")
    remove_application_attribute(manifest, "usesCleartextTraffic")
    ensure_application_attribute(manifest, "networkSecurityConfig", "@xml/network_security_config")
    ensure_network_security_config(args.manifest)

    if hasattr(ET, "indent"):
        ET.indent(tree, space="    ")  # type: ignore[attr-defined]

    try:
        tree.write(args.manifest, encoding="utf-8", xml_declaration=True)
    except OSError as exc:
        raise SystemExit(f"Failed to write patched manifest to {args.manifest}: {exc}") from exc
    else:
        # Ensure the manifest ends with a trailing newline so repeated runs stay idempotent.
        with args.manifest.open("rb+") as handle:
            handle.seek(0, os.SEEK_END)
            size = handle.tell()
            if size == 0:
                return
            handle.seek(-1, os.SEEK_END)
            if handle.read(1) != b"\n":
                handle.write(b"\n")


if __name__ == "__main__":
    main()
