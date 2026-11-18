/**
 * Dutch UI strings: credentialRotation. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { credentialRotation as en } from "../en/credentialRotation.ts";

export const credentialRotation = {
  "credentialRotation.tablist": "Beveiligingsweergaven",
  "credentialRotation.tab.overview": "Overzicht",
  "credentialRotation.tab.rotate": "Nu vernieuwen",
  "credentialRotation.tab.dueHidden": ", {count} te wijzigen",
  "credentialRotation.queue.title": "Nu vernieuwen",
  "credentialRotation.queue.intro":
    "De wachtwoorden die je het eerst moet wijzigen. Mosaic begeleidt je, maar je wijzigt elk wachtwoord zelf op de site.",
  "credentialRotation.queue.empty": "Er is nu niets te wijzigen.",
  "credentialRotation.queue.count":
    "{count, plural, one {# wachtwoord om te wijzigen} other {# wachtwoorden om te wijzigen}}",
  "credentialRotation.queue.list": "Te wijzigen wachtwoorden",
  "credentialRotation.queue.snoozedHeading": "Uitgesteld",
  "credentialRotation.queue.snoozedUntil": "Uitgesteld tot {date, date, medium}",
  "credentialRotation.queue.reason.weak": "Zwak",
  "credentialRotation.queue.reason.reused": "Hergebruikt",
  "credentialRotation.queue.reason.old": "Oud",
  "credentialRotation.queue.reason.breached": "In een datalek",
  "credentialRotation.queue.reason.unfinished": "Wijziging niet afgerond",
  "credentialRotation.queue.change": "Wachtwoord wijzigen",
  "credentialRotation.queue.continue": "Wijziging voortzetten",
  "credentialRotation.queue.editItem": "Item bewerken",
  "credentialRotation.queue.manualHint":
    "Mosaic kan dit niet begeleiden. Open het item en wijzig het zelf.",
  "credentialRotation.queue.actionFor": "{action}: {title}",
  "credentialRotation.queue.snooze": "Uitstellen",
  "credentialRotation.queue.snoozeFor": "{title} uitstellen",
  "credentialRotation.queue.snoozeDays": "{days, plural, one {# dag} other {# dagen}}",
  "credentialRotation.queue.snoozed":
    "Uitgesteld voor {days, plural, one {# dag} other {# dagen}}.",
  "credentialRotation.queue.dismiss": "Negeren",
  "credentialRotation.queue.dismissFor": "{title} negeren",
  "credentialRotation.queue.dismissed": "Genegeerd. Het komt terug zodra het wachtwoord verandert.",
  "credentialRotation.queue.undo": "Ongedaan maken",
  "credentialRotation.panel.title": "Wachtwoord wijzigen voor {title}",
  "credentialRotation.panel.step": "Stap {current} van {total}",
  "credentialRotation.panel.stays":
    "De wijziging voor {title} blijft open. Het nieuwe wachtwoord blijft bij het item bewaard totdat je Het werkte of Het werkte niet kiest.",
  "credentialRotation.panel.cancel": "Annuleren",
  "credentialRotation.steps.new.title": "Kies een nieuw wachtwoord",
  "credentialRotation.steps.new.intro":
    "Mosaic bewaart het nieuwe wachtwoord in dit item voordat je het ergens gebruikt, zodat het niet kwijt kan raken. Het huidige wachtwoord blijft staan totdat je bevestigt.",
  "credentialRotation.steps.new.label": "Nieuw wachtwoord",
  "credentialRotation.steps.new.use": "Dit wachtwoord gebruiken",
  "credentialRotation.steps.new.saving": "Bewaren…",
  "credentialRotation.steps.site.title": "Wijzig het op de site",
  "credentialRotation.steps.site.pageLabel": "Wijzigingspagina",
  "credentialRotation.steps.site.body":
    "Open de wachtwoordpagina van de site in je browser, klik op Mosaic in de werkbalk en kies Wijzigingsformulier invullen. Mosaic verstuurt het formulier nooit.",
  "credentialRotation.steps.site.copyLink": "Link naar de wijzigingspagina kopiëren",
  "credentialRotation.steps.site.linkCopied": "Link gekopieerd.",
  "credentialRotation.steps.site.linkHint":
    "De link is geen geheim. Heeft de site op dat adres geen pagina, zoek dan zelf de wachtwoordinstellingen op.",
  "credentialRotation.steps.site.noLink":
    "Deze login heeft geen https-adres, dus Mosaic heeft geen wijzigingspagina om voor te stellen. Wijzig het wachtwoord zelf op de site.",
  "credentialRotation.steps.site.copyPassword": "Nieuw wachtwoord kopiëren",
  "credentialRotation.steps.site.next": "Ik heb het geprobeerd",
  "credentialRotation.steps.result.title": "Heeft de site het nieuwe wachtwoord geaccepteerd?",
  "credentialRotation.steps.done.failed": "Huidig wachtwoord behouden",
  "credentialRotation.steps.done.notSure": "Wijziging opengelaten",
  "credentialRotation.steps.result.body":
    "Mosaic kan de site niet zien. Vertel wat er gebeurde, zodat het juiste wachtwoord in dit item blijft staan. Er wordt in beide gevallen niets verwijderd.",
  "credentialRotation.steps.result.working": "Bewaren…",
  "credentialRotation.result.worked": "Het werkte",
  "credentialRotation.result.failed": "Het werkte niet",
  "credentialRotation.result.notSure": "Nog niet zeker",
  "credentialRotation.result.workedDone":
    "Klaar. Het nieuwe wachtwoord is nu het wachtwoord van dit item. Het oude staat in de geschiedenis.",
  "credentialRotation.result.failedDone":
    "Het huidige wachtwoord blijft. Het nieuwe staat als niet bevestigd in de geschiedenis, voor het geval de site het toch heeft doorgevoerd.",
  "credentialRotation.result.notSureDone":
    "De wijziging blijft open op dit item. Kom terug als je het weet.",
  "credentialRotation.banner.title": "Wachtwoordwijziging gestart op {date, date, medium}",
  "credentialRotation.banner.prepared":
    "Het nieuwe wachtwoord blijft hier bewaard totdat je bevestigt. Nog geen enkele pagina heeft het ontvangen.",
  "credentialRotation.banner.delivered":
    "Het nieuwe wachtwoord blijft hier bewaard totdat je bevestigt. De browserextensie heeft het in het formulier van de site ingevuld.",
  "credentialRotation.banner.reveal": "Nieuw wachtwoord tonen",
  "credentialRotation.banner.hide": "Nieuw wachtwoord verbergen",
  "credentialRotation.banner.copy": "Nieuw wachtwoord kopiëren",
  "credentialRotation.banner.newPassword": "Nieuw wachtwoord",
  "credentialRotation.banner.copyLink": "Link naar de wijzigingspagina kopiëren",
  "credentialRotation.history.title": "Wachtwoordgeschiedenis",
  "credentialRotation.history.list": "Eerdere wachtwoorden",
  "credentialRotation.history.entry": "Vervangen op {date, date, medium}",
  "credentialRotation.history.unconfirmed": "Niet bevestigd",
  "credentialRotation.history.unconfirmedEntry": "Geprobeerd op {date, date, medium}",
  "credentialRotation.history.unconfirmedHint":
    "De site kan dit wachtwoord hebben doorgevoerd. Zet het terug als inloggen met het huidige wachtwoord mislukt.",
  "credentialRotation.history.reveal": "Wachtwoord van {date, date, medium} tonen",
  "credentialRotation.history.hide": "Wachtwoord van {date, date, medium} verbergen",
  "credentialRotation.history.restore": "Terugzetten",
  "credentialRotation.history.restoreFor": "Het wachtwoord van {date, date, medium} terugzetten",
  "credentialRotation.history.restored": "Teruggezet. Het vorige wachtwoord staat in de geschiedenis.",
  "credentialRotation.policy.label": "Herinner me eraan dit wachtwoord te wijzigen",
  "credentialRotation.policy.none": "Nooit",
  "credentialRotation.policy.every": "Elke {days} dagen",
  "credentialRotation.policy.help":
    "Een herinnering zet het item in de lijst Nu vernieuwen. Mosaic stuurt daarvoor geen melding en doet geen verzoek.",
  "credentialRotation.url.help":
    "Optioneel. Het https-adres van de wachtwoordpagina van de site, op dezelfde site als deze login. Mosaic opent het alleen als je in de browserextensie Wijzigingspagina openen kiest.",
  "credentialRotation.url.invalid":
    "Mosaic gebruikt dit adres niet. Het moet met https:// beginnen en op dezelfde site staan als deze login.",
  "credentialRotation.settings.title": "Wachtwoordherinneringen",
  "credentialRotation.settings.body":
    "Mosaic kan je na het ontgrendelen laten weten hoeveel wachtwoorden je beter kunt wijzigen.",
  "credentialRotation.settings.reminders":
    "Na het ontgrendelen tonen hoeveel wachtwoorden te wijzigen zijn",
  "credentialRotation.settings.remindersHint":
    "Alleen een bericht in de app. Geen melding, geen netwerkverzoek.",
  "credentialRotation.reminder.toast":
    "{count, plural, one {# wachtwoord om te wijzigen} other {# wachtwoorden om te wijzigen}}",
  "credentialRotation.reminder.show": "Tonen",
  "credentialRotation.dialog.beginTitle": "Een wachtwoordwijziging starten?",
  "credentialRotation.dialog.beginBody":
    "{browser} wil het wachtwoord voor {title} op {host} wijzigen. Mosaic bewaart het nieuwe wachtwoord bij deze login en houdt het huidige vast totdat je bevestigt dat de site het heeft geaccepteerd.",
  "credentialRotation.dialog.site": "Website",
  "credentialRotation.dialog.login": "Login",
  "credentialRotation.dialog.password": "Nieuw wachtwoord",
  "credentialRotation.dialog.hidden": "Verborgen",
  "credentialRotation.dialog.show": "Tonen",
  "credentialRotation.dialog.hide": "Verbergen",
  "credentialRotation.dialog.regenerate": "Een ander genereren",
  "credentialRotation.dialog.regenerated": "Er is een nieuw wachtwoord gegenereerd.",
  "credentialRotation.dialog.allow": "Dit wachtwoord gebruiken",
  "credentialRotation.dialog.cancel": "Annuleren",
  "credentialRotation.dialog.decideLater": "Later beslissen",
  "credentialRotation.dialog.untitled": "deze login",
  "credentialRotation.dialog.attempt":
    "Gestart op {at, date, medium}, {at, time, short} · {stage}{n, plural, =0 {} one { · # andere wijziging open} other { · # andere wijzigingen open}}",
  "credentialRotation.dialog.stagePrepared": "nog geen pagina heeft het nieuwe wachtwoord",
  "credentialRotation.dialog.stageDelivered": "het nieuwe wachtwoord is in een pagina ingevuld",
  "credentialRotation.dialog.resultTitle": "Heeft {host} het nieuwe wachtwoord geaccepteerd?",
  "credentialRotation.dialog.resultBody":
    "{browser} meldt dat de wijziging werkte. Als je bevestigt, wordt het nieuwe wachtwoord het wachtwoord voor {title} en gaat het huidige naar de geschiedenis.",
  "credentialRotation.dialog.confirm": "Het werkte",
  "credentialRotation.dialog.keepOld": "Oud wachtwoord behouden",
  "credentialRotation.dialog.failedTitle": "Het oude wachtwoord behouden?",
  "credentialRotation.dialog.failedBody":
    "{browser} meldt dat de wijziging niet werkte. Mosaic behoudt het huidige wachtwoord voor {title}. De nieuwe waarde blijft in de geschiedenis staan, voor het geval de site het toch heeft gewijzigd.",
  "credentialRotation.addOrigin.title": "{host} toevoegen aan {title}?",
  "credentialRotation.addOrigin.body":
    "Mosaic vult deze login dan ook op {host} in. Voeg het alleen toe als {host} echt bij dezelfde site hoort.",
  "credentialRotation.addOrigin.idn":
    "Dit adres bevat speciale tekens. Je browser toont het misschien als {unicode}. Controleer het zorgvuldig.",
  "credentialRotation.addOrigin.add": "Adres toevoegen",
  "credentialRotation.addOrigin.cancel": "Niet nu",
} satisfies Record<keyof typeof en, string>;
