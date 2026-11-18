/**
 * Brazilian Portuguese UI strings: shell. Same keys as English (checked by the type).
 * ICU plural categories of Brazilian Portuguese: one, many, other (CLDR; `other` is required).
 */
import type { shell as en } from "../en/shell.ts";

export const shell = {
  "shell.emergencyKit.title": "Kit de Emergência do Mosaic",
  "shell.emergencyKit.intro":
    "Guarde esta página em um lugar seguro, como uma gaveta trancada. Sua Chave Secreta e a sua senha mestra juntas desbloqueiam o seu cofre em um novo dispositivo. Ninguém, nem mesmo o Mosaic, pode recuperar nenhuma das duas para você.",
  "shell.emergencyKit.email": "E-mail da conta",
  "shell.emergencyKit.accountId": "ID da conta",
  "shell.emergencyKit.signInUrl": "Endereço de login",
  "shell.emergencyKit.signInUrlLocal":
    "Somente este dispositivo (nenhum servidor de sincronização configurado ainda)",
  "shell.emergencyKit.secretKey": "Chave Secreta",
  "shell.emergencyKit.masterPassword": "Senha mestra",
  "shell.emergencyKit.masterPasswordHint":
    "Escreva-a à mão na linha somente se esta página ficar em um lugar seguro. Caso contrário, guarde-a na cabeça.",
  "shell.emergencyKit.footer":
    "O Mosaic é um software pré-auditoria: ainda não foi revisado por um auditor de segurança externo.",
  "shell.shortcut.conflict":
    "Outro app já usa {shortcut}. Feche esse app ou mude o atalho dele e reinicie o Mosaic para usar a busca rápida.",
  "keys.ctrl": "Ctrl",
  "keys.shift": "Shift",
  "keys.alt": "Alt",
  "keys.space": "Espaço",
  "keys.esc": "Esc",
  "keys.enter": "Enter",
} satisfies Record<keyof typeof en, string>;
