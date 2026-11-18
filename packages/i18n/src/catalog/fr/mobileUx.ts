/**
 * French UI strings: mobileUx. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { mobileUx as en } from "../en/mobileUx.ts";

export const mobileUx = {
  "mobile.sync.warningsBanner":
    "La synchronisation a rencontré un problème. Ouvrez les réglages pour le vérifier.",
  "mobile.provider.errorTitle": "Mosaic ne peut pas continuer",
  "mobile.onboarding.details.keyWiped":
    "Mosaic a supprimé la clé secrète scannée quand vous avez quitté l’application. Scannez de nouveau le code ou saisissez la clé.",
  "mobile.common.showField": "Afficher {field}",
  "mobile.common.hideField": "Masquer {field}",
  "mobile.error.storeInUse":
    "Mosaic est occupé ailleurs sur ce téléphone. Patientez un instant, puis réessayez.",
  "mobile.error.noAccount":
    "Mosaic n’est pas encore configuré sur ce téléphone. Ouvrez Mosaic pour le configurer.",
  "mobile.error.wrongCredentials":
    "Ce mot de passe principal ou cette clé secrète n’a pas fonctionné. Vérifiez les fautes de frappe et la langue du clavier, puis réessayez.",
  "mobile.error.invalidInput":
    "Une de vos saisies n’est pas valide. Vérifiez-les et réessayez.",
  "mobile.error.vaultUnavailable":
    "Ce coffre-fort ne peut pas être ouvert pour le moment. Synchronisez et réessayez, ou déverrouillez de nouveau Mosaic.",
  "mobile.unlock.quickOptionTouchId": "Déverrouiller avec Touch ID la prochaine fois",
  "mobile.detail.readCode": "Lire le code",
  "mobile.detail.labelValue": "{label} : {value}",
  "mobile.autofill.warningWithName": "Vérifiez d’abord l’adresse | {name}",
  "mobile.autofill.linkEntry": "Associer {login} et remplir",
  "mobile.link.imported": "Importé pour cette application",
  "mobile.link.confirmTitle": "Associer {login} à {app} ?",
  "mobile.link.confirmSubtitle": "ID de l’application {appId} · Signature {fingerprint}",
  "mobile.link.confirmFallbackHint":
    "Après l’association, touchez de nouveau le champ pour le remplir.",
  "mobile.link.importedNotVerified":
    "Identifiant importé. Le site web n’a pas confirmé cette application.",
} satisfies Record<keyof typeof en, string>;
