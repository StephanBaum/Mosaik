/**
 * Brazilian Portuguese UI strings: otp. Same keys as English (checked by the type).
 * ICU plural categories of Brazilian Portuguese: one, many, other (CLDR; `other` is required).
 */
import type { otp as en } from "../en/otp.ts";

export const otp = {
  "otp.section.label": "Código de uso único (2FA)",
  "otp.section.empty": "Adicione o código da página de configuração de duas etapas do site.",
  "otp.section.required": "Adicione um código de uso único para salvar este item.",
  "otp.source.screen": "Ler a tela",
  "otp.source.clipboard": "Colar captura de tela",
  "otp.source.file": "Escolher imagem…",
  "otp.source.manual": "Não consegue ler? Digite a chave",
  "otp.source.screenHint":
    "O Mosaic sai da frente por um instante, olha a sua tela uma vez e guarda apenas o código.",
  "otp.source.clipboardHint":
    "{platform, select, mac {Pressione Cmd+Ctrl+Shift+4, selecione o código QR e cole aqui.} windows {Pressione Win+Shift+S, selecione o código QR e cole aqui.} other {Copie uma captura de tela do código QR e cole aqui.}}",
  "otp.dialog.title": "Configurar um código de uso único",
  "otp.working.screen": "Procurando um código QR na sua tela…",
  "otp.working.image": "Lendo o código QR…",
  "otp.manual.title": "Digite a chave de configuração",
  "otp.manual.input": "Chave de configuração ou link otpauth://",
  "otp.manual.inputHint":
    "O site a mostra ao lado do código QR, muitas vezes em “Não consegue ler?” ou “Inserir manualmente”. Espaços não importam.",
  "otp.manual.issuer": "Serviço",
  "otp.manual.account": "Conta",
  "otp.manual.advanced": "Avançado",
  "otp.manual.algorithm": "Algoritmo",
  "otp.manual.digits": "Dígitos",
  "otp.manual.period": "Novo código a cada",
  "otp.manual.periodValue": "{seconds} segundos",
  "otp.manual.advancedHint":
    "Altere isto só se o site pedir. A maioria dos sites usa SHA-1, 6 dígitos e 30 segundos.",
  "otp.manual.continue": "Continuar",
  "otp.confirm.title": "Este é o código certo?",
  "otp.confirm.issuer": "Serviço",
  "otp.confirm.account": "Conta",
  "otp.confirm.details":
    "{scheme, select, hotp {Baseado em contador, {digits} dígitos} steam {Steam Guard, 5 caracteres} other {{digits} dígitos, novo código a cada {period} segundos}}",
  "otp.confirm.algorithmValue":
    "{algorithm, select, sha256 {SHA-256} sha512 {SHA-512} other {SHA-1}}",
  "otp.confirm.codeLabel": "Código atual",
  "otp.confirm.codeHint":
    "Se o site pedir um código para concluir a configuração, digite este.",
  "otp.confirm.hotpNote":
    "Os códigos baseados em contador aparecem depois que você salva, para que o contador fique em sincronia com o site.",
  "otp.confirm.weak":
    "Este segredo é incomumente curto. Funciona, mas a proteção de duas etapas deste site é mais fraca que o normal.",
  "otp.confirm.use": "Usar este código",
  "otp.confirm.back": "Ler de novo",
  "otp.confirm.copy": "Copiar código",
  "otp.summary.noIssuer": "Código de uso único",
  "otp.summary.pending": "Salvo junto com este item.",
  "otp.summary.removed": "O código de uso único será removido quando você salvar.",
  "otp.action.replace": "Substituir",
  "otp.action.remove": "Remover código de uso único",
  "otp.action.keep": "Manter",
  "otp.action.cancelSetup": "Não adicionar",
  "otp.storage.label": "Onde guardar o código",
  "otp.storage.item": "Com este item",
  "otp.storage.itemTradeoff":
    "Conveniente, mas quem abrir este cofre recebe a sua senha e o seu segundo fator juntos.",
  "otp.storage.hardware": "Em {vault} (exige a sua chave de segurança)",
  "otp.storage.hardwareClosed": "Abra antes {vault} com a sua chave de segurança.",
  "otp.toast.clipboardCleared": "Captura de tela lida. O Mosaic limpou a área de transferência.",
  "otp.toast.savedInVault": "Código salvo em {vault}.",
  "otp.clipboardCopies":
    "{platform, select, windows {Se o histórico da área de transferência ou a Ferramenta de Captura guardou uma cópia da captura de tela, exclua-a lá.} mac {Se você também salvou a captura de tela como arquivo, exclua-o.} other {Se um gerenciador da área de transferência guardou uma cópia da captura de tela, exclua-a lá.}}",
} satisfies Record<keyof typeof en, string>;
