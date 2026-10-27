"""Release safety tests: real local Git remotes and ephemeral SSH test keys."""
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("release", Path(__file__).parents[1] / "release.py")
assert spec is not None and spec.loader is not None
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


class MetadataTests(unittest.TestCase):
    manifest = '[workspace.package]\nversion = "0.17.1"\n[workspace.dependencies]\n'
    notes = '## [Unreleased]\n\n## [0.17.1] - 2026-10-26\n\n## [0.17.0] - 2026-10-13\n'

    def test_prepared_metadata_is_unchanged(self):
        self.assertTrue(release.metadata_prepared(self.manifest, self.notes, "0.17.1"))
        root = Path(__file__).parents[2]
        manifest = (root / "Cargo.toml").read_text()
        version = release.re.search(r'(?m)^version = "([^"]+)"$', manifest)[1]
        self.assertTrue(release.metadata_prepared(manifest,
                                                 (root / "CHANGELOG.md").read_text(), version))

    def test_duplicate_heading_rejected(self):
        with self.assertRaises(ValueError):
            release.metadata_prepared(self.manifest, self.notes + '## [0.17.1] - 2026-10-27\n', "0.17.1")

    def test_mismatched_version_rejected(self):
        with self.assertRaises(ValueError):
            release.metadata_prepared(self.manifest, '## [Unreleased]\n', "0.17.1")

    def test_stamp_preserves_unreleased_and_links(self):
        original = ('## [Unreleased]\n\n### Fixed\n- A fix\n\n## [0.17.0] - 2026-10-13\n'
                    '[Unreleased]: https://github.com/example/repo/compare/v0.17.0...HEAD\n')
        result = release.stamp_changelog(original, "0.17.1", "2026-10-27")
        self.assertEqual(result.count('## [Unreleased]'), 1)
        self.assertEqual(result.count('## [0.17.1]'), 1)
        self.assertIn('compare/v0.17.1...HEAD', result)
        self.assertIn('[0.17.1]: https://github.com/example/repo/compare/v0.17.0...v0.17.1', result)
        self.assertIn('### Fixed\n- A fix', result)

    def test_stable_version_validation(self):
        self.assertEqual(release.normalize_version("v0.17.1"), "0.17.1")
        for invalid in ("01.2.3", "1.2", "1.2.3-rc1", "1.2.3\n", "1.2.3;false"):
            with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                release.normalize_version(invalid)

    def test_old_release_rejected(self):
        with self.assertRaises(ValueError):
            release.metadata_prepared(self.manifest, self.notes.replace('## [Unreleased]',
                '## [Unreleased]\n\n## [0.18.0] - 2026-10-28'), "0.17.1")


class GitTagTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.repo = self.root / "repo"
        self.remote = str(self.root / "remote.git")
        release.run("git", "init", "--bare", self.remote)
        release.run("git", "init", str(self.repo))
        self.git("config", "user.name", "Release Test")
        self.git("config", "user.email", "release-test@example.invalid")
        self.git("config", "commit.gpgsign", "false")
        self.git("config", "tag.gpgsign", "false")
        self.git("config", "gpg.format", "ssh")
        self.git("config", "gpg.ssh.program", "ssh-keygen")
        key = self.root / "test-key"
        release.run("ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", str(key))
        self.git("config", "user.signingkey", str(key))
        allowed = self.root / "allowed-signers"
        allowed.write_text("release-test@example.invalid " + key.with_suffix(".pub").read_text())
        self.git("config", "gpg.ssh.allowedSignersFile", str(allowed))
        self.git("-c", "core.hooksPath=/dev/null", "commit", "--allow-empty", "-m", "test commit")
        self.sha = self.git("rev-parse", "HEAD")

    def git(self, *args):
        return release.run("git", *args, cwd=self.repo)

    def ensure(self):
        release.ensure_tag(self.repo, "v0.17.1", self.sha, self.remote)

    def test_signed_tag_is_pushed_verified_and_idempotent(self):
        self.ensure()
        original = self.git("rev-parse", "v0.17.1")
        self.ensure()
        self.assertEqual(original, self.git("rev-parse", "v0.17.1"))
        self.assertEqual(self.sha, self.git("rev-parse", "v0.17.1^{commit}"))
        self.git("verify-tag", "v0.17.1")

    def test_lightweight_tag_is_rejected_without_push(self):
        self.git("tag", "v0.17.1")
        with self.assertRaises(subprocess.CalledProcessError):
            self.ensure()
        self.assertEqual(self.git("ls-remote", self.remote, "refs/tags/v0.17.1"), "")

    def test_wrong_commit_is_rejected_without_push(self):
        self.git("tag", "-s", "-m", "wrong target", "v0.17.1")
        self.git("-c", "core.hooksPath=/dev/null", "commit", "--allow-empty", "-m", "next")
        self.sha = self.git("rev-parse", "HEAD")
        with self.assertRaises(ValueError):
            self.ensure()
        self.assertEqual(self.git("ls-remote", self.remote, "refs/tags/v0.17.1"), "")

    def test_conflicting_remote_tag_object_is_not_replaced(self):
        self.ensure()
        original = self.git("ls-remote", self.remote, "refs/tags/v0.17.1")
        self.git("tag", "-d", "v0.17.1")
        self.git("tag", "-s", "-m", "different annotation", "v0.17.1")
        with self.assertRaises(ValueError):
            self.ensure()
        self.assertEqual(original, self.git("ls-remote", self.remote, "refs/tags/v0.17.1"))

    def test_remote_only_signed_tag_can_resume(self):
        self.ensure()
        original = self.git("rev-parse", "v0.17.1")
        self.git("tag", "-d", "v0.17.1")
        self.ensure()
        self.assertEqual(original, self.git("rev-parse", "v0.17.1"))

    def test_untracked_files_block_release(self):
        (self.repo / "untracked").write_text("preserve")
        with self.assertRaises(ValueError):
            release.require_clean(self.repo)


if __name__ == "__main__":
    unittest.main()
