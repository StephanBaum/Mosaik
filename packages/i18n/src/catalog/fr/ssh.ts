/**
 * French UI strings: ssh. Same keys as English (checked by the type).
 * ICU plural categories of French: one, many, other (CLDR; `other` is required).
 */
import type { ssh as en } from "../en/ssh.ts";

export const ssh = {
  "ssh.title": "Une application veut utiliser votre clé SSH",
  "ssh.key": "Clé",
  "ssh.fingerprint": "Empreinte",
  "ssh.client": "Demandé par",
  "ssh.client.unknown": "Un programme inconnu",
  "ssh.expires": "Refus automatique dans {seconds, plural, one {# seconde} other {# secondes}}",
  "ssh.deny": "Refuser",
  "ssh.allowOnce": "Autoriser une fois",
  "ssh.allowFor15Minutes": "Autoriser pendant 15 minutes",
  "ssh.queue":
    "{count, plural, one {# autre demande en attente} other {# autres demandes en attente}}",
  "ssh.signFailed.passphraseRequired":
    "La clé est chiffrée, mais l’élément n’a pas de phrase secrète. Ajoutez-la dans le champ Mot de passe de l’élément.",
  "ssh.signFailed.wrongPassphrase":
    "La phrase secrète de l’élément ne déverrouille pas la clé. Corrigez-la dans le champ Mot de passe de l’élément.",
  "ssh.signFailed.keyMalformed":
    "La clé privée de cet élément est illisible. Collez-la de nouveau au format OpenSSH.",
  "ssh.signFailed.kdfTooExpensive":
    "La protection par phrase secrète de la clé utilise plus de tours que Mosaic n’en autorise. Rechiffrez-la avec « ssh-keygen -p -a 64 ».",
  "ssh.signFailed.publicKeyMismatch":
    "La clé privée ne correspond pas à la clé publique de l’élément. Vérifiez les deux champs.",
  "ssh.signFailed.unsupported": "Mosaic ne signe qu’avec des clés Ed25519 et ECDSA P-256.",
  "ssh.signFailed.internal": "La signature a échoué. Réessayez.",
  "ssh.skip.unsupported":
    "Non proposée : Mosaic ne signe qu’avec des clés Ed25519 et ECDSA P-256.",
  "ssh.skip.malformed": "Non proposée : la clé est illisible.",
  "ssh.skip.duplicate": "Non proposée : un autre élément contient la même clé.",
  "ssh.description":
    "Une application a demandé à l’agent SSH de Mosaic de se connecter avec cette clé.",
  "ssh.origin.forwarded": "Un serveur auquel vous êtes connecté demande à utiliser cette clé.",
  "ssh.origin.bindFailed":
    "Mosaic ne peut pas savoir d’où vient cette demande : la vérification de l’hôte de la connexion a échoué. N’autorisez que si vous l’attendiez.",
  "ssh.escDenies": "pour refuser",
} satisfies Record<keyof typeof en, string>;
