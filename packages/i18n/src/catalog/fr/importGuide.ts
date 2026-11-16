/**
 * French UI strings: importGuide. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { importGuide as en } from "../en/importGuide.ts";

export const importGuide = {
  "import.grid.title": "Où sont vos mots de passe actuellement ?",
  "import.grid.browsers": "Navigateurs",
  "import.grid.managers": "Gestionnaires de mots de passe",
  "import.grid.other": "Autre chose",
  "import.source.chrome": "Chrome",
  "import.source.edge": "Edge",
  "import.source.brave": "Brave",
  "import.source.opera": "Opera",
  "import.source.vivaldi": "Vivaldi",
  "import.source.arc": "Arc",
  "import.source.firefox": "Firefox",
  "import.source.safari": "Safari",
  "import.source.applePasswords": "Mots de passe Apple",
  "import.source.onePassword": "1Password",
  "import.source.bitwarden": "Bitwarden",
  "import.source.keepass": "KeePass",
  "import.source.keepassxcCsv": "KeePassXC (CSV)",
  "import.source.enpass": "Enpass",
  "import.source.lastpass": "LastPass",
  "import.source.dashlane": "Dashlane",
  "import.source.keeper": "Keeper",
  "import.source.nordpass": "NordPass",
  "import.source.protonPass": "Proton Pass",
  "import.source.roboform": "RoboForm",
  "import.source.otherFile": "Autre CSV ou fichier",
  "import.guide.chrome.1":
    "Ouvrez cette adresse dans Chrome : copiez-la et collez-la dans la barre d’adresse.",
  "import.guide.chrome.2":
    "À côté de « Exporter les mots de passe », cliquez sur « Télécharger le fichier » et confirmez avec le mot de passe de votre ordinateur.",
  "import.guide.chrome.3":
    "Chrome enregistre « Chrome Passwords.csv » dans Téléchargements.",
  "import.guide.edge.1":
    "Ouvrez cette adresse dans Edge : copiez-la et collez-la dans la barre d’adresse.",
  "import.guide.edge.2":
    "Cliquez sur « ⋯ » à côté de « Mots de passe enregistrés », puis sur « Exporter les mots de passe », et confirmez.",
  "import.guide.edge.3": "Edge enregistre le fichier CSV dans Téléchargements.",
  "import.guide.brave.1":
    "Ouvrez cette adresse dans Brave : copiez-la et collez-la dans la barre d’adresse.",
  "import.guide.brave.2":
    "À côté de « Exporter les mots de passe », cliquez sur « Télécharger le fichier » et confirmez avec le mot de passe de votre ordinateur.",
  "import.guide.brave.3": "Brave enregistre le fichier CSV dans Téléchargements.",
  "import.guide.opera.1":
    "Ouvrez cette adresse dans Opera : copiez-la et collez-la dans la barre d’adresse.",
  "import.guide.opera.2":
    "À côté de « Exporter les mots de passe », cliquez sur « Télécharger le fichier » et confirmez avec le mot de passe de votre ordinateur.",
  "import.guide.opera.3": "Opera enregistre le fichier CSV dans Téléchargements.",
  "import.guide.vivaldi.1":
    "Ouvrez cette adresse dans Vivaldi : copiez-la et collez-la dans la barre d’adresse.",
  "import.guide.vivaldi.2":
    "À côté de « Exporter les mots de passe », cliquez sur « Télécharger le fichier » et confirmez avec le mot de passe de votre ordinateur.",
  "import.guide.vivaldi.3": "Vivaldi enregistre le fichier CSV dans Téléchargements.",
  "import.guide.arc.1":
    "Ouvrez cette adresse dans Arc : copiez-la et collez-la dans la barre d’adresse.",
  "import.guide.arc.2":
    "À côté de « Exporter les mots de passe », cliquez sur « Télécharger le fichier » et confirmez avec le mot de passe de votre ordinateur.",
  "import.guide.arc.3": "Arc enregistre le fichier CSV dans Téléchargements.",
  "import.guide.firefox.1":
    "Ouvrez cette adresse dans Firefox : copiez-la et collez-la dans la barre d’adresse.",
  "import.guide.firefox.2":
    "Cliquez sur « ⋯ » en haut à droite, puis sur « Exporter les mots de passe… », et confirmez.",
  "import.guide.firefox.3": "Enregistrez « logins.csv » dans Téléchargements.",
  "import.guide.safari.1":
    "Safari → Réglages → Mots de passe (macOS 14 et antérieur ; à partir de macOS 15, utilisez Mots de passe Apple).",
  "import.guide.safari.2":
    "Cliquez sur « ⋯ » sous la liste, puis sur « Exporter tous les mots de passe… ».",
  "import.guide.safari.3": "Enregistrez le fichier CSV dans Téléchargements.",
  "import.guide.applePasswords.1": "Ouvrez l’app Mots de passe.",
  "import.guide.applePasswords.2":
    "Fichier → « Exporter tous les mots de passe dans un fichier… ».",
  "import.guide.applePasswords.3": "Enregistrez le fichier CSV dans Téléchargements.",
  "import.guide.onePassword.1": "Ouvrez l’application 1Password et déverrouillez-la.",
  "import.guide.onePassword.2":
    "Fichier → Exporter → votre compte, puis saisissez votre mot de passe.",
  "import.guide.onePassword.3": "Choisissez « 1PUX » et cliquez sur « Exporter les données ».",
  "import.guide.bitwarden.1":
    "Dans le coffre web ou l’application : Outils → Exporter le coffre.",
  "import.guide.bitwarden.2":
    "Choisissez le format de fichier « .json » (pas « chiffré »).",
  "import.guide.bitwarden.3": "Confirmez avec votre mot de passe principal.",
  "import.guide.keepass.1": "Utilisez votre fichier de base de données (.kdbx) tel quel.",
  "import.guide.keepass.2":
    "Vous saisirez son mot de passe à l’étape suivante. Il reste sur cet appareil.",
  "import.guide.keepassxcCsv.1": "KeePassXC : Base de données → Exporter → Fichier CSV…",
  "import.guide.keepassxcCsv.2":
    "Enregistrez-le dans Téléchargements. Le fichier .kdbx conserve davantage, donc préférez « KeePass » si vous le pouvez.",
  "import.guide.enpass.1": "Enpass : Menu → Fichier → Exporter.",
  "import.guide.enpass.2":
    "Choisissez le coffre-fort, le format « .json » et saisissez votre mot de passe principal.",
  "import.guide.enpass.3": "Enregistrez-le dans Téléchargements.",
  "import.guide.lastpass.1":
    "Connectez-vous sur lastpass.com et ouvrez Options avancées → Exporter.",
  "import.guide.lastpass.2":
    "Saisissez votre mot de passe principal. Le fichier CSV se télécharge.",
  "import.guide.lastpass.3":
    "Si le navigateur affiche du texte à la place, enregistrez la page sous « lastpass.csv ».",
  "import.guide.dashlane.1":
    "Ouvrez Dashlane dans votre navigateur → Mon compte → Paramètres → Exporter les données.",
  "import.guide.dashlane.2": "Choisissez « CSV ». Dashlane télécharge un fichier ZIP.",
  "import.guide.dashlane.3": "Laissez le fichier ZIP tel quel.",
  "import.guide.keeper.1": "Coffre web Keeper → Paramètres → Exporter.",
  "import.guide.keeper.2":
    "Choisissez « JSON » (il conserve les champs personnalisés et les codes 2FA) et exportez.",
  "import.guide.nordpass.1": "NordPass → Paramètres → Exporter les éléments.",
  "import.guide.nordpass.2":
    "Confirmez avec votre mot de passe principal et enregistrez le fichier CSV dans Téléchargements.",
  "import.guide.protonPass.1": "Proton Pass → Paramètres → Exporter.",
  "import.guide.protonPass.2":
    "Choisissez « ZIP » et laissez le chiffrement désactivé. Mosaic ne peut pas lire l’export chiffré avec PGP.",
  "import.guide.protonPass.3": "Déverrouillez et enregistrez-le dans Téléchargements.",
  "import.guide.roboform.1": "RoboForm → Paramètres → Compte et données → Exporter.",
  "import.guide.roboform.2":
    "Choisissez « CSV » et « Identifiants », puis enregistrez-le dans Téléchargements.",
  "import.guide.otherFile.1":
    "Exportez un fichier CSV avec une ligne d’en-tête, ou l’un des formats ci-dessus.",
  "import.guide.otherFile.2": "Choisissez le fichier. Mosaic associe les colonnes.",
  "import.guide.copyUrl": "Copier l’adresse",
  "import.guide.urlCopied": "Adresse copiée.",
  "import.guide.choose": "Choisir le fichier d’export",
  "import.guide.dropHint":
    "…ou déposez un fichier ici. Il est lu uniquement sur cet appareil.",
  "import.guide.back": "Retour",
  "import.detect.detected": "Reconnu : {source}",
  "import.detect.mismatch":
    "Ce fichier ressemble à un export de {source}. Mosaic le lira comme tel.",
  "import.detected.chromium": "Chrome, Edge ou un autre navigateur Chromium",
  "import.detected.firefox": "Firefox",
  "import.detected.safari": "Safari ou Mots de passe Apple",
  "import.detected.bitwardenCsv": "Bitwarden (CSV)",
  "import.detected.bitwarden": "Bitwarden",
  "import.detected.enpass": "Enpass",
  "import.detected.keepass": "KeePass",
  "import.detected.keepassxcCsv": "KeePassXC (CSV)",
  "import.detected.onePassword": "1Password",
  "import.detected.cxf": "un fichier FIDO Credential Exchange",
  "import.detected.lastpass": "LastPass",
  "import.detected.dashlane": "Dashlane",
  "import.detected.keeper": "Keeper",
  "import.detected.nordpass": "NordPass",
  "import.detected.protonPass": "Proton Pass",
  "import.detected.roboform": "RoboForm",
  "import.detected.csv": "un fichier CSV",
  "import.lost.title": "Ne sera pas transféré",
  "import.lost.passkeys":
    "Clés d’accès : {source} ne les inclut pas dans son export. Gardez {source} jusqu’à ce que vous ayez configuré de nouvelles clés d’accès.",
  "import.lost.attachments":
    "Pièces jointes : {source} ne les inclut pas dans son export. Ajoutez-les à la main.",
  "import.lost.history":
    "Anciens mots de passe (historique) : cet export ne les contient pas.",
  "import.lost.roboformExtras":
    "Identités, notes sécurisées et favoris : exportez-les séparément. Ils ne font pas partie du fichier des identifiants.",
  "import.lost.entriesSkipped":
    "{count, plural, one {# entrée n’a pas pu être lue et est ignorée.} other {# entrées n’ont pas pu être lues et sont ignorées.}}",
  "import.lost.fieldsDropped":
    "Certains éléments ont plus de champs que Mosaic n’en conserve. Les champs en trop sont ignorés.",
  "import.lost.notesTruncated":
    "Certaines notes sont plus longues que ce qu’un élément peut contenir. La partie qui ne tenait pas est ignorée.",
  "import.lost.attachmentsSkipped":
    "Certaines pièces jointes sont absentes, trop volumineuses ou ne tiennent pas dans votre espace de pièces jointes. Ajoutez-les à la main.",
  "import.cleanup.title": "Nettoyage",
  "import.cleanup.delete": "Supprimer {name} maintenant",
  "import.cleanup.deleted": "{name} a été écrasé puis supprimé.",
  "import.cleanup.deleteFailed": "{name} n’a pas pu être supprimé. Supprimez-le vous-même.",
  "import.cleanup.keepFile":
    "Certains éléments n’ont pas été importés, ou pas complètement. Gardez {name} jusqu’à ce que vous ayez vérifié ce qui manque, puis supprimez-le vous-même.",
  "import.cleanup.ssdNote":
    "Sur les SSD et dans les dossiers synchronisés, d’anciennes copies peuvent subsister : dans l’historique des versions, les sauvegardes ou la corbeille. Vérifiez-les aussi.",
  "import.cleanup.synced":
    "Ce fichier se trouve dans votre dossier {service}. {service} peut conserver des versions antérieures ; supprimez-les là-bas aussi.",
  "import.synced.oneDrive": "OneDrive",
  "import.synced.dropbox": "Dropbox",
  "import.synced.googleDrive": "Google Drive",
  "import.synced.icloud": "iCloud",
  "import.synced.box": "Box",
  "import.checklist.title": "Avant de quitter votre ancienne application",
  "import.checklist.browserSaving":
    "Désactivez l’enregistrement des mots de passe et la connexion automatique dans {source}, pour qu’il ne propose pas d’enregistrer en parallèle de Mosaic.",
  "import.checklist.firefoxSaving":
    "Ouvrez about:preferences#privacy et décochez « Demander d’enregistrer les identifiants et les mots de passe », pour que Firefox ne propose pas d’enregistrer en parallèle de Mosaic.",
  "import.checklist.safariSaving":
    "Safari → Réglages → Remplissage automatique : décochez « Noms d’utilisateur et mots de passe », pour que Safari ne remplisse pas en parallèle de Mosaic.",
  "import.checklist.oldExtension":
    "Désactivez l’extension de navigateur de {source} pour qu’une seule application remplisse.",
  "import.checklist.deleteOld":
    "Supprimez vos données dans {source} une fois que vous avez tout vérifié dans Mosaic.",
} satisfies Record<keyof typeof en, string>;
