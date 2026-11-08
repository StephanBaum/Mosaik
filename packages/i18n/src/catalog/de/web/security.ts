/** German strings new on the Security page (website v2 §5.3). */
import type { security as en } from "../../en/web/security.ts";

export const security = {
  "web.security.eyebrow": "Sicherheit",
  "web.security.title": "Vertrau keinem Server. Auch nicht unserem.",
  "web.security.sub":
    "{name} verschließt jeden Eintrag auf deinem Gerät, bevor er sich bewegt, lässt dich wählen, wo er liegt, und prüft, ob der, der ihn aufbewahrt, ehrlich bleibt.",
  "web.security.stolen.eyebrow": "Falls wir gehackt werden",
  "web.security.stolen.title": "Ein gestohlener Server enthält nur Rauschen.",
  "web.security.stolen.add":
    "Jeder Eintrag ist einzeln versiegelt. Ohne dein Master-Passwort und deinen Secret Key ist eine Kopie unserer Datenbank ein Haufen verschlüsselter Dateien.",
  "web.security.see.title": "Was wir sehen, und was nicht.",
  "web.security.see.yes": "Wir sehen",
  "web.security.see.yes1": "Deine E-Mail-Adresse, wenn du die gehostete Synchronisierung nutzt.",
  "web.security.see.yes2": "Deine Geräte, damit du eins abmelden kannst.",
  "web.security.see.yes3": "Wie viele verschlüsselte Daten du speicherst.",
  "web.security.see.no": "Wir sehen nie",
  "web.security.see.no1": "Deine Passwörter, Notizen, Dateien oder Codes.",
  "web.security.see.no2": "Für welche Websites du Logins hast.",
  "web.security.see.no3": "Dein Master-Passwort oder deinen Secret Key.",
  "web.security.see.footer":
    "Dein Cloud-Anbieter sieht bei der Ordner-Synchronisierung verschlüsselte Dateien: Namen, Größen und Zeiten, nichts vom Inhalt.",
  "web.security.cta.secondary": "Zum Vergleich",
} satisfies Record<keyof typeof en, string>;
