/**
 * Brazilian Portuguese UI strings: ssh. Same keys as English (checked by the type).
 * ICU plural categories of Brazilian Portuguese: one, many, other (CLDR; `other` is required).
 */
import type { ssh as en } from "../en/ssh.ts";

export const ssh = {
  "ssh.title": "Um app quer usar a sua chave SSH",
  "ssh.key": "Chave",
  "ssh.fingerprint": "Impressão digital",
  "ssh.client": "Solicitado por",
  "ssh.client.unknown": "Um programa desconhecido",
  "ssh.expires": "Recusado automaticamente em {seconds, plural, one {# segundo} other {# segundos}}",
  "ssh.deny": "Recusar",
  "ssh.allowOnce": "Permitir uma vez",
  "ssh.allowFor15Minutes": "Permitir por 15 minutos",
  "ssh.queue":
    "{count, plural, one {# solicitação a mais em espera} other {# solicitações a mais em espera}}",
  "ssh.signFailed.passphraseRequired":
    "A chave está criptografada, mas o item não tem frase secreta. Adicione-a no campo Senha do item.",
  "ssh.signFailed.wrongPassphrase":
    "A frase secreta do item não desbloqueia a chave. Corrija-a no campo Senha do item.",
  "ssh.signFailed.keyMalformed":
    "Não é possível ler a chave privada deste item. Cole-a de novo no formato OpenSSH.",
  "ssh.signFailed.kdfTooExpensive":
    "A proteção por frase secreta da chave usa mais rodadas do que o Mosaic permite. Criptografe-a de novo com “ssh-keygen -p -a 64”.",
  "ssh.signFailed.publicKeyMismatch":
    "A chave privada não corresponde à chave pública do item. Confira os dois campos.",
  "ssh.signFailed.unsupported":
    "O Mosaic só assina com chaves Ed25519 e ECDSA P-256.",
  "ssh.signFailed.internal": "A assinatura falhou. Tente de novo.",
  "ssh.skip.unsupported":
    "Não disponibilizada: o Mosaic só assina com chaves Ed25519 e ECDSA P-256.",
  "ssh.skip.malformed": "Não disponibilizada: não é possível ler a chave.",
  "ssh.skip.duplicate": "Não disponibilizada: outro item tem a mesma chave.",
  "ssh.description": "Um app pediu ao agente SSH do Mosaic para entrar com esta chave.",
  "ssh.origin.forwarded": "Um servidor ao qual você está conectado pede para usar esta chave.",
  "ssh.origin.bindFailed":
    "O Mosaic não consegue saber de onde vem esta solicitação: a verificação do host da conexão falhou. Só permita se você a esperava.",
  "ssh.escDenies": "para recusar",
} satisfies Record<keyof typeof en, string>;
