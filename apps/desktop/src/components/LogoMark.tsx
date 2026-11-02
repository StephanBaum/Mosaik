/** The Mosaic mark: three black tiles and one neutral accent tile (UX_GUIDELINES §2). */
export function LogoMark({ size = "md" }: { size?: "sm" | "md" }) {
  return (
    <svg className={`logo-mark logo-mark--${size}`} viewBox="0 0 20 20" aria-hidden="true" focusable="false">
      <rect className="logo-mark__tile" x="0" y="0" width="9" height="9" rx="2.5" />
      <rect className="logo-mark__tile" x="11" y="0" width="9" height="9" rx="2.5" />
      <rect className="logo-mark__tile" x="0" y="11" width="9" height="9" rx="2.5" />
      <rect className="logo-mark__tile logo-mark__tile--accent" x="11" y="11" width="9" height="9" rx="2.5" />
    </svg>
  );
}
