/**
 * French UI strings: import. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { importMessages as en } from "../en/import.ts";

export const importMessages = {
  "import.format.enpass": "Enpass (.json)",
  "import.format.bitwarden": "Bitwarden (.json, non chiffré)",
  "import.format.keepass": "KeePass (.kdbx)",
  "import.format.onePassword": "1Password (.1pux)",
  "import.format.cxf": "FIDO Credential Exchange (.json) : clés d’accès et mots de passe",
  "import.format.dashlane": "Dashlane (.zip)",
  "import.format.keeper": "Keeper (.json)",
  "import.format.keeperCsv": "Keeper (.csv)",
  "import.format.protonPass": "Proton Pass (.zip ou .json, non chiffré)",
  "import.format.csv": "Fichier CSV (.csv)",
  "import.keepassPassword": "Mot de passe de la base KeePass",
  "import.preview.ready":
    "{count, plural, one {# élément est prêt à être importé.} other {# éléments sont prêts à être importés.}}",
  "import.preview.attachments": "{count, plural, one {# pièce jointe} other {# pièces jointes}}",
  "import.preview.duplicate":
    "Vous avez déjà un élément semblable. Il est décoché, donc il ne sera pas importé deux fois.",
  "import.preview.duplicateInFile":
    "Ce fichier contient deux fois le même élément. La copie est décochée.",
  "import.warning.unmappedFields":
    "Certains champs n’ont pas d’équivalent dans Mosaic. Ils ont été conservés comme champs personnalisés.",
  "import.warning.attachmentsSkipped":
    "Certaines pièces jointes dépassent 50 Mio, ne tiennent pas dans votre espace de 1 Gio pour les pièces jointes, ou sont absentes de l’export. Ajoutez-les à la main après l’import.",
  "import.warning.entriesSkipped":
    "Certaines entrées n’ont pas pu être lues et seront ignorées. Vérifiez-les dans votre ancien gestionnaire de mots de passe.",
  "import.warning.otpUnreadable":
    "Certains réglages de codes à usage unique n’ont pas été reconnus. Ils ont été conservés dans des champs masqués : configurez de nouveau le code à partir de ceux-ci.",
  "import.warning.otpWeak":
    "Certains codes à usage unique ont un secret de moins de 80 bits, ce qui les rend plus faciles à deviner. Ils ont été importés malgré tout ; configurez-les de nouveau avec un nouveau secret dès que possible.",
  "import.warning.passkeysNotImported":
    "Ce format d’export ne contient pas les clés privées des clés d’accès, elles ne peuvent donc pas être importées. Le reste de ces éléments est importé.",
  "import.warning.passkeyUnreadable":
    "Certaines clés d’accès n’ont pas pu être lues et ont été ignorées.",
  "import.warning.passkeyUnsupported":
    "Certaines clés d’accès utilisent un type de clé que Mosaic ne prend pas en charge (seul ES256 fonctionne) et ont été ignorées.",
  "import.warning.passkeysSkipped":
    "Certaines clés d’accès ont été ignorées parce qu’elles sont trop nombreuses (100 au maximum par élément et 10 000 par fichier).",
  "import.warning.passkeysKeepOldApp":
    "Gardez votre ancienne application jusqu’à ce que vous ayez déplacé les clés d’accès.",
  "import.entry.unmappedFields":
    "Certains champs n’ont pas d’équivalent dans Mosaic. Ils ont été conservés comme champs personnalisés.",
  "import.entry.otpUnreadable":
    "Le réglage du code à usage unique n’a pas été reconnu. Il a été conservé dans un champ masqué : configurez de nouveau le code à partir de celui-ci.",
  "import.entry.otpWeak":
    "Le secret du code à usage unique fait moins de 80 bits, ce qui rend le code plus facile à deviner. Il a été importé malgré tout ; configurez-le de nouveau avec un nouveau secret dès que possible.",
  "import.entry.attachmentTooLarge":
    "Une pièce jointe dépasse 50 Mio et a été ignorée. Ajoutez-la à la main après l’import.",
  "import.entry.attachmentMissing":
    "Une pièce jointe est absente de l’export. Ajoutez-la à la main après l’import.",
  "import.entry.attachmentQuota":
    "Une pièce jointe ne tient pas dans votre espace de 1 Gio pour les pièces jointes et a été ignorée.",
  "import.entry.passkeysNotImported":
    "Ce format d’export ne contient pas la clé privée de la clé d’accès de cet élément, elle ne peut donc pas être importée. Gardez votre ancienne application jusqu’à ce que vous l’ayez déplacée.",
  "import.entry.passkeyUnreadable":
    "Une clé d’accès de cet élément n’a pas pu être lue et a été ignorée. Gardez votre ancienne application jusqu’à ce que vous l’ayez déplacée.",
  "import.entry.passkeyUnsupported":
    "Une clé d’accès de cet élément utilise un type de clé que Mosaic ne prend pas en charge (seul ES256 fonctionne) et a été ignorée. Gardez votre ancienne application jusqu’à ce que vous l’ayez déplacée.",
  "import.entry.historyTruncated":
    "Seuls les 32 anciens mots de passe les plus récents ont été conservés.",
  "import.entry.tagsDropped": "Les étiquettes vides ou trop longues ont été ignorées.",
  "import.entry.archived":
    "Archivé dans votre ancienne application. Il reçoit l’étiquette « archived ».",
  "import.entry.unknownCategory":
    "Type d’élément inconnu. Il est importé comme note sécurisée.",
  "import.entry.titleCleaned":
    "Le titre contenait des caractères invisibles ou qui inversent le texte. Ils ont été supprimés.",
  "import.entry.passkeysLimit":
    "Cet élément contient plus de clés d’accès qu’un import n’en accepte (100 par élément, 10 000 par fichier). Les clés d’accès en trop ont été ignorées. Gardez votre ancienne application jusqu’à ce que vous les ayez déplacées.",
  "import.entry.fieldsDropped":
    "Cet élément contient plus de champs que Mosaic n’en conserve. Les champs en trop sont ignorés.",
  "import.entry.notesTruncated":
    "Les notes de cet élément sont plus longues que ce qu’un élément peut contenir. La partie qui ne tenait pas a été ignorée.",
  "import.entry.idnHost":
    "Cette adresse utilise des caractères spéciaux dans son nom de domaine et peut ressembler à un autre site. Vérifiez-la avant de vous y fier.",
  "import.entry.urlKeptAsText":
    "Cette adresse web n’a pas pu être utilisée comme lien. Elle a été conservée comme texte.",
  "import.skip.malformed": "Cette entrée est endommagée et ne peut pas être lue.",
  "import.skip.empty": "Cette entrée est vide.",
  "import.skip.tooLarge":
    "Cette entrée dépasse 1 Mio, le maximum qu’un élément peut contenir.",
  "import.result":
    "{imported, plural, one {# élément importé.} other {# éléments importés.}} {skipped, plural, =0 {} one {# ignoré.} other {# ignorés.}}",
  "import.flow.title": "Importer",
  "import.flow.formatStep": "D’où importez-vous ?",
  "import.flow.chooseFile": "Choisir le fichier d’export",
  "import.flow.fileChosen": "Fichier : {name}",
  "import.flow.password": "Mot de passe de la base KeePass",
  "import.flow.preview": "Aperçu",
  "import.flow.previewBusy": "Lecture du fichier. Cela peut prendre un moment.",
  "import.flow.selectAll": "Tout sélectionner",
  "import.flow.target": "Importer dans",
  "import.flow.commit":
    "Importer {count, plural, one {# élément} other {# éléments}}",
  "import.flow.entries": "Entrées",
  "import.flow.skipped": "Ignorés",
  "import.flow.attachments": "{count, plural, one {# pièce jointe} other {# pièces jointes}}",
  "import.flow.cancel": "Annuler l’import",
  "import.flow.noneSelected": "Sélectionnez au moins un élément à importer.",
  "import.flow.resultFailed":
    "{failed, plural, =0 {} one {# élément n’a pas pu être enregistré.} other {# éléments n’ont pas pu être enregistrés.}}",
  "import.flow.deleteReminder":
    "Supprimez maintenant le fichier d’export. Il contient vos mots de passe en clair.",
  "import.flow.dropHint":
    "Choisissez votre fichier d’export. Tout est lu et chiffré sur cet appareil.",
} satisfies Record<keyof typeof en, string>;
