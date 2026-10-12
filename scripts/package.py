"""Package a locally built native release; no signing credentials required."""
import pathlib
import platform
import plistlib
import shutil
import sys

root = pathlib.Path(__file__).resolve().parent.parent
dist = root / "dist"
dist.mkdir(exist_ok=True)
name = f"self-devkits-{platform.system().lower()}-{platform.machine().lower()}"
stage = dist / name
stage.mkdir(exist_ok=True)
binary = root / "target" / "release" / ("self-devkits.exe" if sys.platform == "win32" else "self-devkits")
if sys.platform == "darwin":
    contents = stage / "Self DevKits.app" / "Contents"
    (contents / "MacOS").mkdir(parents=True, exist_ok=True)
    shutil.copy2(binary, contents / "MacOS" / "self-devkits")
    with (contents / "Info.plist").open("wb") as f:
        plistlib.dump({
            "CFBundleName": "Self DevKits",
            "CFBundleDisplayName": "Self DevKits",
            "CFBundleExecutable": "self-devkits",
            "CFBundleIdentifier": "dev.selfdevkits.desktop",
            "CFBundlePackageType": "APPL",
            "CFBundleShortVersionString": "0.1.0",
            "CFBundleVersion": "1",
            "NSHighResolutionCapable": True,
        }, f)
else:
    shutil.copy2(binary, stage / binary.name)
for filename in ("README.md", "LICENSE"):
    shutil.copy2(root / filename, stage / filename)
archive = shutil.make_archive(str(dist / name), "gztar" if sys.platform.startswith("linux") else "zip", dist, name)
print(archive)
