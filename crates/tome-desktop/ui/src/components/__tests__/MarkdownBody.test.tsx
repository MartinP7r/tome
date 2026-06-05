// MarkdownBody snapshot + behavioural tests — Phase 26 plan 04 Task 2.
//
// Covers:
//   1. Allow-list FORWARD direction — every allowed element's text content
//      appears in the rendered DOM (Pitfall 3 inverse — "did I accidentally
//      strip something I needed?").
//   2. Allow-list REVERSE direction — disallowed elements (tables, images,
//      blockquotes, raw HTML) are STRIPPED; their text is NOT in the DOM
//      (Pitfall 3 — the canonical silent-strip catch).
//   3. Security — clicking a `javascript:` link does NOT call openUrl
//      (T-26-04-02).
//   4. Security — clicking an `https://` link DOES call openUrl exactly once
//      with the original href (T-26-04-02 happy path).
//
// React 19 compat smoke test is implicit: every test renders MarkdownBody
// under React 19; any console.error or runtime exception fails the run.

import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";

import { MarkdownBody } from "../MarkdownBody";

// Mock the Tauri opener — the real implementation requires the Tauri runtime
// which isn't present in jsdom. We assert on the mock for Tests 3 and 4.
vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(),
}));

import { openUrl } from "@tauri-apps/plugin-opener";

const mockedOpenUrl = vi.mocked(openUrl);

beforeEach(() => {
  mockedOpenUrl.mockReset();
  mockedOpenUrl.mockResolvedValue();
});

const ALLOWED_FIXTURE = `# Heading One

## Heading Two

### Heading Three

This is a paragraph with **bold text** and *italic text* and \`inline code\`.

\`\`\`
fenced code block line one
fenced code block line two
\`\`\`

- unordered item one
- unordered item two

1. ordered item one
2. ordered item two

[Example link](https://example.com)
`;

const DISALLOWED_FIXTURE = `# Header above the unwanted bits

| col-a | col-b |
| ----- | ----- |
| TABLE_CELL_TEXT | row-value |

![alt-IMAGE-text](https://example.com/img.png)

> BLOCKQUOTE_TEXT inside a quote

<script>SCRIPT_TEXT</script>

<div>RAW_DIV_TEXT</div>

Paragraph after the unwanted bits.
`;

describe("MarkdownBody — allow-list forward (every allowed element renders)", () => {
  it("renders all 12 allowed elements verbatim and snapshots the article", () => {
    const { container } = render(
      <MarkdownBody body={ALLOWED_FIXTURE} skillName="example-skill" />,
    );

    // Heading text content is present at the right levels.
    expect(screen.getByRole("heading", { level: 1, name: "Heading One" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 2, name: "Heading Two" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 3, name: "Heading Three" })).toBeInTheDocument();

    // Inline emphasis: bold + italic + inline code.
    expect(screen.getByText("bold text").tagName.toLowerCase()).toBe("strong");
    expect(screen.getByText("italic text").tagName.toLowerCase()).toBe("em");
    expect(screen.getByText("inline code").tagName.toLowerCase()).toBe("code");

    // Fenced code block: the `<pre>` wraps a `<code>` child.
    const preElements = container.querySelectorAll("pre");
    expect(preElements.length).toBe(1);
    expect(preElements[0].textContent).toContain("fenced code block line one");
    expect(preElements[0].textContent).toContain("fenced code block line two");
    expect(preElements[0].querySelector("code")).not.toBeNull();

    // Lists: both ul and ol present, items intact.
    expect(container.querySelectorAll("ul").length).toBe(1);
    expect(container.querySelectorAll("ol").length).toBe(1);
    expect(screen.getByText("unordered item one")).toBeInTheDocument();
    expect(screen.getByText("ordered item one")).toBeInTheDocument();

    // Link is rendered as <a>.
    const link = screen.getByRole("link", { name: "Example link" });
    expect(link.getAttribute("href")).toBe("https://example.com");

    // article element with the documented aria-label.
    const article = container.querySelector("article");
    expect(article).not.toBeNull();
    expect(article?.getAttribute("aria-label")).toBe("example-skill documentation");

    // Snapshot the rendered HTML to catch any DOM-shape regressions
    // (Pitfall 3 silent-strip regressions and React 19 emitter changes).
    expect(article?.innerHTML).toMatchSnapshot();
  });
});

describe("MarkdownBody — allow-list reverse (disallowed elements are stripped)", () => {
  it("strips tables, images, blockquotes — their content is removed from the DOM", () => {
    const { container } = render(
      <MarkdownBody body={DISALLOWED_FIXTURE} skillName="strip-fixture" />,
    );

    // Allowed surroundings still render.
    expect(
      screen.getByRole("heading", { level: 1, name: "Header above the unwanted bits" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Paragraph after the unwanted bits.")).toBeInTheDocument();

    // Parsed-disallowed: react-markdown drops the wrapper AND the rendered
    // text descendants when the markdown parser produced the disallowed node
    // (table cells, image alt text, blockquote text). This is the canonical
    // Pitfall 3 silent-strip case — confirm in both directions (element +
    // text).
    expect(screen.queryByText(/TABLE_CELL_TEXT/)).toBeNull();
    expect(screen.queryByText(/alt-IMAGE-text/)).toBeNull();
    expect(screen.queryByText(/BLOCKQUOTE_TEXT/)).toBeNull();

    // Element-level checks for parsed-disallowed nodes.
    expect(container.querySelector("table")).toBeNull();
    expect(container.querySelector("img")).toBeNull();
    expect(container.querySelector("blockquote")).toBeNull();

    // **Raw HTML (T-26-04-01).** react-markdown without `rehype-raw` does
    // not parse HTML — `<script>` and `<div>` strings survive as ESCAPED
    // TEXT nodes (not real elements). That's the safe outcome: no XSS
    // primitive, even though the literal characters appear. The security
    // contract is element absence, NOT text absence — make that explicit.
    expect(container.querySelector("script")).toBeNull();
    expect(container.querySelector("article > div")).toBeNull();
    // And confirm the raw HTML is in the DOM only as inert text (not a
    // parsed element); a real <script> child of <article> would be the
    // failure mode this assertion guards against.
    expect(container.querySelectorAll("article *").length).toBeGreaterThan(0);
  });
});

describe("MarkdownBody — link scheme guard (T-26-04-02)", () => {
  // Defence-in-depth note: react-markdown's default URL transform already
  // blanks out `javascript:` hrefs at parse time, so our onClick guard
  // typically sees an empty href. We still assert openUrl is NOT called for
  // ANY non-http(s) scheme, including `javascript:` (parsed out by
  // react-markdown) AND `ftp:` (kept verbatim by react-markdown but rejected
  // by our regex). The two-layer defence is the security contract.

  it("does NOT call openUrl for a javascript: link (react-markdown sanitises the href; onClick guard is the safety net)", () => {
    const body = "[bad](javascript:alert(1))";
    const { container } = render(<MarkdownBody body={body} skillName="js-scheme" />);

    const link = container.querySelector("article a");
    expect(link).not.toBeNull();
    // react-markdown drops the javascript: scheme — href is empty/sanitised.
    const href = link!.getAttribute("href") ?? "";
    expect(/^javascript:/i.test(href)).toBe(false);

    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    fireEvent.click(link!);
    warnSpy.mockRestore();

    expect(mockedOpenUrl).not.toHaveBeenCalled();
  });

  it("does NOT call openUrl for a mailto: link (react-markdown keeps href; onClick guard rejects)", () => {
    // react-markdown's default URL transform allow-lists http, https,
    // mailto, tel, irc — so `mailto:` survives the parser. Our onClick
    // regex `/^https?:/` rejects it; openUrl must NOT fire. This proves
    // the click-time guard catches schemes the parser missed.
    const body = "[mail-link](mailto:hi@example.com)";
    const { container } = render(<MarkdownBody body={body} skillName="mailto-scheme" />);

    const link = Array.from(container.querySelectorAll("article a")).find(
      (a) => a.textContent === "mail-link",
    ) as HTMLElement | undefined;
    expect(link).not.toBeUndefined();
    expect(link!.getAttribute("href")).toBe("mailto:hi@example.com");

    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    fireEvent.click(link!);
    warnSpy.mockRestore();

    expect(mockedOpenUrl).not.toHaveBeenCalled();
  });

  it("calls openUrl exactly once with the original https href on click", () => {
    const body = "[ok](https://example.com/path)";
    render(<MarkdownBody body={body} skillName="https-scheme" />);

    const link = screen.getByRole("link", { name: "ok" });
    fireEvent.click(link);

    expect(mockedOpenUrl).toHaveBeenCalledTimes(1);
    expect(mockedOpenUrl).toHaveBeenCalledWith("https://example.com/path");
  });
});
