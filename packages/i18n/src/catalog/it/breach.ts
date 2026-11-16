/**
 * Italian UI strings: breach. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { breach as en } from "../en/breach.ts";

export const breach = {
  "breach.card.title": "Controllo delle violazioni",
  "breach.card.off":
    "Disattivato. Mosaic non contatta mai Have I Been Pwned, a meno che tu non lo attivi.",
  "breach.card.about":
    "Confronta le tue password con le violazioni di dati note. Per ogni password lascia questo dispositivo solo un frammento di 5 caratteri di un codice offuscato.",
  "breach.mode.label": "Controlla le violazioni di dati note",
  "breach.mode.off": "Disattivato",
  "breach.mode.manual": "Solo quando lo scelgo io",
  "breach.mode.weekly": "Ogni settimana, dopo lo sblocco",
  "breach.card.checkNow": "Controlla ora",
  "breach.card.stop": "Interrompi controllo",
  "breach.card.turnOff": "Disattiva",
  "breach.progress.starting": "Controllo delle tue password…",
  "breach.progress.step": "Controllo… {done} di {total}",
  "breach.result.never": "Non ancora controllato.",
  "breach.result.last":
    "Ultimo controllo il {date, date, medium}. {found, plural, =0 {Nessuna password è stata trovata in una violazione.} one {# password è stata trovata in una violazione.} other {# password sono state trovate in una violazione.}}",
  "breach.result.finished":
    "Controllate {checked, plural, one {# password} other {# password}}. {found, plural, =0 {Nessuna è stata trovata in una violazione.} one {# è stata trovata in una violazione.} other {# sono state trovate in una violazione.}}",
  "breach.result.upToDate": "Tutto è stato controllato di recente. Niente di nuovo da cercare.",
  "breach.error.offline":
    "Impossibile raggiungere Have I Been Pwned. Vengono mostrati gli ultimi risultati. Riprova quando sei online.",
  "breach.error.rateLimited":
    "Have I Been Pwned ha chiesto a Mosaic di rallentare. Riprova tra qualche minuto.",
  "breach.error.protocol":
    "Have I Been Pwned ha inviato una risposta di cui Mosaic non si fida, quindi alcune password non sono state controllate. Riprova più tardi.",
  "breach.error.cancelled":
    "Il controllo è stato interrotto. I risultati già completati vengono conservati.",
  "breach.consent.title": "Controllare le tue password rispetto alle violazioni di dati note?",
  "breach.consent.body":
    "Mosaic trasforma ogni password in un codice offuscato che non si può ricostruire (un hash) e invia a Have I Been Pwned (api.pwnedpasswords.com) solo i primi 5 caratteri di quel codice. Il servizio conosce il tuo indirizzo IP e quei frammenti di 5 caratteri. Non vede mai una password, il tuo indirizzo email né a quale sito appartiene una password. I risultati restano su questo dispositivo.",
  "breach.consent.detail":
    "Molte altre password condividono ogni frammento di 5 caratteri, quindi la risposta non rivela la tua. Resta comunque una richiesta a un altro servizio, perciò rimane disattivata finché non accetti. Puoi disattivarla in qualsiasi momento.",
  "breach.consent.agree": "Attiva e controlla ora",
  "breach.consent.decline": "Non ora",
} satisfies Record<keyof typeof en, string>;
