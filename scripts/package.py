"""Build native DevKits installers from a release binary (Python 3.11+)."""
import argparse
import hashlib
import os
import pathlib
import platform
import plistlib
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib

ROOT = pathlib.Path(__file__).resolve().parent.parent
VERSION = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]


def run(*args):
    subprocess.run([str(arg) for arg in args], check=True)


def documents(destination):
    destination.mkdir(parents=True, exist_ok=True)
    for name in ("README.md", "LICENSE"):
        shutil.copy2(ROOT / name, destination / name)
    license_dir = destination / "licenses" / "noto-sans-mono"
    license_dir.mkdir(parents=True, exist_ok=True)
    for name in ("OFL.txt", "README.txt"):
        shutil.copy2(ROOT / "assets/fonts/noto-sans-mono" / name, license_dir / name)


def linux_package(binary, stage, dist, arch):
    package = stage / "deb"
    executable = package / "usr/bin/devkits"
    executable.parent.mkdir(parents=True)
    shutil.copy2(binary, executable)
    executable.chmod(0o755)
    documents(package / "usr/share/doc/devkits")
    desktop = package / "usr/share/applications/devkits.desktop"
    desktop.parent.mkdir(parents=True)
    desktop.write_text("[Desktop Entry]\nType=Application\nName=DevKits\nComment=Offline developer utilities\nExec=devkits\nIcon=devkits\nTerminal=false\nCategories=Development;Utility;\nStartupWMClass=devkits\n")
    icon = package / "usr/share/icons/hicolor/scalable/apps/devkits.svg"
    icon.parent.mkdir(parents=True)
    shutil.copy2(ROOT / "assets/devkits.svg", icon)
    # Match the actual binary, rather than claiming compatibility with older glibc.
    symbols = subprocess.check_output(["objdump", "-T", str(binary)], text=True)
    versions = re.findall(r"GLIBC_(\d+)\.(\d+)", symbols)
    required = max((tuple(map(int, v)) for v in versions), default=(2, 35))
    glibc = ".".join(map(str, required))
    control = package / "DEBIAN/control"
    control.parent.mkdir()
    control.write_text(
        f"Package: devkits\nVersion: {VERSION}\nArchitecture: {arch}\n"
        "Maintainer: DevKits maintainers\nSection: devel\nPriority: optional\n"
        f"Depends: libc6 (>= {glibc}), libgcc-s1, libstdc++6, libxkbcommon0, libxkbcommon-x11-0, libxcb1, libwayland-client0, libx11-6, libxcursor1, libxi6, libxrandr2, libegl1, libgl1, poppler-utils\n"
        "Recommends: xdg-desktop-portal, tesseract-ocr, tesseract-ocr-eng, tesseract-ocr-vie\n"
        "Description: Offline developer utilities\n JSON, XML, SQL, regex, password, QR and PDF tools in a native desktop app.\n"
    )
    artifact = dist / f"DevKits-{VERSION}-linux-{arch}.deb"
    run("dpkg-deb", "--root-owner-group", "--build", package, artifact)
    return artifact


def windows_package(binary, stage, dist, arch):
    payload = stage / "payload"
    documents(payload)
    shutil.copy2(binary, payload / "DevKits.exe")
    shutil.copy2(ROOT / "assets/icons/devkits.ico", payload / "devkits.ico")
    artifact = dist / f"DevKits-{VERSION}-windows-{arch}-setup.exe"
    makensis = shutil.which("makensis")
    if not makensis:
        for folder in (os.environ.get("ProgramFiles(x86)", ""), os.environ.get("ProgramFiles", "")):
            candidate = pathlib.Path(folder) / "NSIS/makensis.exe"
            if candidate.is_file():
                makensis = str(candidate)
                break
    if not makensis:
        raise RuntimeError("NSIS is required: choco install nsis -y")
    run(makensis, "/WX", f"/DVERSION={VERSION}", f"/DPAYLOAD={payload}", f"/DOUTPUT={artifact}", ROOT / "scripts/windows-installer.nsi")
    return artifact


def macos_package(binary, stage, dist, arch):
    volume = stage / "volume"
    app = volume / "DevKits.app/Contents"
    (app / "MacOS").mkdir(parents=True)
    shutil.copy2(binary, app / "MacOS/DevKits")
    (app / "MacOS/DevKits").chmod(0o755)
    documents(app / "Resources")
    shutil.copy2(ROOT / "assets/icons/devkits.icns", app / "Resources/devkits.icns")
    with (app / "Info.plist").open("wb") as handle:
        plistlib.dump({
            "CFBundleName": "DevKits", "CFBundleDisplayName": "DevKits",
            "CFBundleIconFile": "devkits.icns",
            "CFBundleExecutable": "DevKits", "CFBundleIdentifier": "dev.devkits.desktop",
            "CFBundlePackageType": "APPL", "CFBundleShortVersionString": VERSION,
            "CFBundleVersion": VERSION, "NSHighResolutionCapable": True,
            "LSMinimumSystemVersion": "11.0",
        }, handle)
    # Ad-hoc signing makes the bundle internally consistent; it is not notarization.
    run("codesign", "--force", "--deep", "--sign", "-", app.parent)
    (volume / "Applications").symlink_to("/Applications", target_is_directory=True)
    artifact = dist / f"DevKits-{VERSION}-macos-{arch}.dmg"
    run("hdiutil", "create", "-volname", "DevKits", "-srcfolder", volume, "-ov", "-format", "UDZO", artifact)
    run("hdiutil", "verify", artifact)
    return artifact


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=pathlib.Path, help="Override the release binary path")
    parser.add_argument("--output-dir", type=pathlib.Path, default=ROOT / "dist")
    args = parser.parse_args()
    binary = (args.binary or ROOT / "target/release" / ("self-devkits.exe" if sys.platform == "win32" else "self-devkits")).resolve()
    if not binary.is_file():
        parser.error(f"Build the release binary first: {binary}")
    if os.environ.get("GITHUB_REF_TYPE") == "tag" and os.environ.get("GITHUB_REF_NAME") != f"v{VERSION}":
        parser.error(f"Release tag must match Cargo.toml: v{VERSION}")
    dist = args.output_dir.resolve()
    dist.mkdir(parents=True, exist_ok=True)
    machine = platform.machine().lower()
    with tempfile.TemporaryDirectory(prefix="devkits-package-") as temp:
        stage = pathlib.Path(temp)
        if sys.platform.startswith("linux"):
            arch = {"x86_64": "amd64", "aarch64": "arm64"}.get(machine)
            if arch is None:
                parser.error(f"Unsupported Linux architecture: {machine}")
            artifact = linux_package(binary, stage, dist, arch)
        elif sys.platform == "win32":
            artifact = windows_package(binary, stage, dist, "x64" if machine in ("amd64", "x86_64") else machine)
        elif sys.platform == "darwin":
            artifact = macos_package(binary, stage, dist, "arm64" if machine in ("arm64", "aarch64") else "x64")
        else:
            parser.error(f"Unsupported platform: {sys.platform}")
    checksum = hashlib.sha256(artifact.read_bytes()).hexdigest()
    artifact.with_name(artifact.name + ".sha256").write_text(f"{checksum}  {artifact.name}\n")
    print(artifact)


if __name__ == "__main__":
    main()
