/**
 * Brazilian Portuguese UI strings: mobileUx. Same keys as English (checked by the type).
 * ICU plural categories of Brazilian Portuguese: one, many, other (CLDR; `other` is required).
 */
import type { mobileUx as en } from "../en/mobileUx.ts";

export const mobileUx = {
  "mobile.sync.warningsBanner": "A sincronização encontrou um problema. Abra as Configurações para verificar.",
  "mobile.provider.errorTitle": "O Mosaic não pode continuar",
  "mobile.onboarding.details.keyWiped":
    "O Mosaic removeu a Chave Secreta lida quando você saiu do app. Leia o código de novo ou digite a chave.",
  "mobile.common.showField": "Mostrar {field}",
  "mobile.common.hideField": "Ocultar {field}",
  "mobile.error.storeInUse":
    "O Mosaic está ocupado em outro lugar neste celular. Aguarde um momento e tente de novo.",
  "mobile.error.noAccount":
    "O Mosaic ainda não está configurado neste celular. Abra o Mosaic para configurá-lo.",
  "mobile.error.wrongCredentials":
    "Essa senha mestra ou Chave Secreta não funcionou. Verifique erros de digitação e o idioma do teclado e tente de novo.",
  "mobile.error.invalidInput":
    "Algo que você digitou não é válido. Confira o que digitou e tente de novo.",
  "mobile.error.vaultUnavailable":
    "Este cofre não pode ser aberto agora. Sincronize e tente de novo, ou desbloqueie o Mosaic outra vez.",
  "mobile.unlock.quickOptionTouchId": "Desbloquear com Touch ID da próxima vez",
  "mobile.detail.readCode": "Ler código",
  "mobile.detail.labelValue": "{label}: {value}",
  "mobile.autofill.warningWithName": "Confira o endereço primeiro | {name}",
  "mobile.autofill.linkEntry": "Vincular {login} e preencher",
  "mobile.link.imported": "Importado para este app",
  "mobile.link.confirmTitle": "Vincular {login} a {app}?",
  "mobile.link.confirmSubtitle": "ID do app {appId} · Assinatura {fingerprint}",
  "mobile.link.confirmFallbackHint": "Após vincular, toque no campo de novo para preenchê-lo.",
  "mobile.link.importedNotVerified":
    "Login importado. O site não confirmou este app.",
} satisfies Record<keyof typeof en, string>;
