// TriageDetail tests — Phase 27 plan 27-02 / SYNC-02.
//
// Pins:
// 1. Placeholder renders when entry is null.
// 2. Added / Changed entries render the RadioGroup with Keep + Disable.
// 3. Removed entries omit the RadioGroup and render the verbatim
//    removed-helper copy.
// 4. Managed + git-sourced entries (git_commit_sha_new !== null) add a
//    third radio "View source" — selecting it fires onViewSource and
//    the decision reverts to the previously-selected value.
// 5. Section aria-label = "${name} change details".

import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { TriageDetail } from "../TriageDetail";
import type { TriageEntry } from "../../bindings";

function addedLocal(name: string): TriageEntry {
  return {
    name,
    change_kind: "added",
    source_name: "plugins",
    previous_source: null,
    origin: { kind: "local" },
    content_hash_old: null,
    content_hash_new: "b".repeat(64),
    registry_id: null,
    version_old: null,
    version_new: null,
    git_commit_sha_old: null,
    git_commit_sha_new: null,
    synced_at: null,
  };
}

function addedManagedGit(name: string): TriageEntry {
  return {
    name,
    change_kind: "added",
    source_name: "plugins",
    previous_source: null,
    origin: {
      kind: "managed",
      provenance: {
        registry_id: "axiom@npm",
        version: "1.0.0",
        git_commit_sha: "abc1234",
      },
    },
    content_hash_old: null,
    content_hash_new: "b".repeat(64),
    registry_id: "axiom@npm",
    version_old: null,
    version_new: "1.0.0",
    git_commit_sha_old: null,
    git_commit_sha_new: "abc1234",
    synced_at: null,
  };
}

function removed(name: string): TriageEntry {
  return {
    name,
    change_kind: "removed",
    source_name: "plugins",
    previous_source: null,
    origin: { kind: "local" },
    content_hash_old: "a".repeat(64),
    content_hash_new: null,
    registry_id: null,
    version_old: null,
    version_new: null,
    git_commit_sha_old: null,
    git_commit_sha_new: null,
    synced_at: "2026-06-01T00:00:00Z",
  };
}

const NOOP = () => undefined;

describe("TriageDetail — placeholder + selection", () => {
  it("renders 'Select a change to view details' when entry === null", () => {
    render(
      <TriageDetail
        entry={null}
        decision="keep"
        onDecisionChange={NOOP}
        onViewSource={NOOP}
      />,
    );
    expect(screen.getByText("Select a change to view details")).toBeInstanceOf(
      HTMLElement,
    );
  });

  it("section aria-label = '${name} change details' when entry is present", () => {
    const { container } = render(
      <TriageDetail
        entry={addedLocal("axiom-build")}
        decision="keep"
        onDecisionChange={NOOP}
        onViewSource={NOOP}
      />,
    );
    const section = container.querySelector('[aria-label$="change details"]');
    expect(section).not.toBeNull();
    expect(section?.getAttribute("aria-label")).toBe(
      "axiom-build change details",
    );
  });
});

describe("TriageDetail — Added / Changed RadioGroup", () => {
  it("renders 2 radios (Keep + Disable) for a local Added entry", () => {
    render(
      <TriageDetail
        entry={addedLocal("axiom-build")}
        decision="keep"
        onDecisionChange={NOOP}
        onViewSource={NOOP}
      />,
    );
    expect(screen.getByText("Keep this skill")).toBeInstanceOf(HTMLElement);
    expect(screen.getByText("Disable on this machine")).toBeInstanceOf(
      HTMLElement,
    );
    // No "View source" radio for local skills.
    expect(screen.queryByText("View source (open in Finder)")).toBeNull();
  });

  it("renders 3 radios (incl. 'View source') for a managed+git-sourced entry", () => {
    render(
      <TriageDetail
        entry={addedManagedGit("axiom-build")}
        decision="keep"
        onDecisionChange={NOOP}
        onViewSource={NOOP}
      />,
    );
    expect(screen.getByText("View source (open in Finder)")).toBeInstanceOf(
      HTMLElement,
    );
  });

  it("View-source radio selection fires onViewSource AND reverts to the previous legitimate decision", () => {
    const onViewSource = vi.fn();
    const onDecisionChange = vi.fn();
    render(
      <TriageDetail
        entry={addedManagedGit("axiom-build")}
        decision="keep"
        onDecisionChange={onDecisionChange}
        onViewSource={onViewSource}
      />,
    );
    // Click the View-source radio's clickable label.
    fireEvent.click(screen.getByText("View source (open in Finder)"));
    expect(onViewSource).toHaveBeenCalledTimes(1);
    // The component bounces back to the previously-selected legitimate
    // decision — onDecisionChange is fired with "keep" (the prior value).
    expect(onDecisionChange).toHaveBeenCalledWith("keep");
  });
});

describe("TriageDetail — Removed entries (D-13)", () => {
  it("omits the RadioGroup and renders the verbatim removed-helper copy", () => {
    render(
      <TriageDetail
        entry={removed("axiom-build")}
        decision="keep"
        onDecisionChange={NOOP}
        onViewSource={NOOP}
      />,
    );
    expect(
      screen.getByText(
        "This skill will be removed from the lockfile. No action required.",
      ),
    ).toBeInstanceOf(HTMLElement);
    // No radios.
    expect(screen.queryByRole("radio")).toBeNull();
  });
});
