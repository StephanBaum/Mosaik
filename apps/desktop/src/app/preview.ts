/**
 * Whether this is a preview build (ADR-0153): set by the preview workflow through `VITE_MOSAIC_PREVIEW`
 * at build time. A release never sets it; the Rust side has its own `preview` feature.
 */
export const IS_PREVIEW: boolean = import.meta.env["VITE_MOSAIC_PREVIEW"] === "1";
