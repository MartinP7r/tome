// PreviewPopover — slot-refactored preview/confirm sheet (Phase 27 plan
// 27-03 / Pitfall 3 atomic refactor).
//
// React Aria `<DialogTrigger>` + `<Button>` + `<Popover>` + `<Dialog>` —
// preview-then-confirm sheet rendered when the user clicks the trigger.
// Originally built for Doctor's per-item Fix flow (Phase 26 D-09 / NF-04);
// generalized in Phase 27 to also host the SYNC-03 machine.toml diff
// (UI-SPEC §Apply flow).
//
// Body shape:
//   1. PREVIEW caption header (small caps, label-secondary).
//   2. Body slot — caller-provided ReactNode (`children`). Doctor passes
//      <p>{dry_run_description}</p>; the Apply flow passes
//      <MachineTomlDiff preview={…} />.
//   3. Reversibility helper text (`helperText`, optional override).
//   4. Cancel / Apply button row.
//
// Apply triggers `onApply()`. Failures are caught and passed to `onError`,
// which the caller renders inline as a `[Code] message` disclosure (D-11
// / SAFE-01). The popover stays open through the awaiting promise so the
// user keeps context, then closes on resolve.
//
// Width: 320px default (Doctor flow) / 480px when the caller passes
// `width={480}` (Apply flow per UI-SPEC §Spacing exceptions). The
// `data-width` attribute on the Popover hooks the wider variant in CSS.
//
// Pitfall 3 atomicity: every caller updated in this plan — Doctor's
// FindingRow caller and the new TriagePanel caller — to use the new
// children-slot API. Build will fail loudly if the old `dryRunDescription`
// prop is left on any call site.

import type { ReactNode } from "react";
import {
  Button as AriaButton,
  Dialog,
  DialogTrigger,
  Heading,
  Popover,
} from "react-aria-components";
import type { TomeError } from "../bindings";
import styles from "./PreviewPopover.module.css";

const DEFAULT_HELPER_TEXT = "This change is reversible by running tome sync.";
const DEFAULT_TRIGGER_LABEL = "Fix";
const DEFAULT_WIDTH = 320;

export interface PreviewPopoverProps {
  /** Body slot — caller-provided ReactNode rendered between the PREVIEW
   *  heading and the helper text. Doctor wraps a single sentence in `<p>`;
   *  the Apply flow renders a `<MachineTomlDiff />`. */
  children: ReactNode;
  /** Action called when the user confirms (Apply). The promise resolves on
   *  success. Rejections are forwarded to `onError` and the popover stays
   *  open so the user can read the inline error + retry. */
  onApply: () => Promise<void>;
  /** Invoked when `onApply` rejects (D-11 inline failed-fix surface). */
  onError: (err: TomeError) => void;
  /** Optional ReactNode trigger slot. Overrides the default
   *  `<Button>{triggerLabel}</Button>` entirely so the Apply flow can
   *  render its own decorated Button (e.g. with a pending-count badge). */
  trigger?: ReactNode;
  /** Label for the default trigger button. Defaults to "Fix" (Doctor flow). */
  triggerLabel?: string;
  /** aria-label for the default trigger button. Defaults to "Fix". */
  triggerAriaLabel?: string;
  /** Popover width in pixels. The component sets `data-width` on the
   *  Popover element so CSS rules can hook a wider variant (the 480px
   *  rule in `PreviewPopover.module.css` matches `data-width="480"`).
   *  Defaults to 320. */
  width?: number;
  /** Helper text shown below the body slot. Defaults to the Doctor copy
   *  ("This change is reversible by running tome sync."); the Apply flow
   *  overrides with the machine.toml-write copy. */
  helperText?: string;
}

export function PreviewPopover({
  children,
  onApply,
  onError,
  trigger,
  triggerLabel = DEFAULT_TRIGGER_LABEL,
  triggerAriaLabel,
  width = DEFAULT_WIDTH,
  helperText = DEFAULT_HELPER_TEXT,
}: PreviewPopoverProps) {
  // Default trigger — used when no `trigger` slot is passed. React Aria's
  // <DialogTrigger> only accepts native interactive children as the
  // trigger, so the slot must wrap an <AriaButton> (or another React Aria
  // primitive).
  // When no explicit triggerAriaLabel is supplied, the accessible name
  // tracks the trigger label so a custom triggerLabel="Apply 3 decisions"
  // is announced verbatim by VoiceOver. The Doctor flow keeps the
  // historical "Fix" both ways via the DEFAULT_* constants.
  const ariaLabel = triggerAriaLabel ?? triggerLabel;
  const triggerNode = trigger ?? (
    <AriaButton className={styles.fix} aria-label={ariaLabel}>
      {triggerLabel}
    </AriaButton>
  );

  return (
    <DialogTrigger>
      {triggerNode}
      <Popover className={styles.popover} data-width={String(width)}>
        <Dialog
          className={styles.dialog}
          aria-labelledby="preview-heading"
          data-width={String(width)}
        >
          {({ close }) => (
            <>
              <Heading id="preview-heading" slot="title" className={styles.heading}>
                PREVIEW
              </Heading>
              {/* Body slot — caller-supplied ReactNode. */}
              <div className={styles.body}>{children}</div>
              <p className={styles.helper}>{helperText}</p>
              <div className={styles.actions}>
                <AriaButton
                  className={styles.cancel}
                  onPress={close}
                  aria-label="Cancel"
                >
                  Cancel
                </AriaButton>
                <AriaButton
                  className={styles.apply}
                  aria-label="Apply"
                  onPress={() => {
                    // Close the popover before awaiting so the focus
                    // transition matches the React Aria default; surface
                    // any error via the caller's local-error state.
                    close();
                    onApply().catch((e: unknown) => {
                      onError(e as TomeError);
                    });
                  }}
                >
                  Apply
                </AriaButton>
              </div>
            </>
          )}
        </Dialog>
      </Popover>
    </DialogTrigger>
  );
}
