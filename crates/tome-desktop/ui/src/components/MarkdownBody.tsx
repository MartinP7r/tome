// MarkdownBody — UI-SPEC §Component Contract §MarkdownBody (VIEW-04, D-08).
//
// Renders the SKILL.md body (post-frontmatter) returned by
// `commands.getSkillDetail` as a scrollable article below the `DetailHeader`.
//
// **Allow-list (12 elements):** h1, h2, h3, p, strong, em, code, ul, ol, li, a,
// pre. Tables, images, blockquotes, task lists, raw HTML, footnotes are
// STRIPPED by `react-markdown`'s `allowedElements`. This is the SC#4 markdown
// subset — wider than `browse/markdown.rs` (TUI) but deliberately narrower
// than CommonMark + GFM. Pitfall 3 in 26-RESEARCH calls out the silent-strip
// behaviour; the Task-2 snapshot test verifies both directions (allowed
// elements render; disallowed are stripped).
//
// **Security (T-26-04-01, T-26-04-02):** No `rehype-raw` — never add it. The
// link `onClick` handler accepts only `http(s)://` schemes; `javascript:` and
// `data:` URLs are rejected silently with a `console.warn`. External links
// open in the system browser via `@tauri-apps/plugin-opener`'s `openUrl`.

import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { openUrl } from "@tauri-apps/plugin-opener";

import styles from "./MarkdownBody.module.css";

// Verbatim per UI-SPEC §MarkdownBody allow-list. Do NOT extend without a
// matching UI-SPEC update — wider subsets need design + a11y review.
const ALLOWED = [
  "h1",
  "h2",
  "h3",
  "p",
  "strong",
  "em",
  "code",
  "ul",
  "ol",
  "li",
  "a",
  "pre",
] as const;

export interface MarkdownBodyProps {
  body: string;
  skillName: string;
}

export function MarkdownBody({ body, skillName }: MarkdownBodyProps) {
  return (
    <article
      aria-label={`${skillName} documentation`}
      className={styles.body}
    >
      <ReactMarkdown
        allowedElements={ALLOWED as unknown as string[]}
        remarkPlugins={[remarkGfm]}
        components={{
          a: ({ href, children }) => (
            <a
              href={href}
              onClick={async (event) => {
                event.preventDefault();
                if (href && /^https?:/.test(href)) {
                  try {
                    await openUrl(href);
                  } catch (err) {
                    console.warn("openUrl failed", err);
                  }
                } else {
                  console.warn(
                    "blocked non-http(s) URL scheme:",
                    href ?? "(no href)",
                  );
                }
              }}
            >
              {children}
            </a>
          ),
        }}
      >
        {body}
      </ReactMarkdown>
    </article>
  );
}
