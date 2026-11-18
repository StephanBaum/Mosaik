/**
 * Brazilian Portuguese UI strings: ext. Same keys as English (checked by the type).
 * ICU plural categories of Brazilian Portuguese: one, many, other (CLDR; `other` is required).
 */
import type { ext as en } from "../en/ext.ts";

export const ext = {
  "ext.manifest.name": "Mosaic",
  "ext.manifest.description":
    "Preencha logins a partir do seu cofre do Mosaic. Sua senha mestra fica no app para computador.",
  "ext.command.fill": "Preencher login com o Mosaic",
  "ext.command.generate": "Gerar uma senha com o Mosaic",
  "ext.save.badgeTitle": "Salvar login?",
  "ext.loading": "Carregando…",
  "ext.site.secure": "Site atual, conexão segura (https): {host}",
  "ext.site.insecure": "Site atual, sem criptografia (http): {host}",
  "ext.site.none": "Esta aba não tem um site",
  "ext.nav.label": "Visões",
  "ext.nav.logins": "Logins e busca",
  "ext.nav.generator": "Gerador de senhas",
  "ext.footer.open": "Abrir o Mosaic",
  "ext.footer.more": "Mais opções",
  "ext.footer.connected": "Conectado ao Mosaic",
  "ext.footer.locked": "O Mosaic está bloqueado",
  "ext.footer.offline": "Não conectado",
  "ext.more.saveOffers": "Oferecer para salvar logins",
  "ext.more.saveOffersHint":
    "Precisa de acesso a todos os sites, para que a extensão perceba um login que você envia. Nada é salvo até você confirmar no app Mosaic.",
  "ext.passkeys.toggle": "Usar o Mosaic para chaves de acesso",
  "ext.passkeys.hint":
    "Precisa de acesso a todos os sites para responder a solicitações de chave de acesso. Você confirma cada uma no app Mosaic.",
  "ext.passkeys.permissionDenied":
    "O Mosaic precisa de acesso aos sites para responder a solicitações de chave de acesso, e você não permitiu.",
  "ext.more.disconnect": "Desconectar do Mosaic",
  "ext.more.disconnectTitle": "Desconectar este navegador?",
  "ext.more.disconnectBody":
    "Este navegador não poderá preencher logins até você conectá-lo de novo. Você aprova a nova conexão no app Mosaic.",
  "ext.action.cancel": "Cancelar",
  "ext.neverSave.menu": "Sites que nunca oferecem salvar…",
  "ext.neverSave.title": "Nunca oferecer para salvar",
  "ext.neverSave.intro":
    "O Mosaic não oferece para salvar logins nestes sites. Remova um site para voltar a ser perguntado nele.",
  "ext.neverSave.empty":
    "Nenhum site ainda. Você pode adicionar um com “Nunca neste site” quando o Mosaic oferecer para salvar um login.",
  "ext.neverSave.remove": "Remover",
  "ext.neverSave.removeLabel": "Remover {origin} da lista",
  "ext.neverSave.removed":
    "{origin} removido. O Mosaic pode voltar a oferecer para salvar logins ali.",
  "ext.neverSave.close": "Fechar",
  "ext.connect.checking": "Conectando ao Mosaic…",
  "ext.connect.hostMissing.title": "O Mosaic não está configurado para este navegador",
  "ext.connect.hostMissing.body":
    "Instale o app Mosaic neste computador e reinicie o navegador.",
  "ext.connect.hostMissing.dev": "Desenvolvedores: execute {command} e reinicie o navegador.",
  "ext.connect.hostForbidden.title": "Este navegador não tem permissão para se conectar",
  "ext.connect.hostForbidden.body":
    "O Mosaic só conversa com a extensão oficial do Mosaic. Instale-a pela loja do seu navegador.",
  "ext.connect.desktopNotRunning.title": "Abra o Mosaic neste computador",
  "ext.connect.desktopNotRunning.body":
    "A extensão funciona por meio do app Mosaic. Inicie o app e verifique de novo.",
  "ext.connect.check": "Verificar de novo",
  "ext.connect.notPaired.title": "Conectar ao Mosaic",
  "ext.connect.notPaired.body":
    "Conecte este navegador ao app Mosaic neste computador. Você aprova no app.",
  "ext.connect.start": "Conectar",
  "ext.connect.pairing.title": "Confirme no Mosaic",
  "ext.connect.pairing.body":
    "Verifique se o Mosaic mostra o mesmo código e clique em Permitir lá.",
  "ext.connect.pairing.code": "Código",
  "ext.connect.pairing.waiting": "Aguardando a sua resposta no app Mosaic…",
  "ext.connect.protocolMismatch.title": "Atualize o Mosaic",
  "ext.connect.protocolMismatch.body":
    "A extensão e o app Mosaic estão em versões diferentes. Atualize os dois para a versão mais recente.",
  "ext.connect.error.title": "Não foi possível acessar o Mosaic",
  "ext.connect.error.body": "Verifique se o app Mosaic está em execução e tente de novo.",
  "ext.locked.title": "O Mosaic está bloqueado",
  "ext.locked.body": "Desbloqueie-o no app Mosaic para preencher em {host}.",
  "ext.locked.bodyNoSite": "Desbloqueie-o no app Mosaic para usá-lo aqui.",
  "ext.locked.open": "Abrir o Mosaic",
  "ext.locked.note": "Sua senha mestra só é digitada no app Mosaic.",
  "ext.logins.forSite": "Para {host}",
  "ext.logins.count": "{count, plural, one {# login} other {# logins}}",
  "ext.logins.sameSite": "mesmo site",
  "ext.logins.none": "Nenhum login salvo para {host}.",
  "ext.logins.noneHint":
    "O Mosaic compara o endereço completo, então sites parecidos nunca correspondem.",
  "ext.logins.unsupported":
    "O Mosaic não pode preencher nesta página. Pesquise no seu cofre para copiar um login.",
  "ext.logins.shortcut": "{shortcut} preenche a primeira correspondência.",
  "ext.logins.frameNote":
    "O Mosaic só preenche em {host}, nunca dentro de quadros incorporados de outros sites.",
  "ext.item.untitled": "Sem título",
  "ext.item.noUsername": "Sem nome de usuário",
  "ext.item.twoFactor": "2FA",
  "ext.item.fill": "Preencher",
  "ext.item.fillLabel": "Preencher {name}",
  "ext.item.copyMenu": "Opções de cópia para {name}",
  "ext.item.copyUsername": "Copiar nome de usuário",
  "ext.item.copyPassword": "Copiar senha",
  "ext.item.copyCode": "Copiar código",
  "ext.item.blocked": "Bloqueado: esta página não tem criptografia e este login não permite isso.",
  "ext.search.label": "Outros itens",
  "ext.search.placeholder": "Pesquisar no seu cofre",
  "ext.search.note":
    "Os resultados ficam neste pop-up. A página só recebe o único login que você preenche.",
  "ext.search.none": "Nada encontrado.",
  "ext.search.copy": "Copiar",
  "ext.search.copyOnly": "Somente copiar: não salvo para este site",
  "ext.shortcut.copyUser": "{platform, select, mac {⌘C} other {Ctrl+C}}",
  "ext.shortcut.copyPassword": "{platform, select, mac {⇧⌘C} other {Ctrl+Shift+C}}",
  "ext.shortcut.copyCode": "{platform, select, mac {⌥⌘C} other {Ctrl+Alt+C}}",
  "ext.search.results": "{count, plural, one {# resultado} other {# resultados}}",
  "ext.frame.cross":
    "O campo em foco está em um quadro de outro site. O Mosaic não preenche ali.",
  "ext.frameConfirm.title": "Preencher dentro de um quadro incorporado?",
  "ext.frameConfirm.body":
    "O campo está em um quadro em {host}, não na própria página. Só preencha aqui se você confiar neste quadro.",
  "ext.frameConfirm.confirm": "Preencher no quadro",
  "ext.warning.idn.title": "Confira o endereço",
  "ext.warning.idn.body":
    "Este endereço usa caracteres especiais e pode se parecer com outro site. Confira a barra de endereços antes de entrar.",
  "ext.warning.lookalike.body":
    "Este site não é {saved}. Ele é parecido, um truque comum de phishing. Feche esta aba ou digite {saved} você mesmo.",
  "ext.warning.youAreOn": "Você está em",
  "ext.warning.writtenAs": "Escrito como",
  "ext.warning.savedFor": "Salvo para",
  "ext.warning.insecure.title": "Esta página não tem criptografia",
  "ext.warning.insecure.body":
    "A página usa http. Outras pessoas na rede poderiam ler a sua senha. O Mosaic só preenche aqui para logins em que você permitiu.",
  "ext.otp.title": "Preencher o seu código de uso único",
  "ext.otp.next": "Senha preenchida para {name}. Em seguida: o código de verificação em duas etapas.",
  "ext.otp.for": "Código para {name}",
  "ext.otp.fill": "Preencher código",
  "ext.otp.copy": "Copiar código",
  "ext.otp.note":
    "O código vai do Mosaic direto para o campo. Este pop-up nunca o mostra.",
  "ext.save.titleNew": "Salvar este login para {host}?",
  "ext.save.titleUpdate": "Atualizar o login salvo para {host}?",
  "ext.save.generated": "Uma senha que o Mosaic gerou para {host}.",
  "ext.save.updates": "Atualiza “{title}”.",
  "ext.save.username": "Nome de usuário",
  "ext.save.noUsername": "Nenhum",
  "ext.save.password": "Senha",
  "ext.save.passwordHidden": "Oculta. Você a vê no app Mosaic.",
  "ext.save.insecure": "Esta página não tem criptografia (http).",
  "ext.save.action": "Salvar no Mosaic…",
  "ext.save.notNow": "Agora não",
  "ext.save.never": "Nunca neste site",
  "ext.save.waiting": "Confirme no app Mosaic. Nada é salvo até você confirmar.",
  "ext.save.saved": "Salvo no Mosaic.",
  "ext.save.updated": "Atualizado no Mosaic.",
  "ext.save.cancelled": "Não salvo.",
  "ext.save.dismissed": "Descartado. A senha é esquecida.",
  "ext.save.neverDone": "O Mosaic não oferecerá mais para salvar em {host}.",
  "ext.save.permissionAsk":
    "O Mosaic pede ao seu navegador acesso aos sites. Permita para ativar isto.",
  "ext.generator.title": "Gerar uma senha para {host}",
  "ext.generator.titleGeneric": "Gerar uma senha",
  "ext.generator.value": "Senha gerada",
  "ext.generator.regenerate": "Gerar outra senha",
  "ext.generator.type": "Tipo de senha",
  "ext.generator.random": "Aleatória",
  "ext.generator.passphrase": "Frase secreta",
  "ext.generator.include": "Incluir",
  "ext.generator.rules.page": "Segue as regras de senha desta página",
  "ext.generator.rules.siteQuirks": "Segue as regras conhecidas deste site",
  "ext.generator.fill": "Preencher",
  "ext.generator.fillHint":
    "Para preencher, clique em um campo de nova senha na página e abra este pop-up de novo.",
  "ext.generator.copy": "Copiar",
  "ext.generator.expired": "Essa senha não estava mais guardada, então uma nova foi gerada.",
  "ext.generator.filled": "Preenchida. O Mosaic oferecerá para salvá-la.",
  "ext.copied": "Copiado. Será limpo em {seconds, plural, one {# segundo} other {# segundos}}.",
  "ext.copiedNoClear": "Copiado.",
  "ext.filled": "Preenchido.",
  "ext.filledCode": "Código preenchido.",
  "ext.error.invalidRequest":
    "O Mosaic não entendeu essa solicitação. Atualize a extensão e o app Mosaic.",
  "ext.error.notPaired": "Este navegador não está conectado ao Mosaic. Conecte-o de novo.",
  "ext.error.protocolVersion":
    "A extensão e o app Mosaic estão em versões diferentes. Atualize os dois.",
  "ext.error.busy": "O Mosaic está ocupado com outra solicitação. Tente de novo em instantes.",
  "ext.error.locked": "O Mosaic está bloqueado. Desbloqueie-o no app Mosaic.",
  "ext.error.rateLimited":
    "Foram solicitações demais em pouco tempo. Aguarde um momento e tente de novo.",
  "ext.error.originMismatch":
    "Este login não está salvo para este site, então o Mosaic não o preencherá aqui.",
  "ext.error.insecureOrigin":
    "Esta página não tem criptografia e o login não permite preenchimento nela. Nada foi preenchido.",
  "ext.error.crossOriginFrame":
    "Esse campo está em um quadro de outro site. O Mosaic não preenche ali.",
  "ext.error.itemNotFound": "Esse item não existe mais. Ele pode ter sido movido ou excluído.",
  "ext.error.vaultUnavailable": "Esse cofre está fechado. Abra-o antes no app Mosaic.",
  "ext.error.noOtp": "Este item não tem código de uso único.",
  "ext.error.otpNotFillable":
    "Este código aumenta a cada uso (HOTP), então não pode ser preenchido nem copiado daqui. Use-o no app Mosaic.",
  "ext.error.fieldEmpty": "Esse campo está vazio, então não há nada para preencher ou copiar.",
  "ext.error.clipboardUnavailable":
    "O Mosaic não conseguiu usar a área de transferência. Tente de novo ou copie no app Mosaic.",
  "ext.error.cancelled": "Cancelado no app Mosaic. Tente de novo quando estiver pronto.",
  "ext.error.passkeyNotFound": "O Mosaic não tem uma chave de acesso para este site.",
  "ext.error.passkeyExcluded": "Você já tem uma chave de acesso para esta conta no Mosaic.",
  "ext.error.rpIdMismatch":
    "Este site não pode usar uma chave de acesso para esse domínio, então o Mosaic não respondeu.",
  "ext.error.passkeyUnsupported":
    "O Mosaic não consegue criar uma chave de acesso com as opções que este site pediu.",
  "ext.error.pairingDenied":
    "A conexão foi recusada no app Mosaic. Escolha Conectar para tentar de novo e depois escolha Permitir.",
  "ext.error.pairingExpired":
    "O pedido de conexão expirou. Escolha Conectar para começar de novo.",
  "ext.error.pairingConflict":
    "Outro pedido de conexão estava aberto. Escolha Conectar para tentar de novo.",
  "ext.error.pairingBlocked":
    "Conexões demais foram recusadas. Aguarde dez minutos e tente de novo.",
  "ext.error.pairingLimit":
    "Este computador tem navegadores conectados demais. Desconecte um nas configurações do Mosaic.",
  "ext.error.internal":
    "O Mosaic encontrou um problema inesperado. Tente de novo e, se repetir, reinicie o app Mosaic.",
  "ext.error.desktopNotRunning":
    "O Mosaic não está em execução neste computador. Abra-o e tente de novo.",
  "ext.error.desktopNotAuthentic":
    "O app que respondeu não é o app Mosaic ao qual este navegador está conectado. Nada foi enviado.",
  "ext.error.timeout":
    "O Mosaic não respondeu a tempo. Se houver uma janela aberta no app Mosaic, responda lá.",
  "ext.error.frameTooLarge": "Eram dados demais para enviar ao Mosaic.",
  "ext.error.hostMissing":
    "O auxiliar do Mosaic não está instalado para este navegador. Instale o app Mosaic e reinicie o navegador.",
  "ext.error.hostForbidden":
    "Este navegador ou extensão não tem permissão para conversar com o Mosaic.",
  "ext.error.hostExited": "A conexão com o Mosaic foi encerrada. Tente de novo.",
  "ext.error.unsupportedPage": "O Mosaic não pode preencher neste tipo de página.",
  "ext.error.noFields":
    "Nenhum campo de login encontrado aqui. Clique em um campo e tente de novo.",
  "ext.error.noMatch": "O Mosaic não tem um login salvo para este site.",
  "ext.error.frameNeedsConfirmation":
    "O campo está dentro de um quadro. Abra o pop-up do Mosaic e confirme o quadro para preenchê-lo.",
  "ext.error.frameOpaque": "O campo está em um quadro isolado (sandbox). O Mosaic não preenche ali.",
  "ext.error.originChanged":
    "A página mudou enquanto o Mosaic preenchia. Nada foi preenchido. Tente de novo.",
  "ext.error.pageHidden": "A página não está visível, então o Mosaic não preencheu.",
  "ext.error.permissionDenied":
    "O Mosaic precisa de acesso aos sites para perceber logins, e você não permitiu.",
  "ext.error.generatedExpired":
    "O Mosaic não guarda mais essa senha. Gere uma nova.",
  "ext.rotation.card.title": "Troca de senha",
  "ext.rotation.card.for": "Para {name}",
  "ext.rotation.card.started": "Iniciada {when}",
  "ext.rotation.card.delivered": "A nova senha já foi preenchida uma vez.",
  "ext.rotation.card.note":
    "A nova senha só é mostrada no app Mosaic. O Mosaic nunca envia o formulário por você.",
  "ext.rotation.card.list": "Trocas de senha em andamento",
  "ext.rotation.change": "Trocar senha…",
  "ext.rotation.changeLabel": "Trocar senha… de {name}",
  "ext.rotation.openPage": "Abrir página de troca",
  "ext.rotation.fillChange": "Preencher formulário de troca",
  "ext.rotation.fillPending": "Preencher a nova senha para entrar",
  "ext.rotation.worked": "Funcionou",
  "ext.rotation.failed": "Não funcionou",
  "ext.rotation.fresh": "Recomeçar com uma nova senha",
  "ext.rotation.waiting": "Aguardando você no app Mosaic…",
  "ext.rotation.started": "Iniciada. Escolha Usar esta senha no app Mosaic.",
  "ext.rotation.startCancelled": "Nada foi iniciado.",
  "ext.rotation.filled": "Formulário de troca preenchido. Confira e envie você mesmo.",
  "ext.rotation.opened": "Página de troca aberta.",
  "ext.rotation.confirmed": "Pronto. A nova senha agora está salva no Mosaic.",
  "ext.rotation.keptOld": "O Mosaic manteve a senha antiga.",
  "ext.rotation.resultCancelled": "Nada mudou. A troca ainda está aberta.",
  "ext.rotation.offer.title": "Um endereço relacionado",
  "ext.rotation.offer.body":
    "Esta página está em {host}, que não está salvo para {title}. Adicioná-lo para o Mosaic poder preencher ali?",
  "ext.rotation.offer.add": "Adicionar {host}",
  "ext.rotation.offer.added": "Adicionado. A troca agora aparece aqui.",
  "ext.rotation.offer.cancelled": "Não adicionado.",
  "ext.error.rotationNotFound":
    "Essa troca de senha não está mais aberta. Inicie uma nova a partir do login.",
  "ext.error.originNotRelated":
    "Esse endereço não está no mesmo site deste login, então o Mosaic não o adicionou.",
  "ext.error.rotationLimit":
    "Este login já tem quatro trocas não concluídas. Conclua ou cancele uma delas antes no app Mosaic.",
  "ext.error.changeFormNotFound":
    "O Mosaic não encontrou um formulário de troca de senha aqui. Abra a página de senha da sua conta e tente de novo.",
  "ext.error.changeFormAmbiguous":
    "O Mosaic não conseguiu distinguir qual campo é qual. Copie a nova senha do Mosaic e cole você mesmo.",
} satisfies Record<keyof typeof en, string>;
