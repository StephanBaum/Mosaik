/** German UI strings: unlock. Informal "du". Same keys as English (checked by the type). */
import type { unlock as en } from "../en/unlock.ts";

export const unlock = {
  "lock.unlockTitle": "Mosaic entsperren",
  "lock.masterPassword": "Master-Passwort",
  "lock.wrongPassword":
    "Dieses Master-Passwort hat nicht funktioniert. Prüfe die Feststelltaste und dein Tastaturlayout und versuche es erneut.",
  "lock.locked": "Mosaic ist gesperrt.",
  "lock.secretKey": "Secret Key",
  "lock.secretKeyHint":
    "Dieses Gerät kennt deinen Secret Key noch nicht. Gib ihn aus deinem Emergency Kit ein, dann merkt Mosaic ihn sich auf diesem Gerät.",
  "lock.submit": "Entsperren",
  "lock.working": "Wird entsperrt …",
  "lock.reason.manual": "Du hast Mosaic gesperrt.",
  "lock.reason.idle": "Mosaic hat sich nach einer Weile ohne Aktivität selbst gesperrt.",
  "lock.reason.sleep": "Mosaic wurde beim Ruhezustand des Computers gesperrt.",
  "lock.reason.screenLock": "Mosaic wurde zusammen mit deinem Bildschirm gesperrt.",
} satisfies Record<keyof typeof en, string>;
