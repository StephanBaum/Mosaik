/** How long deleted items stay in the trash. Matches the `vault.trashEmpty` and `trash.retention` copy. */
export const TRASH_RETENTION_DAYS = 30;
/** From this many days left the row is marked as urgent. */
export const TRASH_URGENT_DAYS = 7;

const DAY_MS = 86_400_000;

/** Whole days an item deleted at `trashedAt` (Unix ms) stays in the trash, between zero and the full retention. */
export function daysLeft(trashedAt: number, now: number): number {
  // A clock read slightly before the deletion must not give more than the full retention.
  const elapsedDays = Math.max(0, Math.floor((now - trashedAt) / DAY_MS));
  return Math.max(0, TRASH_RETENTION_DAYS - elapsedDays);
}
