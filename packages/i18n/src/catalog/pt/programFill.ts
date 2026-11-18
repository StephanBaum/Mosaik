/**
 * Brazilian Portuguese UI strings: programFill. Same keys as English (checked by the type).
 * ICU plural categories of Brazilian Portuguese: one, many, other (CLDR; `other` is required).
 */
import type { programFill as en } from "../en/programFill.ts";

export const programFill = {
  "programFill.chip.label": "Preencher com o Mosaic",
  "programFill.chip.locked": "Desbloqueie o Mosaic para preencher",
  "programFill.list.title": "Logins para {program}",
  "programFill.list.none": "Nenhum login para {program} ainda",
  "programFill.list.signerChanged": "O assinante deste programa mudou. Vincular de novo?",
  "programFill.list.fileChanged": "Este programa mudou desde que você o vinculou. Vincular de novo?",
  "programFill.list.suggestions": "Sugestões",
  "programFill.list.search": "Buscar logins",
  "programFill.list.link": "Vincular um login…",
  "programFill.list.block": "Não oferecer para este programa",
  "programFill.link.packaged": "{name} · App da loja · Editor {publisher}",
  "programFill.link.packagedNoPublisher": "{name} · App da loja",
  "programFill.link.signed": "{exe} · Assinado por {signer}, verificado pelo Windows",
  "programFill.link.unsigned": "{exe} · Sem assinatura · {path}",
  "programFill.link.unsignedNote":
    "O Mosaic preencherá apenas este arquivo exato. Se ele mudar, o Mosaic pergunta de novo.",
  "programFill.link.lookalike":
    "Cuidado: este nome é parecido com {similar}, que você vinculou antes.",
  "programFill.link.hashChange": "Arquivo vinculado {old}…, este arquivo {new}…",
  "programFill.link.login": "Login: {title}",
  "programFill.link.confirm": "Vincular e entrar",
  "programFill.link.confirmFill": "Vincular e preencher",
  "programFill.link.confirmUnsigned": "Vincular este programa sem assinatura",
  "programFill.link.confirmAgain": "Confirme de novo para vincular",
  "programFill.notice.elevated":
    "O Mosaic não pode preencher programas executados como administrador. Copie a senha do Mosaic.",
  "programFill.notice.noField": "O Mosaic não vê um campo de login aqui.",
  "programFill.notice.aborted":
    "O Mosaic parou de preencher porque a janela mudou. Confira o campo.",
  "programFill.notice.abortedCleared":
    "O Mosaic parou de preencher porque a janela mudou e removeu o que havia digitado.",
  "programFill.notice.modifiersHeld":
    "Solte Shift, Ctrl, Alt e a tecla Windows e escolha o login de novo.",
  "programFill.notice.notResponding":
    "Este programa não respondeu a tempo. Nada mais foi preenchido.",
  "programFill.notice.close": "Fechar",
  "programFill.settings.title": "Preencher em programas",
  "programFill.settings.description":
    "O Mosaic observa qual campo está em foco em outros programas neste dispositivo para oferecer os seus logins ali. Nada é armazenado nem enviado.",
  "programFill.settings.enable": "Oferecer logins em programas",
  "programFill.settings.submit": "Entrar após preencher",
  "programFill.settings.hotkey": "Atalho para abrir a lista",
  "programFill.settings.hotkeyNone": "Sem atalho",
  "programFill.settings.hotkeySave": "Salvar atalho da lista",
  "programFill.settings.blocked": "Programas para os quais o Mosaic não oferece logins",
  "programFill.settings.blockedNone": "Nenhum",
  "programFill.settings.unblock": "Oferecer de novo",
  "programFill.settings.notWindows": "Por enquanto, apenas no Windows.",
  "programFill.settings.mosaicElevated":
    "O Mosaic está sendo executado como administrador, então não preenche programas. Inicie-o normalmente.",
  "programFill.settings.hostFailed":
    "O Mosaic não conseguiu começar a observar programas. Reinicie o Mosaic para tentar de novo.",
  "programFill.card.title": "Preencha logins também em programas",
  "programFill.card.turnOn": "Ativar",
  "programFill.card.notNow": "Agora não",
  "programFill.item.program": "Programa",
  "programFill.item.storeApp": "App da loja",
  "programFill.item.signedBy": "Assinado por {signer}",
  "programFill.item.unsigned": "Sem assinatura: {path}",
  "programFill.item.submit": "Entrar após preencher",
  "programFill.item.unlink": "Desvincular",
} satisfies Record<keyof typeof en, string>;
