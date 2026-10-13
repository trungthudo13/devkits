"""Verify package metadata and layout without installing software on the test host."""
import importlib.util
import hashlib
import json
import plistlib
from unittest.mock import patch
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("devkits_package", ROOT / "scripts/package.py")
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)


class PackagingTests(unittest.TestCase):
    def test_generated_icons_match_source_manifest_and_native_formats(self):
        manifest = json.loads((ROOT / "assets/icons/manifest.json").read_text())
        for path, expected in manifest.items():
            self.assertEqual(hashlib.sha256((ROOT / path).read_bytes()).hexdigest(), expected, path)
        self.assertTrue((ROOT / "assets/icons/devkits.png").read_bytes().startswith(b"\x89PNG\r\n\x1a\n"))
        self.assertTrue((ROOT / "assets/icons/devkits.ico").read_bytes().startswith(b"\x00\x00\x01\x00"))
        self.assertTrue((ROOT / "assets/icons/devkits.icns").read_bytes().startswith(b"icns"))

    def test_macos_bundle_declares_and_contains_icon(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            stage, dist = root / "stage", root / "dist"
            stage.mkdir(); dist.mkdir()
            # Test bundle composition independently of macOS-only signing/disk tools.
            with patch.object(package, "run"), patch.object(Path, "symlink_to"):
                package.macos_package(Path(sys.executable), stage, dist, "arm64")
            app = stage / "volume/DevKits.app/Contents"
            with (app / "Info.plist").open("rb") as handle:
                info = plistlib.load(handle)
            self.assertEqual(info["CFBundleIconFile"], "devkits.icns")
            self.assertEqual((app / "Resources/devkits.icns").read_bytes(), (ROOT / "assets/icons/devkits.icns").read_bytes())

    def test_tag_version_mismatch_fails_before_packaging(self):
        with tempfile.TemporaryDirectory() as temp:
            result = subprocess.run(
                [sys.executable, str(ROOT / "scripts/package.py"), "--binary", sys.executable, "--output-dir", temp],
                env={**os.environ, "GITHUB_REF_TYPE": "tag", "GITHUB_REF_NAME": "v999.999.999"},
                capture_output=True, text=True,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("Release tag must match", result.stderr)
            self.assertEqual(list(Path(temp).iterdir()), [])

    @unittest.skipUnless(sys.platform.startswith("linux") and shutil.which("dpkg-deb") and shutil.which("objdump"), "Debian packaging tools required")
    def test_deb_contains_launcher_dependencies_binary_and_font_license(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            stage, dist = root / "stage", root / "dist"
            stage.mkdir(); dist.mkdir()
            arch = "arm64" if platform.machine() == "aarch64" else "amd64"
            # A real ELF executable makes dependency detection exercise real symbol data.
            binary = Path(sys.executable).resolve()
            artifact = package.linux_package(binary, stage, dist, arch)
            control = subprocess.check_output(["dpkg-deb", "-f", str(artifact)], text=True)
            self.assertIn("Package: devkits", control)
            self.assertIn("poppler-utils", control)
            self.assertIn("tesseract-ocr-vie", control)
            extracted = root / "extracted"
            subprocess.run(["dpkg-deb", "-x", str(artifact), str(extracted)], check=True)
            self.assertEqual((extracted / "usr/bin/devkits").read_bytes(), binary.read_bytes())
            self.assertTrue(os.access(extracted / "usr/bin/devkits", os.X_OK))
            launcher = (extracted / "usr/share/applications/devkits.desktop").read_text()
            self.assertIn("Name=DevKits", launcher)
            self.assertIn("Exec=devkits", launcher)
            self.assertIn("Icon=devkits", launcher)
            self.assertEqual((extracted / "usr/share/icons/hicolor/scalable/apps/devkits.svg").read_bytes(), (ROOT / "assets/devkits.svg").read_bytes())
            self.assertTrue((extracted / "usr/share/doc/devkits/licenses/noto-sans-mono/OFL.txt").is_file())


if __name__ == "__main__":
    unittest.main()
