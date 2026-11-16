/**
 * Brazilian Portuguese UI strings: sharing. Same keys as English (checked by the type).
 * ICU plural categories of Brazilian Portuguese: one, many, other (CLDR; `other` is required).
 */
import type { sharing as en } from "../en/sharing.ts";

export const sharing = {
  "sharing.title": "Compartilhamento",
  "sharing.body":
    "Compartilhe cofres com pessoas que você conhece. Tudo continua criptografado: só as pessoas que você convida podem ler um cofre compartilhado, e o Mosaic só compartilha com pessoas cujo código de segurança você comparou.",
  "sharing.notHosted.none":
    "O compartilhamento exige um servidor Mosaic. Ative antes a sincronização com um servidor Mosaic nas configurações de Sincronização.",
  "sharing.notHosted.folder":
    "O compartilhamento não está disponível com uma pasta de sincronização. Uma pasta não consegue saber quem pode ler um cofre, então o Mosaic mantém os cofres em uma pasta privados para você. Para compartilhar, desconecte a pasta e conecte um servidor Mosaic nas configurações de Sincronização.",
  "sharing.notHosted":
    "O compartilhamento exige um servidor Mosaic. Conecte um antes nas configurações de Sincronização.",
  "sharing.syncSettings": "Abrir configurações de sincronização",
  "sharing.ownCode.title": "Seu código de segurança",
  "sharing.ownCode.body":
    "Leia este código para a pessoa com quem você compartilha, pessoalmente ou por telefone. Se ele for igual ao que ela vê para você, ninguém está no meio.",
  "sharing.ownCode.pending":
    "Seu código de segurança aparece após a primeira sincronização com o servidor.",
  "sharing.contacts.title": "Pessoas que você procurou",
  "sharing.contacts.empty": "Você ainda não procurou ninguém.",
  "sharing.contacts.verified": "Verificado",
  "sharing.contacts.notVerified": "Não verificado",
  "sharing.contacts.changed": "Precisa de nova verificação",
  "sharing.contacts.verify": "Comparar código…",
  "sharing.contacts.verifyOf": "Comparar código com {email}",
  "sharing.contacts.review": "Revisar alteração…",
  "sharing.contacts.reviewOf": "Revisar alteração de {email}",
  "sharing.share.open": "Compartilhar…",
  "sharing.share.title": "Compartilhar {vault}",
  "sharing.share.dialog": "Compartilhamento: {vault}",
  "sharing.share.roleStep": "Escolha uma função para {email}",
  "sharing.lookup.email": "Endereço de e-mail da pessoa",
  "sharing.lookup.find": "Procurar",
  "sharing.lookup.searching": "Procurando…",
  "sharing.lookup.notFoundHint":
    "O Mosaic não pode dizer se um endereço tem uma conta. Se o código acima não for igual ao que a pessoa vê, não compartilhe.",
  "sharing.code.title": "Comparar códigos de segurança",
  "sharing.code.compare":
    "Compare este código com {email}, pessoalmente ou por telefone. Não use e-mail nem chat.",
  "sharing.code.compareNew":
    "Compare o novo código com {email}, pessoalmente ou por telefone. Não use e-mail nem chat.",
  "sharing.code.shown": "Código que o Mosaic mostra para {email}",
  "sharing.code.typed": "Código que a pessoa leu para você (opcional)",
  "sharing.code.typedMatch": "Este é o código acima.",
  "sharing.code.attest":
    "Comparei este código com {email} por telefone ou pessoalmente, e ele é o mesmo.",
  "sharing.code.matches": "Os códigos conferem",
  "sharing.code.mismatch":
    "Os códigos são diferentes. Não compartilhe. Alguém pode estar entre você e {email}.",
  "sharing.code.mismatchHelp":
    "Confira o endereço e peça a {email} que leia o código de novo. Se ainda for diferente, cancele e avise a pessoa por outro meio.",
  "sharing.code.verified": "{email} está verificado.",
  "sharing.code.status.new": "Novo contato",
  "sharing.code.status.pinned": "Ainda não verificado",
  "sharing.code.status.verified": "Verificado",
  "sharing.code.status.changed": "Chave alterada",
  "sharing.code.status.conflicting": "Os dispositivos divergem",
  "sharing.code.conflicting":
    "Seus dispositivos viram chaves diferentes para {email}. Compare o código com a pessoa de novo antes de compartilhar.",
  "sharing.verify.title": "Verificar {email}",
  "sharing.role.label": "Função",
  "sharing.role.owner": "Proprietário",
  "sharing.role.manage": "Gerente",
  "sharing.role.write": "Editor",
  "sharing.role.read": "Leitor",
  "sharing.role.desc.manage": "Pode convidar e remover pessoas e alterar funções.",
  "sharing.role.desc.write": "Pode adicionar e alterar itens.",
  "sharing.role.desc.read": "Só pode ler.",
  "sharing.role.honesty":
    "Quem pode ler um item pode copiá-lo. Um leitor não pode alterar o cofre, mas nada o impede de guardar o que vê, e remover alguém depois não desfaz o que a pessoa já baixou.",
  "sharing.invite.send": "Enviar convite",
  "sharing.invite.sent":
    "Convite enviado para {email}. A pessoa precisa abrir o Mosaic e aceitá-lo.",
  "sharing.invite.sentTitle": "Convite enviado",
  "sharing.invite.pending": "Aguardando {email}",
  "sharing.invite.expires": "Expira {when}",
  "sharing.invite.cancel": "Retirar convite",
  "sharing.invite.cancelOf": "Retirar convite de {email}",
  "sharing.invite.cancelled": "Convite retirado.",
  "sharing.inbox.title": "Convites",
  "sharing.inbox.nav": "Convites",
  "sharing.inbox.navCount": "Convites, {count} em espera",
  "sharing.inbox.intro":
    "Pessoas que convidaram você para um cofre compartilhado. O Mosaic verifica cada convite antes de mostrá-lo a você.",
  "sharing.inbox.empty":
    "Nenhum convite no momento. Quando alguém convidar você para um cofre compartilhado, ele aparecerá aqui.",
  "sharing.inbox.from": "De {email}",
  "sharing.inbox.review": "Revisar",
  "sharing.inbox.reviewOf": "Revisar convite de {email}",
  "sharing.inbox.refresh": "Atualizar",
  "sharing.inbox.preview.title": "Entrar em {vault}?",
  "sharing.inbox.preview.body": "{email} convida você como {role}.",
  "sharing.inbox.preview.unnamed": "um cofre compartilhado",
  "sharing.inbox.preview.owner": "O cofre pertence a {email}.",
  "sharing.inbox.verifySender": "Antes de entrar, compare o código de segurança de {email}.",
  "sharing.inbox.verifyOwner":
    "{sender} não é o proprietário. Antes de entrar, compare também o código de segurança do proprietário: é o proprietário quem pode alterar a chave do cofre.",
  "sharing.inbox.ownerCode": "Código do proprietário, {email}",
  "sharing.inbox.accept": "Entrar no cofre",
  "sharing.inbox.readyTitle": "Tudo verificado. Entrar neste cofre?",
  "sharing.inbox.decline": "Recusar",
  "sharing.inbox.block": "Recusar e bloquear remetente",
  "sharing.inbox.block.title": "Recusar e bloquear este remetente?",
  "sharing.inbox.block.body":
    "O Mosaic recusa o convite e passa a ignorar todos os convites de {email}. Isto não pode ser desfeito no Mosaic.",
  "sharing.inbox.unreadable":
    "Este convite não pode ser aberto. Ele pode estar danificado ou não ser para você. Peça a {email} que o envie de novo.",
  "sharing.inbox.joined":
    "Você entrou no cofre. Ele aparece na sua lista de cofres em instantes.",
  "sharing.inbox.declined": "Convite recusado.",
  "sharing.inbox.outgoing": "Convites que você enviou",
  "sharing.inbox.outgoing.empty": "Nenhum dos seus convites está aguardando resposta.",
  "sharing.inbox.unknownVault": "um cofre",
  "sharing.members.title": "Pessoas com acesso",
  "sharing.members.you": "Você",
  "sharing.members.unverified": "Não verificado",
  "sharing.members.unconfirmed": "Não está na lista de membros",
  "sharing.members.unconfirmed.help":
    "O servidor lista esta pessoa, mas a lista de membros do próprio cofre não. A próxima troca de chave a deixa de fora. Se você não a conhece, remova-a.",
  "sharing.members.dropped": "Não está mais no servidor",
  "sharing.members.dropped.help":
    "Esta pessoa está na lista de membros do cofre, mas o servidor não a lista mais. Ela pode ter saído. Confira se isso era esperado.",
  "sharing.members.invited": "Convidado",
  "sharing.members.removing": "Sendo removido",
  "sharing.members.verified": "Verificado",
  "sharing.members.verify": "Verificar…",
  "sharing.members.verifyOf": "Verificar {email}",
  "sharing.members.remove": "Remover",
  "sharing.members.removeOf": "Remover {email}",
  "sharing.members.remove.title": "Remover {email}?",
  "sharing.members.remove.confirm":
    "{email} mantém tudo que já baixou. O Mosaic troca a chave do cofre agora, e a pessoa não consegue ler nada novo.",
  "sharing.members.remove.confirmManager":
    "{email} mantém tudo que já baixou. O servidor deixa de enviar atualizações a ela agora. O dispositivo do proprietário do cofre troca a chave do cofre na próxima sincronização, e até lá só o servidor mantém {email} de fora.",
  "sharing.members.cancel": "Cancelar",
  "sharing.members.removed": "Removido.",
  "sharing.members.leave": "Sair do cofre",
  "sharing.members.leave.title": "Sair deste cofre?",
  "sharing.members.leave.confirm":
    "A cópia neste dispositivo continua legível, mas deixa de ser atualizada. Para entrar de novo, você precisa de um novo convite.",
  "sharing.members.left": "Você saiu do cofre.",
  "sharing.members.roleOf": "Função de {email}",
  "sharing.members.changeRole": "Alterar função",
  "sharing.members.changeRoleOf": "Alterar função de {email}",
  "sharing.members.invite": "Convidar alguém…",
  "sharing.members.empty": "Até agora, só você pode abrir este cofre.",
  "sharing.members.unknownEmail": "Endereço desconhecido",
  "sharing.members.managerNote":
    "Você pode convidar e remover pessoas. Só o dispositivo do proprietário troca a chave do cofre, então uma remoção só é completa depois que o dispositivo do proprietário sincronizar.",
  "sharing.members.readerNote":
    "Você pode ler este cofre, mas não alterar quem tem acesso.",
  "sharing.rotation.running":
    "Uma remoção está aguardando o dispositivo do proprietário trocar a chave do cofre. Até ele sincronizar, só o servidor mantém a pessoa removida de fora.",
  "sharing.rotation.unprotected":
    "{count, plural, one {# arquivo mantém} other {# arquivos mantêm}} a proteção antiga porque este dispositivo não {count, plural, one {o tem} other {os tem}}. Uma pessoa que você removeu ainda pode abri-{count, plural, one {lo} other {los}} se guardou a chave antiga. Adicione-{count, plural, one {o} other {os}} de novo a partir de um dispositivo que {count, plural, one {o tenha} other {os tenha}}.",
  "sharing.keyChanged.title": "O código de segurança de {email} mudou",
  "sharing.keyChanged.body":
    "O servidor agora mostra uma chave diferente para {email}. Isso acontece quando a pessoa configura o Mosaic de novo, e também é como um ataque ao servidor se pareceria. Nada é compartilhado com a nova chave até você comparar os códigos com a pessoa, pessoalmente ou por telefone.",
  "sharing.keyChanged.old": "Código que você comparou antes",
  "sharing.keyChanged.new": "Novo código",
  "sharing.keyChanged.accept": "O novo código confere",
  "sharing.finding.region": "Avisos de compartilhamento",
  "sharing.finding.summary":
    "{count, plural, one {# aviso de compartilhamento} other {# avisos de compartilhamento}}",
  "sharing.finding.review": "Revisar",
  "sharing.finding.collapse": "Recolher",
  "sharing.finding.dismiss": "Dispensar",
  "sharing.finding.unknownEmail": "uma pessoa desconhecida",
  "sharing.finding.verify": "Verificar…",
  "sharing.finding.remove": "Remover",
  "sharing.finding.removeOf": "Remover {email} de {vault}",
  "sharing.finding.identityMismatch":
    "O servidor mostra uma chave de compartilhamento diferente da que este dispositivo criou para a sua conta, então o compartilhamento está pausado. Não compartilhe nada e verifique o seu servidor e os seus outros dispositivos. Depois escolha “Já verifiquei…”.",
  "sharing.finding.rosterInvalid":
    "A lista de membros de {vault} está danificada ou alguém a alterou sem permissão. Não adicione pessoas até verificar quem tem acesso. Depois escolha “Já verifiquei…”.",
  "sharing.finding.shareUntrusted":
    "Uma nova chave de {vault} foi recusada: {sender} a enviou, mas não tem permissão para alterar a chave deste cofre. O cofre continua com a chave antiga. Pergunte ao proprietário se ele a alterou. Depois escolha “Já verifiquei…”.",
  "sharing.finding.memberUnverified":
    "Você não verificou {email} em {vault}. O Mosaic não envia novas chaves a essa pessoa, e a próxima troca de chave a deixa de fora. Compare o código de segurança dela ou remova-a.",
  "sharing.finding.memberDropped":
    "{email} está na lista de membros de {vault}, mas o servidor não a lista mais. Ela pode ter saído, ou o servidor a removeu. Confira se isso era esperado e remova-a se não era.",
  "sharing.finding.memberKeyMismatch":
    "O Mosaic recebeu uma chave para {email} que não corresponde à que você verificou, então a pessoa foi removida de {vault}. Compare os códigos de segurança com ela de novo antes de convidá-la outra vez.",
  "sharing.finding.vaultHeld":
    "{vault} está aguardando até os seus outros dispositivos confirmarem quem é o proprietário. Abra o Mosaic em um dispositivo que já usa este cofre.",
  "sharing.finding.accessLost":
    "Você não tem mais acesso a {vault}. A cópia neste dispositivo continua legível, mas não é atualizada. Peça ao proprietário que convide você de novo.",
  "sharing.finding.rotationBlocked":
    "Não foi possível trocar a chave de {vault} depois que você removeu alguém, então essa pessoa ainda tem a chave antiga. Sincronize de novo. Se continuar assim, abra a lista de membros e verifique quem tem acesso.",
  "emergency.title": "Acesso de emergência",
  "emergency.body":
    "Indique pessoas que podem pedir acesso aos seus cofres se algo acontecer com você. Isto não é recuperação de conta: elas recebem acesso somente de leitura aos cofres que você escolher, apenas após um tempo de espera, e você pode recusar durante esse tempo. Não substitui o seu Kit de Emergência.",
  "emergency.folderUnsupported":
    "O acesso de emergência exige o servidor Mosaic. Uma pasta de sincronização não consegue impor um tempo de espera.",
  "emergency.granted.title": "Pessoas que podem pedir acesso",
  "emergency.granted.empty": "Ninguém pode pedir acesso aos seus cofres.",
  "emergency.granted.add": "Adicionar uma pessoa…",
  "emergency.granted.scopeAll": "Todos os seus cofres",
  "emergency.granted.scopeSome": "{count, plural, one {# cofre} other {# cofres}}",
  "emergency.granted.wait": "Espera {days, plural, one {# dia} other {# dias}}",
  "emergency.granted.waitingUntil":
    "Pediu acesso. Ele é aberto {when}, a menos que você recuse.",
  "emergency.granted.approved": "Você aprovou o pedido.",
  "emergency.granted.released": "O acesso foi liberado. A pessoa pode ler os cofres.",
  "emergency.granted.idle": "Nenhum pedido",
  "emergency.granted.remove": "Remover acesso",
  "emergency.granted.removeOf": "Remover acesso de {email}",
  "emergency.granted.remove.title": "Remover o acesso de emergência de {email}?",
  "emergency.granted.remove.body":
    "A pessoa não pode mais pedir. Se o acesso dela já foi liberado, o Mosaic troca as chaves dos cofres que ela abriu; ela mantém o que já baixou.",
  "emergency.granted.removed": "Acesso de emergência removido.",
  "emergency.grant.title": "Permitir acesso de emergência",
  "emergency.grant.lookup": "Quem deve poder pedir?",
  "emergency.grant.optionsTitle": "O que {email} pode abrir",
  "emergency.grant.scope": "O que a pessoa pode abrir",
  "emergency.grant.scope.all": "Todos os meus cofres, inclusive os novos",
  "emergency.grant.scope.selected": "Somente os cofres que eu escolher",
  "emergency.grant.scope.pick": "Escolha pelo menos um cofre.",
  "emergency.grant.wait": "Tempo de espera",
  "emergency.grant.wait.option": "{days, plural, one {# dia} other {# dias}}",
  "emergency.grant.waitHint":
    "Abra o Mosaic pelo menos uma vez dentro deste tempo após um pedido, para poder vê-lo e recusar. Quando o tempo acabar, a pessoa recebe acesso somente de leitura, sem que ninguém pergunte a você de novo.",
  "emergency.grant.serverTrust":
    "O servidor controla o tempo de espera. Se a pessoa indicada e quem administra o servidor agirem em conjunto, poderiam pulá-lo. O servidor sozinho não consegue ler os seus cofres.",
  "emergency.grant.vaultKeysOnly":
    "Isto abrange apenas as chaves dos seus cofres. Não dá acesso à sua conta, à sua senha mestra nem à sua Chave Secreta.",
  "emergency.grant.submit": "Permitir acesso de emergência",
  "emergency.grant.done": "{email} agora pode pedir acesso de emergência.",
  "emergency.grant.needVerify": "Compare antes o código de segurança de {email}.",
  "emergency.received.title": "Pessoas que indicaram você",
  "emergency.received.empty": "Ninguém indicou você para acesso de emergência.",
  "emergency.received.request": "Pedir acesso",
  "emergency.received.requestOf": "Pedir acesso a {email}",
  "emergency.received.request.title": "Pedir acesso a {email}?",
  "emergency.received.request.body":
    "{email} vê o seu pedido na próxima vez que abrir o Mosaic. Após {days, plural, one {# dia} other {# dias}} você pode abrir os cofres dela, somente leitura, a menos que ela recuse.",
  "emergency.received.requested":
    "Pedido enviado. {email} o vê na próxima vez que abrir o Mosaic.",
  "emergency.received.requestHint":
    "{email} vê o seu pedido no Mosaic. Após {days, plural, one {# dia} other {# dias}} você pode abrir os cofres dela, somente leitura, a menos que ela recuse.",
  "emergency.received.countdown": "Você pode abrir os cofres dela {when}.",
  "emergency.received.waitingLive": "Seu pedido está em espera.",
  "emergency.received.claim": "Abrir os cofres dela",
  "emergency.received.claimOf": "Abrir os cofres de {email}",
  "emergency.received.claimed":
    "{count, plural, one {# cofre foi adicionado} other {# cofres foram adicionados}}, somente leitura.",
  "emergency.received.cooldown": "{email} recusou. Você pode pedir de novo {when}.",
  "emergency.received.verifyFirst":
    "Compare o código de segurança de {email} antes de abrir os cofres dela. O Mosaic só usa uma chave que você verificou.",
  "emergency.received.verify": "Comparar código…",
  "emergency.received.verifyOf": "Comparar código com {email}",
  "emergency.received.approved": "{email} aprovou o seu pedido.",
  "emergency.received.waitDays": "Espera {days, plural, one {# dia} other {# dias}}",
  "emergency.banner.region": "Pedido de acesso de emergência",
  "emergency.banner.body":
    "{email} pediu acesso de emergência aos seus cofres. Ele é aberto {when}, a menos que você recuse.",
  "emergency.banner.refuse": "Recusar",
  "emergency.banner.approve": "Aprovar agora…",
  "emergency.banner.approve.title": "Aprovar o acesso de emergência agora?",
  "emergency.banner.approve.body":
    "{email} recebe acesso somente de leitura aos seus cofres imediatamente. Digite a sua senha mestra para confirmar.",
  "emergency.banner.approve.confirm": "Aprovar",
  "emergency.banner.refused": "Pedido recusado.",
  "emergency.banner.approved": "Acesso aprovado.",
} satisfies Record<keyof typeof en, string>;
