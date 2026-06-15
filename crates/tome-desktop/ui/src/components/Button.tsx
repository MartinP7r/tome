// Button — UI-SPEC §Atoms §Button.
//
// Three variants:
//   - primary: --accent fill, --accent-on text, --text-body 13px / 600.
//     Used for "Disable on this machine" (the lone Phase-26 mutation, D-06)
//     and "Apply" (popover).
//   - secondary: --bg-window fill, --label-primary text, 1px --separator
//     border, --text-body 13px / 400. Used for "Open source folder", "Copy
//     path", "Cancel".
//   - small-fix: secondary metrics + smaller padding + --text-small 12px /
//     600. Reserved for FindingRow's "Fix" button (plan 26-05).
//
// React Aria <Button> for keyboard semantics + focus management. The visible
// label comes from `children`; `ariaLabel` overrides it for the accessibility
// tree (used by the DetailHeader action triplet which needs explicit
// "${action} for ${skill}" templates per UI-SPEC §VoiceOver labels).

import type { ReactNode } from "react";
import { Button as AriaButton } from "react-aria-components";
import styles from "./Button.module.css";

export type ButtonVariant = "primary" | "secondary" | "small-fix";

export interface ButtonProps {
  variant: ButtonVariant;
  children: ReactNode;
  onPress?: () => void;
  ariaLabel?: string;
  disabled?: boolean;
}

const VARIANT_CLASS: Record<ButtonVariant, string> = {
  primary: styles.primary,
  secondary: styles.secondary,
  "small-fix": styles.smallFix,
};

export function Button({
  variant,
  children,
  onPress,
  ariaLabel,
  disabled = false,
}: ButtonProps) {
  return (
    <AriaButton
      className={`${styles.button} ${VARIANT_CLASS[variant]}`}
      onPress={onPress}
      aria-label={ariaLabel}
      isDisabled={disabled}
    >
      {children}
    </AriaButton>
  );
}
