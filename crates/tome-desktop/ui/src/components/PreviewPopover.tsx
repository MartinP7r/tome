// PreviewPopover — Doctor per-item fix preview (D-09 / NF-04).
//
// React Aria `<DialogTrigger>` + `<Button>` + `<Popover>` + `<Dialog>` —
// preview-then-confirm sheet rendered when the user clicks Fix on a
// `FindingRow`. Three sections inside the popover (UI-SPEC §PreviewPopover):
//   1. PREVIEW caption header (small caps, label-secondary).
//   2. Body sentence — the `dry_run_description` from the DoctorFinding
//      (matches `repair_kind_action_label` on the Rust side).
//   3. Reversibility helper text.
//   4. Cancel / Apply button row.
//
// Apply triggers `onApply()`, which dispatches `commands.doctorRepairOne`
// via the parent's `useDoctorReport` refetch chain. Failures are caught and
// passed to `onError`, which the parent FindingRow renders inline as a
// `[Code] message` disclosure (D-11 / SAFE-01).
//
// Width: 320px max per UI-SPEC. Width is the only metric the popover owns
// — the React Aria `<Popover>` handles positioning, scroll containment, and
// the close-on-outside-click contract. Escape dismisses; Cancel returns focus
// to the Fix button (React Aria default).

import {
  Button as AriaButton,
  Dialog,
  DialogTrigger,
  Heading,
  Popover,
} from "react-aria-components";
import type { TomeError } from "../bindings";
import styles from "./PreviewPopover.module.css";

export interface PreviewPopoverProps {
  /** Verbatim sentence from `repair_kind_action_label` — e.g. "will delete
   *  the real directory and replace it with a symlink into the library". */
  dryRunDescription: string;
  /** Action called when the user confirms (Apply). The promise resolves on
   *  successful repair. Rejections are forwarded to `onError`. */
  onApply: () => Promise<void>;
  /** Invoked when `onApply` rejects (D-11 inline failed-fix surface). */
  onError: (err: TomeError) => void;
}

export function PreviewPopover({
  dryRunDescription,
  onApply,
  onError,
}: PreviewPopoverProps) {
  return (
    <DialogTrigger>
      <AriaButton className={styles.fix} aria-label="Fix">
        Fix
      </AriaButton>
      <Popover className={styles.popover}>
        <Dialog className={styles.dialog} aria-labelledby="preview-heading">
          {({ close }) => (
            <>
              <Heading id="preview-heading" slot="title" className={styles.heading}>
                PREVIEW
              </Heading>
              {/* Body — the verbatim dry_run_description. Path-like fragments
                  could be wrapped in <code>, but the labels never contain
                  paths in alpha (the four kinds emit kind-level strings, not
                  per-item paths), so plain text suffices. */}
              <p className={styles.body}>{dryRunDescription}</p>
              <p className={styles.helper}>
                This change is reversible by running tome sync.
              </p>
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
                    // any error via the parent row's local-error state.
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
