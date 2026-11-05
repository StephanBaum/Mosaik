/** Icon geometry (24x24 viewBox, Lucide style), copied from docs/design/screens. */
export type IconElement =
  | { d: string }
  | { circle: readonly [cx: number, cy: number, r: number] }
  | { rect: readonly [x: number, y: number, w: number, h: number, rx: number] }
  /** A small filled dot (used by `more` and `securityKey`). */
  | { dot: readonly [cx: number, cy: number, r: number] };

const shield =
  "M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z";
const corners = [
  "M3 7V5a2 2 0 0 1 2-2h2",
  "M17 3h2a2 2 0 0 1 2 2v2",
  "M21 17v2a2 2 0 0 1-2 2h-2",
  "M7 21H5a2 2 0 0 1-2-2v-2",
];
const note: IconElement[] = [
  { d: "M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" },
  { d: "M14 2v4a2 2 0 0 0 2 2h4" },
  { d: "M16 13H8" },
  { d: "M16 17H8" },
];
const eye: IconElement[] = [
  {
    d: "M2.06 12.35a1 1 0 0 1 0-.7 10.75 10.75 0 0 1 19.88 0 1 1 0 0 1 0 .7 10.75 10.75 0 0 1-19.88 0",
  },
  { circle: [12, 12, 3] },
];

export const ICONS = {
  layers: [{ d: "M12 2 2 7l10 5 10-5-10-5Z" }, { d: "m2 17 10 5 10-5" }, { d: "m2 12 10 5 10-5" }],
  star: [{ d: "M12 2l3.1 6.3 6.9 1-5 4.9 1.2 6.8L12 17.8 5.8 21l1.2-6.8-5-4.9 6.9-1z" }],
  key: [{ circle: [7.5, 15.5, 5.5] }, { d: "m21 2-9.6 9.6" }, { d: "m15.5 7.5 3 3L22 7l-3-3" }],
  passkey: [
    ...corners.map((d) => ({ d })),
    { d: "M8 14s1.5 2 4 2 4-2 4-2" },
    { d: "M9 9h.01" },
    { d: "M15 9h.01" },
  ],
  timer: [{ d: "M10 2h4" }, { d: "m12 14 3-3" }, { circle: [12, 14, 8] }],
  card: [{ rect: [2, 5, 20, 14, 2] }, { d: "M2 10h20" }],
  note,
  file: note,
  fileText: note,
  image: [{ rect: [3, 3, 18, 18, 2] }, { circle: [9, 9, 2] }, { d: "m21 15-3.1-3.1a2 2 0 0 0-2.8 0L6 21" }],
  terminal: [{ d: "m4 17 6-6-6-6" }, { d: "M12 19h8" }],
  shield: [{ d: shield }],
  shieldAlert: [{ d: shield }, { d: "M12 8v4" }, { d: "M12 16h.01" }],
  trash: [
    { d: "M3 6h18" },
    { d: "M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" },
    { d: "M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" },
  ],
  sliders: [{ d: "M20 7h-9" }, { d: "M14 17H5" }, { circle: [17, 17, 3] }, { circle: [7, 7, 3] }],
  lock: [{ rect: [3, 11, 18, 11, 2] }, { d: "M7 11V7a5 5 0 0 1 10 0v4" }],
  lockOpen: [{ rect: [3, 11, 18, 11, 2] }, { d: "M7 11V7a5 5 0 0 1 9.9-1" }],
  search: [{ circle: [11, 11, 8] }, { d: "m21 21-4.3-4.3" }],
  plus: [{ d: "M5 12h14" }, { d: "M12 5v14" }],
  copy: [
    { rect: [8, 8, 14, 14, 2] },
    { d: "M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2" },
  ],
  eye,
  eyeOff: [...eye, { d: "m2 2 20 20" }],
  external: [
    { d: "M15 3h6v6" },
    { d: "M10 14 21 3" },
    { d: "M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" },
  ],
  more: [{ dot: [5, 12, 1.6] }, { dot: [12, 12, 1.6] }, { dot: [19, 12, 1.6] }],
  chevronsUpDown: [{ d: "m7 15 5 5 5-5" }, { d: "m7 9 5-5 5 5" }],
  chevronRight: [{ d: "m9 18 6-6-6-6" }],
  arrowLeft: [{ d: "m12 19-7-7 7-7" }, { d: "M19 12H5" }],
  check: [{ d: "M20 6 9 17l-5-5" }],
  alert: [
    { d: "m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3" },
    { d: "M12 9v4" },
    { d: "M12 17h.01" },
  ],
  info: [{ circle: [12, 12, 10] }, { d: "M12 16v-4" }, { d: "M12 8h.01" }],
  download: [
    { d: "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" },
    { d: "m7 10 5 5 5-5" },
    { d: "M12 15V3" },
  ],
  upload: [
    { d: "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" },
    { d: "m17 8-5-5-5 5" },
    { d: "M12 3v12" },
  ],
  printer: [
    { d: "M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2" },
    { d: "M6 9V3a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v6" },
    { rect: [6, 14, 12, 8, 1] },
  ],
  refresh: [{ d: "M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8" }, { d: "M21 3v5h-5" }],
  clock: [{ circle: [12, 12, 10] }, { d: "M12 6v6l4 2" }],
  smartphone: [{ rect: [5, 2, 14, 20, 2] }, { d: "M12 18h.01" }],
  x: [{ d: "M18 6 6 18" }, { d: "m6 6 12 12" }],
  clipboard: [
    { rect: [8, 2, 8, 4, 1] },
    { d: "M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2" },
  ],
  scan: [...corners.map((d) => ({ d })), { d: "M7 12h10" }],
  restore: [{ d: "M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" }, { d: "M3 3v5h5" }],
  wifi: [
    { d: "M12 20h.01" },
    { d: "M2 8.82a15 15 0 0 1 20 0" },
    { d: "M5 12.859a10 10 0 0 1 14 0" },
    { d: "M8.5 16.429a5 5 0 0 1 7 0" },
  ],
  sidebarExpand: [{ rect: [3, 3, 18, 18, 2] }, { d: "M9 3v18" }, { d: "m14 9 3 3-3 3" }],
  securityKey: [
    {
      d: "M2.586 17.414A2 2 0 0 0 2 18.828V21a1 1 0 0 0 1 1h3a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h1a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h.172a2 2 0 0 0 1.414-.586l.814-.814a6.5 6.5 0 1 0-4-4z",
    },
    { dot: [16.5, 7.5, 0.5] },
  ],
} as const satisfies Record<string, readonly IconElement[]>;

export type IconName = keyof typeof ICONS;
