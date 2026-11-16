/**
 * Italian UI strings: insights. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { insights as en } from "../en/insights.ts";

export const insights = {
  "insights.title": "Sicurezza",
  "insights.intro":
    "I controlli vengono eseguiti su questo dispositivo. Ultimo controllo il {checkedAt, date, medium} alle {checkedAt, time, short} su {count, plural, one {# elemento} other {# elementi}}.",
  "insights.checkAgain": "Controlla di nuovo",
  "insights.summary": "Riepilogo",
  "insights.fixFirst": "Risolvi prima questi",
  "insights.showAll": "Mostra {count, plural, one {l'unica segnalazione} other {tutte le # segnalazioni}}",
  "insights.empty":
    "Nessun problema trovato. Mosaic controlla di nuovo ogni volta che apri questa pagina.",
  "insights.kind.weak": "Debole",
  "insights.kind.weak.description": "Facile da indovinare",
  "insights.kind.reused": "Riutilizzata",
  "insights.kind.reused.description": "{groups, plural, one {# gruppo} other {# gruppi}}",
  "insights.kind.old": "Vecchia (oltre 1 anno)",
  "insights.kind.old.description": "Non cambiata da oltre un anno",
  "insights.kind.missingTotp": "2FA mancante",
  "insights.kind.missingTotp.description": "Il sito la supporta, ma non l'hai configurata",
  "insights.kind.insecureUrl": "HTTP non protetto",
  "insights.kind.insecureUrl.description": "Invia la tua password senza cifratura",
  "insights.kind.breached": "In una violazione",
  "insights.kind.breached.description": "Trovata dal controllo delle violazioni, che è facoltativo",
  "insights.finding.breached":
    "Questa password compare in una violazione di dati nota. Cambiala subito, ovunque l'hai usata.",
  "insights.finding.weak": "Facile da indovinare. Cambiala con una password generata.",
  "insights.finding.reused":
    "Usata anche da {count, plural, one {# altro elemento} other {# altri elementi}}. Dai a ogni sito una password diversa.",
  "insights.finding.old":
    "Non cambiata da oltre un anno. Cambiala se usi ancora questo account.",
  "insights.finding.missingTotp":
    "{site} offre i codici della verifica in due passaggi. Attivali lì e salva il codice in Mosaic.",
  "insights.finding.insecureUrl":
    "Usa HTTP semplice: {url}. Passa a https:// se il sito lo supporta.",
  "insights.severity.high": "Alta",
  "insights.severity.medium": "Media",
  "insights.severity.low": "Bassa",
  "insights.action.changePassword": "Cambia password",
  "insights.action.setUpTotp": "Configura la 2FA",
  "insights.action.reviewUrl": "Controlla",
  "insights.attribution.twoFactor": "Dati 2FA: 2FA Directory (2fa.directory), licenza MIT.",
  "insights.counts": "{count, plural, =0 {Nessun problema} one {# problema} other {# problemi}}",
  "insights.documentation": "Come attivarla: {url}",
  "insights.actionFor": "{action}: {title}",
} satisfies Record<keyof typeof en, string>;
