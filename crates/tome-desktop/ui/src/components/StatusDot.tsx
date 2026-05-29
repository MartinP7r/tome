// StatusDot — atom (UI-SPEC §Atoms — StatusDot).
//
// 8px circle, --success when ok, --danger otherwise. Decorative — the parent
// row carries the textual status ("In sync"). `aria-hidden="true"`.

export interface StatusDotProps {
  ok: boolean;
}

export function StatusDot({ ok }: StatusDotProps) {
  const color = ok ? "#28c840" : "#ff3b30";
  return (
    <span
      aria-hidden="true"
      style={{
        display: "inline-block",
        width: 8,
        height: 8,
        borderRadius: "50%",
        background: color,
      }}
    />
  );
}
