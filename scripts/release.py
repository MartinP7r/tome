#!/usr/bin/env python3
"""Guarded CLI release entry point. Preparation and publication are resumable."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import tempfile
import time


VERSION_RE = re.compile(r"v?(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)\Z")
TARGETS = ("aarch64-apple-darwin", "x86_64-apple-darwin", "x86_64-unknown-linux-gnu")


def run(*args, cwd=None, capture=True):
    result = subprocess.run(args, cwd=cwd, check=True, text=True,
                            stdout=subprocess.PIPE if capture else None)
    return result.stdout.strip() if capture else ""


def normalize_version(value):
    if not VERSION_RE.fullmatch(value):
        raise ValueError("VERSION must be a stable numeric version, e.g. 0.17.1")
    return value.removeprefix("v") if hasattr(value, "removeprefix") else value.lstrip("v")


def metadata_prepared(manifest, changelog, version):
    package = re.search(r"(?ms)^\[workspace\.package\]\n(.*?)(?=^\[|\Z)", manifest)
    if not package:
        raise ValueError("Missing workspace.package")
    current = re.search(r'^version = "([^"]+)"$', package[1], re.M)
    if not current:
        raise ValueError("Missing workspace version")
    headings = re.findall(r"^## \[([^\]]+)\](?: - \d{4}-\d{2}-\d{2})?\s*$", changelog, re.M)
    if headings.count(version) > 1:
        raise ValueError("Duplicate release heading")
    prepared = current[1] == version and headings.count(version) == 1
    if not prepared and (current[1] == version or version in headings):
        raise ValueError("Version and changelog disagree; repair metadata before release")
    if prepared and [h for h in headings if h != "Unreleased"][0] != version:
        raise ValueError("Requested version is not the latest changelog release")
    return prepared


def stamp_changelog(text, version, date):
    if len(re.findall(r"^## \[Unreleased\]\s*$", text, re.M)) != 1:
        raise ValueError("Expected exactly one Unreleased section")
    text = re.sub(r"^## \[Unreleased\]\s*$",
                  "## [Unreleased]\n\n## [{}] - {}\n".format(version, date),
                  text, count=1, flags=re.M)
    match = re.search(r"^\[Unreleased\]: (.+)/compare/(v[^.]+(?:\.[^.]+){2})\.\.\.HEAD$", text, re.M)
    if not match:
        raise ValueError("Missing Unreleased compare link")
    replacement = "[Unreleased]: {0}/compare/v{1}...HEAD\n[{1}]: {0}/compare/{2}...v{1}".format(
        match[1], version, match[2])
    return text[:match.start()] + replacement + text[match.end():]


def require_clean(root):
    if run("git", "status", "--porcelain", cwd=root):
        raise ValueError("Release requires a clean worktree (including untracked files)")


def prepare(root, version, repo, url):
    """Create a preparation PR without automatically authorizing its merge."""
    branch = "chore/release-v" + version
    work = Path(tempfile.mkdtemp(prefix="tome-release-prepare-")) / "checkout"
    run("git", "worktree", "add", "-b", branch, str(work), "HEAD", cwd=root)
    run("git", "commit", "-S", "--allow-empty", "-m", "chore: start release v" + version, cwd=work)
    run("git", "verify-commit", "HEAD", cwd=work)
    run("git", "push", url, "HEAD:refs/heads/" + branch, cwd=work)
    pr = run("gh", "pr", "create", "--repo", repo, "--base", "main", "--head", branch,
             "--draft", "--title", "chore: prepare CLI v" + version,
             "--body", "CLI-only release preparation. Signed bootstrap; validation follows. Merge approval required.", cwd=work)
    manifest = work / "Cargo.toml"
    original = manifest.read_text()
    updated, count = re.subn(r'(?m)^version = "[^"]+"$', 'version = "' + version + '"', original, count=1)
    if count != 1:
        raise ValueError("Workspace version not found")
    metadata = json.loads(run("cargo", "metadata", "--locked", "--no-deps", "--format-version", "1", cwd=work))
    members = set(metadata["workspace_members"])
    packages = [p for p in metadata["packages"] if p["id"] in members]
    old_match = re.search(r'(?m)^version = "([^"]+)"$', original)
    if not old_match:
        raise ValueError("Missing original workspace version")
    old = old_match[1]
    lock = (work / "Cargo.lock").read_text()
    for package in packages:
        crate_manifest = Path(package["manifest_path"]).read_text()
        if re.search(r"(?m)^version\.workspace\s*=\s*true", crate_manifest):
            pattern = r'(\[\[package\]\]\nname = "' + re.escape(package["name"]) + r'"\nversion = ")' + re.escape(old) + '"'
            lock, count = re.subn(pattern, lambda m: m[1] + version + '"', lock)
            if count != 1:
                raise ValueError("Expected one lock entry for " + package["name"])
    manifest.write_text(updated)
    (work / "Cargo.lock").write_text(lock)
    changelog = work / "CHANGELOG.md"
    changelog.write_text(stamp_changelog(changelog.read_text(), version, datetime.datetime.utcnow().date().isoformat()))
    run("cargo", "check", "--locked", "-p", "tome", cwd=work, capture=False)
    run("git", "add", "Cargo.toml", "Cargo.lock", "CHANGELOG.md", cwd=work)
    run("git", "commit", "-S", "-m", "chore: prepare CLI v" + version, cwd=work)
    run("git", "verify-commit", "HEAD", cwd=work)
    run("git", "push", url, "HEAD:refs/heads/" + branch, cwd=work)
    print("Preparation pushed: {}. Review/merge it, then rerun make release at merged main.".format(pr))


def ensure_tag(root, tag, sha, url):
    remote = run("git", "ls-remote", "--tags", url, "refs/tags/" + tag, cwd=root)
    local = subprocess.run(["git", "show-ref", "--verify", "--quiet", "refs/tags/" + tag], cwd=root).returncode == 0
    if remote and not local:
        run("git", "fetch", url, "refs/tags/{0}:refs/tags/{0}".format(tag), cwd=root)
        local = True
    if local:
        run("git", "verify-tag", tag, cwd=root)
        if run("git", "rev-parse", tag + "^{commit}", cwd=root) != sha:
            raise ValueError("Existing tag points to a different commit; refusing replacement")
        if remote and remote.split()[0] != run("git", "rev-parse", "refs/tags/" + tag, cwd=root):
            raise ValueError("Local and remote tag objects differ; refusing replacement")
    else:
        run("git", "tag", "-s", "-m", "Tome CLI " + tag, tag, sha, cwd=root)
        run("git", "verify-tag", tag, cwd=root)
    if not remote:
        run("git", "push", url, "refs/tags/" + tag, cwd=root)
    confirmed = run("git", "ls-remote", "--tags", url, "refs/tags/" + tag, cwd=root)
    if confirmed.split()[0] != run("git", "rev-parse", "refs/tags/" + tag, cwd=root):
        raise ValueError("Remote tag readback mismatch")


def verify_publication(root, tag, sha, repo):
    workflow = None
    for _ in range(30):
        runs = json.loads(run("gh", "run", "list", "--repo", repo, "--workflow", "release.yml",
                              "--branch", tag, "--json", "databaseId,headSha", cwd=root))
        workflow = next((r for r in runs if r["headSha"] == sha), None)
        if workflow:
            break
        time.sleep(2)
    if not workflow:
        raise ValueError("Tag pushed, but release workflow not found; inspect GitHub before retrying")
    run("gh", "run", "watch", str(workflow["databaseId"]), "--repo", repo, "--exit-status", "--interval", "10", cwd=root, capture=False)
    release = json.loads(run("gh", "release", "view", tag, "--repo", repo,
                             "--json", "tagName,isDraft,isPrerelease,assets,url", cwd=root))
    required = {"tome-" + t + ".tar.xz" for t in TARGETS}
    required |= {name + ".sha256" for name in required}
    required |= {"tome.rb", "sha256.sum", "source.tar.gz", "source.tar.gz.sha256"}
    if release["tagName"] != tag or release["isDraft"] or release["isPrerelease"] or not required.issubset({a["name"] for a in release["assets"]}):
        raise ValueError("Stable release or required assets missing")
    dest = Path(tempfile.mkdtemp(prefix="tome-published-"))
    run("gh", "release", "download", tag, "--repo", repo, "--dir", str(dest), cwd=root)
    for filename in sorted(required):
        if filename.endswith(".sha256"):
            expected = (dest / filename).read_text().split()[0]
            actual = hashlib.sha256((dest / filename.removesuffix(".sha256")).read_bytes()).hexdigest()
            if actual != expected:
                raise ValueError("Checksum mismatch: " + filename)
    target = {("Darwin", "arm64"): "aarch64-apple-darwin", ("Darwin", "x86_64"): "x86_64-apple-darwin", ("Linux", "x86_64"): "x86_64-unknown-linux-gnu"}.get((platform.system(), platform.machine()))
    if not target:
        raise ValueError("No smoke-test archive for this host")
    # Extract only the executable from a known release archive, never arbitrary paths.
    import tarfile
    with tarfile.open(dest / ("tome-" + target + ".tar.xz")) as archive:
        candidates = [m for m in archive.getmembers() if m.isfile() and Path(m.name).name == "tome"]
        if len(candidates) != 1:
            raise ValueError("Expected one executable in archive")
        binary = dest / "tome-smoke"
        stream = archive.extractfile(candidates[0])
        if stream is None:
            raise ValueError("Cannot read archive executable")
        binary.write_bytes(stream.read())
        binary.chmod(0o755)
    if run(str(binary), "--version") != "tome " + tag[1:]:
        raise ValueError("Published executable version mismatch")
    run(str(binary), "--help")
    print("Published and verified:", release["url"])
    print("Assets and checksum-verified Homebrew formula:", dest)
    print("Homebrew publication remains a separate signed PR.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True)
    args = parser.parse_args()
    version = normalize_version(args.version)
    root = Path(run("git", "rev-parse", "--show-toplevel"))
    require_clean(root)
    identity = json.loads(run("gh", "repo", "view", "--json", "nameWithOwner,sshUrl", cwd=root))
    repo, url = identity["nameWithOwner"], identity["sshUrl"]
    run("git", "fetch", url, "main", cwd=root)
    sha = run("git", "rev-parse", "FETCH_HEAD", cwd=root)
    if run("git", "rev-parse", "HEAD", cwd=root) != sha:
        raise ValueError("Run from a clean isolated worktree at exact remote main")
    if not metadata_prepared((root / "Cargo.toml").read_text(), (root / "CHANGELOG.md").read_text(), version):
        prepare(root, version, repo, url)
        return
    run("cargo", "check", "--locked", "-p", "tome", cwd=root, capture=False)

    checks = json.loads(run("gh", "run", "list", "--repo", repo, "--workflow", "ci.yml", "--commit", sha,
                            "--json", "databaseId,status,conclusion", cwd=root))
    if not checks:
        raise ValueError("No CI run found for release commit")
    for check in checks:
        if check["status"] != "completed":
            run("gh", "run", "watch", str(check["databaseId"]), "--repo", repo, "--exit-status", cwd=root, capture=False)
        elif check["conclusion"] != "success":
            raise ValueError("Release commit CI did not succeed")
    ensure_tag(root, "v" + version, sha, url)
    verify_publication(root, "v" + version, sha, repo)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, subprocess.CalledProcessError) as error:
        raise SystemExit("Release stopped: " + str(error))
