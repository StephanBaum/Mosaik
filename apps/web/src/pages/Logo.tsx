/** The Mosaic mark: three tiles and an accent tile (UX_GUIDELINES §2); the accent tile takes the art's lime. */
export function LogoMark() {
  return (
    <svg className="logo-mark" viewBox="0 0 20 20" aria-hidden="true" focusable="false">
      <rect className="logo-mark__tile" x="0" y="0" width="9" height="9" rx="2.5" />
      <rect className="logo-mark__tile" x="11" y="0" width="9" height="9" rx="2.5" />
      <rect className="logo-mark__tile" x="0" y="11" width="9" height="9" rx="2.5" />
      <rect className="logo-mark__tile logo-mark__tile--accent" x="11" y="11" width="9" height="9" rx="2.5" />
    </svg>
  );
}
