// SearchField — wraps React Aria SearchField (UI-SPEC §Atoms §SearchField).
//
// Placeholder is "Search skills" verbatim (UI-SPEC §Copywriting). React Aria
// provides the X-clear button for free when the field has a value; we render
// our own magnifier glyph (inline SVG, SF Symbols-shape — UI-SPEC §"Icon set").
// ⌘F focus is wired at the SkillsView level via a ref → input.focus().

import { forwardRef, useImperativeHandle, useRef } from "react";
import {
  SearchField as AriaSearchField,
  Input,
  Button,
} from "react-aria-components";
import styles from "./SearchField.module.css";

export interface SearchFieldProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  ariaLabel?: string;
}

export interface SearchFieldHandle {
  focus: () => void;
}

export const SearchField = forwardRef<SearchFieldHandle, SearchFieldProps>(
  function SearchField(
    {
      value,
      onChange,
      placeholder = "Search skills",
      ariaLabel = "Search skills",
    },
    ref,
  ) {
    const inputRef = useRef<HTMLInputElement>(null);
    useImperativeHandle(
      ref,
      () => ({
        focus: () => inputRef.current?.focus(),
      }),
      [],
    );

    return (
      <AriaSearchField
        aria-label={ariaLabel}
        value={value}
        onChange={onChange}
        className={styles.field}
      >
        <MagnifierIcon />
        <Input ref={inputRef} className={styles.input} placeholder={placeholder} />
        {value.length > 0 && (
          <Button slot="clear" className={styles.clear} aria-label="Clear search">
            ×
          </Button>
        )}
      </AriaSearchField>
    );
  },
);

function MagnifierIcon() {
  return (
    <svg
      className={styles.icon}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      aria-hidden="true"
    >
      <circle cx="7" cy="7" r="5" />
      <line x1="11" y1="11" x2="14" y2="14" strokeLinecap="round" />
    </svg>
  );
}
