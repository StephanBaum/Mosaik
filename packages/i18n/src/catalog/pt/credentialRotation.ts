/**
 * Brazilian Portuguese UI strings: credentialRotation. Same keys as English (checked by the type).
 * ICU plural categories of Brazilian Portuguese: one, many, other (CLDR; `other` is required).
 */
import type { credentialRotation as en } from "../en/credentialRotation.ts";

export const credentialRotation = {
  "credentialRotation.tablist": "Visões de segurança",
  "credentialRotation.tab.overview": "Visão geral",
  "credentialRotation.tab.rotate": "Trocar agora",
  "credentialRotation.tab.dueHidden": ", {count} para trocar",
  "credentialRotation.queue.title": "Trocar agora",
  "credentialRotation.queue.intro":
    "As senhas que mais vale a pena trocar primeiro. O Mosaic orienta você, mas é você quem troca cada senha no site.",
  "credentialRotation.queue.empty": "Nada para trocar agora.",
  "credentialRotation.queue.count":
    "{count, plural, one {# senha para trocar} other {# senhas para trocar}}",
  "credentialRotation.queue.list": "Senhas para trocar",
  "credentialRotation.queue.snoozedHeading": "Adiadas",
  "credentialRotation.queue.snoozedUntil": "Adiada até {date, date, medium}",
  "credentialRotation.queue.reason.weak": "Fraca",
  "credentialRotation.queue.reason.reused": "Reutilizada",
  "credentialRotation.queue.reason.old": "Antiga",
  "credentialRotation.queue.reason.breached": "Em um vazamento",
  "credentialRotation.queue.reason.unfinished": "Troca não concluída",
  "credentialRotation.queue.change": "Trocar senha",
  "credentialRotation.queue.continue": "Continuar troca",
  "credentialRotation.queue.editItem": "Editar item",
  "credentialRotation.queue.manualHint":
    "O Mosaic não consegue orientar esta troca. Abra o item e troque você mesmo.",
  "credentialRotation.queue.actionFor": "{action}: {title}",
  "credentialRotation.queue.snooze": "Adiar",
  "credentialRotation.queue.snoozeFor": "Adiar {title}",
  "credentialRotation.queue.snoozeDays": "{days, plural, one {# dia} other {# dias}}",
  "credentialRotation.queue.snoozed": "Adiada por {days, plural, one {# dia} other {# dias}}.",
  "credentialRotation.queue.dismiss": "Dispensar",
  "credentialRotation.queue.dismissFor": "Dispensar {title}",
  "credentialRotation.queue.dismissed": "Dispensada. Ela volta quando a senha mudar.",
  "credentialRotation.queue.undo": "Desfazer",
  "credentialRotation.panel.title": "Trocar a senha de {title}",
  "credentialRotation.panel.step": "Etapa {current} de {total}",
  "credentialRotation.panel.stays":
    "A troca de {title} continua aberta. A nova senha fica guardada no item até você escolher Funcionou ou Não funcionou.",
  "credentialRotation.panel.cancel": "Cancelar",
  "credentialRotation.steps.new.title": "Escolha uma nova senha",
  "credentialRotation.steps.new.intro":
    "O Mosaic salva a nova senha neste item antes de você usá-la em qualquer lugar, para que ela não se perca. A senha atual permanece até você confirmar.",
  "credentialRotation.steps.new.label": "Nova senha",
  "credentialRotation.steps.new.use": "Usar esta senha",
  "credentialRotation.steps.new.saving": "Salvando…",
  "credentialRotation.steps.site.title": "Troque no site",
  "credentialRotation.steps.site.pageLabel": "Página de troca",
  "credentialRotation.steps.site.body":
    "Abra a página de senha do site no navegador, clique no Mosaic na barra de ferramentas e escolha Preencher formulário de troca. O Mosaic nunca envia o formulário.",
  "credentialRotation.steps.site.copyLink": "Copiar link da página de troca",
  "credentialRotation.steps.site.linkCopied": "Link copiado.",
  "credentialRotation.steps.site.linkHint":
    "O link não é um segredo. Se o site não tiver uma página nesse endereço, procure as configurações de senha dele manualmente.",
  "credentialRotation.steps.site.noLink":
    "Este login não tem endereço https, então o Mosaic não tem uma página de troca para sugerir. Troque a senha no site manualmente.",
  "credentialRotation.steps.site.copyPassword": "Copiar nova senha",
  "credentialRotation.steps.site.next": "Já tentei",
  "credentialRotation.steps.result.title": "O site aceitou a nova senha?",
  "credentialRotation.steps.done.failed": "Senha atual mantida",
  "credentialRotation.steps.done.notSure": "Troca deixada em aberto",
  "credentialRotation.steps.result.body":
    "O Mosaic não enxerga o site. Conte a ele o que aconteceu para que a senha certa fique neste item. Nada é excluído em nenhum dos casos.",
  "credentialRotation.steps.result.working": "Salvando…",
  "credentialRotation.result.worked": "Funcionou",
  "credentialRotation.result.failed": "Não funcionou",
  "credentialRotation.result.notSure": "Ainda não sei",
  "credentialRotation.result.workedDone":
    "Pronto. A nova senha agora é a senha deste item. A antiga está no histórico.",
  "credentialRotation.result.failedDone":
    "A senha atual continua. A nova fica no histórico como não confirmada, caso o site a tenha aplicado mesmo assim.",
  "credentialRotation.result.notSureDone":
    "A troca continua aberta neste item. Volte quando souber.",
  "credentialRotation.banner.title": "Troca de senha iniciada em {date, date, medium}",
  "credentialRotation.banner.prepared":
    "A nova senha fica guardada aqui até você confirmar. Nenhuma página a recebeu ainda.",
  "credentialRotation.banner.delivered":
    "A nova senha fica guardada aqui até você confirmar. A extensão do navegador a preencheu no formulário do site.",
  "credentialRotation.banner.reveal": "Mostrar nova senha",
  "credentialRotation.banner.hide": "Ocultar nova senha",
  "credentialRotation.banner.copy": "Copiar nova senha",
  "credentialRotation.banner.newPassword": "Nova senha",
  "credentialRotation.banner.copyLink": "Copiar link da página de troca",
  "credentialRotation.history.title": "Histórico de senhas",
  "credentialRotation.history.list": "Senhas anteriores",
  "credentialRotation.history.entry": "Substituída em {date, date, medium}",
  "credentialRotation.history.unconfirmed": "Não confirmada",
  "credentialRotation.history.unconfirmedEntry": "Tentada em {date, date, medium}",
  "credentialRotation.history.unconfirmedHint":
    "O site pode ter aplicado esta. Restaure-a se o login com a senha atual falhar.",
  "credentialRotation.history.reveal": "Mostrar a senha de {date, date, medium}",
  "credentialRotation.history.hide": "Ocultar a senha de {date, date, medium}",
  "credentialRotation.history.restore": "Restaurar",
  "credentialRotation.history.restoreFor": "Restaurar a senha de {date, date, medium}",
  "credentialRotation.history.restored": "Restaurada. A senha anterior está no histórico.",
  "credentialRotation.policy.label": "Lembrar-me de trocar esta senha",
  "credentialRotation.policy.none": "Nunca",
  "credentialRotation.policy.every": "A cada {days} dias",
  "credentialRotation.policy.help":
    "Um lembrete coloca o item na lista Trocar agora. O Mosaic não envia notificação nem faz nenhuma solicitação por isso.",
  "credentialRotation.url.help":
    "Opcional. O endereço https da página de senha do site, no mesmo site deste login. O Mosaic só o abre quando você escolhe Abrir página de troca na extensão do navegador.",
  "credentialRotation.url.invalid":
    "O Mosaic não usará este endereço. Ele precisa começar com https:// e estar no mesmo site deste login.",
  "credentialRotation.settings.title": "Lembretes de senha",
  "credentialRotation.settings.body":
    "Depois que você desbloqueia, o Mosaic pode informar quantas senhas vale a pena trocar.",
  "credentialRotation.settings.reminders": "Mostrar quantas senhas trocar após desbloquear",
  "credentialRotation.settings.remindersHint":
    "Apenas uma mensagem no app. Sem notificação, sem solicitação de rede.",
  "credentialRotation.reminder.toast":
    "{count, plural, one {# senha para trocar} other {# senhas para trocar}}",
  "credentialRotation.reminder.show": "Mostrar",
  "credentialRotation.dialog.beginTitle": "Iniciar uma troca de senha?",
  "credentialRotation.dialog.beginBody":
    "{browser} quer trocar a senha de {title} em {host}. O Mosaic guarda a nova senha junto a este login e mantém a atual até você confirmar que o site a aceitou.",
  "credentialRotation.dialog.site": "Site",
  "credentialRotation.dialog.login": "Login",
  "credentialRotation.dialog.password": "Nova senha",
  "credentialRotation.dialog.hidden": "Oculta",
  "credentialRotation.dialog.show": "Mostrar",
  "credentialRotation.dialog.hide": "Ocultar",
  "credentialRotation.dialog.regenerate": "Gerar outra",
  "credentialRotation.dialog.regenerated": "Uma nova senha foi gerada.",
  "credentialRotation.dialog.allow": "Usar esta senha",
  "credentialRotation.dialog.cancel": "Cancelar",
  "credentialRotation.dialog.decideLater": "Decidir depois",
  "credentialRotation.dialog.untitled": "este login",
  "credentialRotation.dialog.attempt":
    "Iniciada em {at, date, medium}, {at, time, short} · {stage}{n, plural, =0 {} one { · # outra troca aberta} other { · # outras trocas abertas}}",
  "credentialRotation.dialog.stagePrepared": "nenhuma página tem a nova senha ainda",
  "credentialRotation.dialog.stageDelivered": "a nova senha foi preenchida em uma página",
  "credentialRotation.dialog.resultTitle": "{host} aceitou a nova senha?",
  "credentialRotation.dialog.resultBody":
    "{browser} informa que a troca funcionou. Se você confirmar, a nova senha passa a ser a senha de {title}, e a atual vai para o histórico.",
  "credentialRotation.dialog.confirm": "Funcionou",
  "credentialRotation.dialog.keepOld": "Manter senha antiga",
  "credentialRotation.dialog.failedTitle": "Manter a senha antiga?",
  "credentialRotation.dialog.failedBody":
    "{browser} informa que a troca não funcionou. O Mosaic mantém a senha atual de {title}. O novo valor fica no histórico, caso o site o tenha alterado mesmo assim.",
  "credentialRotation.addOrigin.title": "Adicionar {host} a {title}?",
  "credentialRotation.addOrigin.body":
    "O Mosaic também preencherá este login em {host}. Só adicione se {host} realmente fizer parte do mesmo site.",
  "credentialRotation.addOrigin.idn":
    "Este endereço usa caracteres especiais. Seu navegador pode exibi-lo como {unicode}. Confira com atenção.",
  "credentialRotation.addOrigin.add": "Adicionar endereço",
  "credentialRotation.addOrigin.cancel": "Agora não",
} satisfies Record<keyof typeof en, string>;
