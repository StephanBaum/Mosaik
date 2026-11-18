/**
 * Brazilian Portuguese UI strings: rotation. Same keys as English (checked by the type).
 * ICU plural categories of Brazilian Portuguese: one, many, other (CLDR; `other` is required).
 */
import type { rotation as en } from "../en/rotation.ts";

export const rotation = {
  "rotation.section.title": "Senha e chaves",
  "rotation.section.body":
    "Altere a sua senha mestra, obtenha uma nova Chave Secreta ou mude a sua conta inteira para novas chaves. Cada alteração mostra o que vai acontecer antes que algo seja tocado.",
  "rotation.action.password": "Alterar senha mestra",
  "rotation.action.password.body":
    "Escolha uma nova senha mestra. Sua Chave Secreta e os seus cofres continuam como estão.",
  "rotation.action.password.button": "Alterar…",
  "rotation.action.secretKey": "Trocar Chave Secreta",
  "rotation.action.secretKey.body":
    "Obtenha uma nova Chave Secreta e um novo Kit de Emergência. Use isto se o seu kit foi perdido ou alguém pode tê-lo visto.",
  "rotation.action.secretKey.button": "Trocar…",
  "rotation.action.profile": "Reforçar proteção",
  "rotation.action.profile.body":
    "Dificulte adivinhar a sua senha mestra usando mais memória a cada tentativa. O desbloqueio leva um pouco mais de tempo.",
  "rotation.action.profile.button": "Reforçar…",
  "rotation.action.rekey": "Renovar chaves da conta",
  "rotation.action.rekey.body":
    "Renove todas as chaves da sua conta: uma nova Chave Secreta, novas chaves de cofre e um novo código de segurança. Use isto se alguém pode ter copiado os dados do seu cofre junto com a sua senha e a Chave Secreta.",
  "rotation.action.rekey.button": "Renovar chaves…",
  "rotation.action.rekey.needsSync":
    "Renovar as chaves atualiza a cópia no seu servidor ou pasta de sincronização, então exige sincronização. Configure a sincronização primeiro.",
  "rotation.title.password": "Alterar senha mestra",
  "rotation.title.secretKey": "Trocar Chave Secreta",
  "rotation.title.profile": "Reforçar proteção",
  "rotation.title.rekey": "Renovar chaves da conta",
  "rotation.step": "Etapa {current} de {total}",
  "rotation.back": "Voltar",
  "rotation.stop.title": "Interromper esta alteração?",
  "rotation.stop.body":
    "O Kit de Emergência que você acabou de salvar não funcionará. Guarde o seu kit antigo.",
  "rotation.stop.confirm": "Interromper alteração",
  "rotation.stop.keep": "Continuar",
  "rotation.next": "Continuar",
  "rotation.close": "Fechar",
  "rotation.review.title.password": "Antes de alterar a sua senha",
  "rotation.review.title.secretKey": "Antes de trocar a sua Chave Secreta",
  "rotation.review.title.profile": "Antes de reforçar a proteção",
  "rotation.review.title.rekey": "Algo foi comprometido?",
  "rotation.review.changes": "O que muda",
  "rotation.review.stays": "O que continua igual",
  "rotation.review.password.changes":
    "A sua senha mestra. O desbloqueio rápido neste dispositivo é desativado, e você pode configurá-lo de novo depois.",
  "rotation.review.password.stays":
    "Sua Chave Secreta, seus cofres e itens, seu código de segurança e seus contatos de emergência.",
  "rotation.review.password.devices":
    "Seus outros dispositivos são desconectados da sincronização. Em cada um, digite a nova senha mestra uma vez e a sincronização continua.",
  "rotation.review.secretKey.changes":
    "Sua Chave Secreta e seu Kit de Emergência. O desbloqueio rápido neste dispositivo é desativado, e você pode configurá-lo de novo depois.",
  "rotation.review.secretKey.stays":
    "Sua senha mestra, seus cofres e itens, seu código de segurança e seus contatos de emergência. As chaves de segurança que você registrou continuam funcionando.",
  "rotation.review.secretKey.devices":
    "Seus outros dispositivos são desconectados da sincronização. Em cada um, digite a sua senha e a nova Chave Secreta do novo kit uma vez.",
  "rotation.review.secretKey.limit":
    "Isto impede o login com a Chave Secreta antiga. Quem já copiou os dados do seu cofre com a sua senha e a Chave Secreta antigas ainda pode lê-los. Renove as chaves da conta para impedir isso.",
  "rotation.review.secretKey.compromised": "Algo foi comprometido: renovar as chaves em vez disso",
  "rotation.review.profile.changes":
    "Quanta memória cada tentativa de desbloqueio exige, para você e para quem tentar adivinhar. O desbloqueio rápido neste dispositivo é desativado, e você pode configurá-lo de novo depois.",
  "rotation.review.profile.stays":
    "Sua senha mestra, sua Chave Secreta, seus cofres e seu código de segurança. As chaves de segurança que você registrou continuam funcionando.",
  "rotation.review.profile.devices":
    "Seus outros dispositivos são desconectados da sincronização. Em cada um, digite a sua senha mestra uma vez e a sincronização continua. Dispositivos mais lentos levam um pouco mais para desbloquear.",
  "rotation.review.rekey.intro":
    "Renovar as chaves serve para o caso de alguém poder ter os dados do seu cofre e também a sua senha e a Chave Secreta. Se você apenas perdeu um dispositivo, trocar a Chave Secreta ou alterar a senha é suficiente.",
  "rotation.review.rekey.yes": "Sim, renovar as chaves da minha conta",
  "rotation.review.rekey.no": "Não, apenas trocar a Chave Secreta",
  "rotation.review.rekey.limit":
    "Renovar as chaves não desfaz o que alguém já copiou. Cópias antigas em servidores e em backups continuam criptografadas com as chaves antigas, e as chaves antigas ainda as abrem.",
  "rotation.review.rekey.title": "O que a renovação de chaves faz",
  "rotation.review.rekey.vaults":
    "{count, plural, one {# cofre que só você usa recebe uma nova chave.} other {# cofres que só você usa recebem novas chaves.}}",
  "rotation.review.rekey.keys":
    "Uma nova Chave Secreta, um novo Kit de Emergência, um novo código de segurança e uma nova chave de conta.",
  "rotation.review.rekey.devices":
    "Seus outros dispositivos são desconectados e não podem sincronizar até você digitar a nova Chave Secreta e a sua nova senha mestra em cada um.",
  "rotation.review.rekey.shared":
    "{count, plural, one {Você compartilha # cofre.} other {Você compartilha # cofres.}} Cada um recebe uma nova chave.",
  "rotation.review.rekey.sharedRow": "{name}, {count, plural, one {# membro} other {# membros}}",
  "rotation.review.rekey.sharedMembers":
    "Peça a cada pessoa com quem você compartilha que compare o seu novo código de segurança. Até que façam isso, elas não recebem novas chaves de você e não podem abrir o cofre renovado.",
  "rotation.review.rekey.memberVaults":
    "{count, plural, one {# cofre pertence a outra pessoa.} other {# cofres pertencem a outras pessoas.}} Pede-se aos proprietários que os renovem. Até que façam isso, as suas chaves antigas ainda podem lê-los.",
  "rotation.review.rekey.grants":
    "{count, plural, one {# pessoa tem acesso de emergência aos seus cofres.} other {# pessoas têm acesso de emergência aos seus cofres.}} O acesso delas é renovado para as novas chaves. Cada uma delas também precisa comparar o seu novo código de segurança.",
  "rotation.review.rekey.quick":
    "O desbloqueio rápido neste dispositivo é desativado. Você pode configurá-lo de novo depois.",
  "rotation.review.rekey.hardwareKeys":
    "{count, plural, one {# chave de segurança que você registrou para desbloquear é desativada.} other {# chaves de segurança que você registrou para desbloquear são desativadas.}} Você pode registrá-las de novo depois.",
  "rotation.review.rekey.hardwareVaults":
    "{count, plural, one {# cofre protegido por hardware não é afetado.} other {# cofres protegidos por hardware não são afetados.}}",
  "rotation.review.rekey.attachments":
    "{count, plural, one {# anexo não está neste dispositivo.} other {# anexos não estão neste dispositivo.}} Eles são baixados primeiro para poderem ser protegidos de novo. Os que não puderem ser acessados mantêm a chave antiga.",
  "rotation.review.rekey.folder":
    "Sua pasta de sincronização recebe um novo arquivo de conta. Os outros dispositivos encontram ali um aviso de que as chaves mudaram.",
  "rotation.review.rekey.unavailable":
    "Renovar as chaves não está disponível: a sincronização não está configurada.",
  "rotation.review.loading": "Verificando o que a renovação de chaves afeta…",
  "rotation.review.rekey.needsClean":
    "Renovar as chaves exige que a sincronização esteja em dia e sem avisos. Sincronize agora, veja qualquer aviso e tente de novo.",
  "rotation.credentials.title": "Confirme que é você",
  "rotation.credentials.password.title": "Suas senhas mestras",
  "rotation.credentials.current": "Senha mestra atual",
  "rotation.credentials.new": "Nova senha mestra",
  "rotation.credentials.confirm": "Digite a nova senha mestra de novo",
  "rotation.credentials.mismatch": "As duas senhas são diferentes.",
  "rotation.credentials.same": "A nova senha mestra precisa ser diferente da atual.",
  "rotation.credentials.hint":
    "Uma frase secreta longa é melhor que uma curta e complicada. Você não pode recuperá-la: o Mosaic não a conhece.",
  "rotation.credentials.code": "Código de 6 dígitos do seu app autenticador",
  "rotation.credentials.profile": "Nível de proteção",
  "rotation.credentials.profile.standard": "Padrão: 256 MB de memória por tentativa",
  "rotation.credentials.profile.strong": "Forte: 512 MB de memória por tentativa",
  "rotation.credentials.profile.max": "Máximo: 1 GB de memória por tentativa",
  "rotation.credentials.profile.slow":
    "Desbloquear com este nível leva mais de dois segundos neste dispositivo. Outros dispositivos, mais antigos, serão ainda mais lentos.",
  "rotation.credentials.profile.force": "Reforçar mesmo assim",
  "rotation.kit.title": "Salve o seu novo Kit de Emergência",
  "rotation.kit.body":
    "Sua nova Chave Secreta está neste kit. Salve-o primeiro: o Mosaic só altera a sua conta depois que você confirma que o tem, porque sem ele você não consegue entrar em um novo dispositivo.",
  "rotation.kit.save": "Salvar novo Kit de Emergência",
  "rotation.kit.saved": "Salvo. Guarde-o em um lugar seguro, longe dos seus dispositivos.",
  "rotation.kit.cancelled": "Nada foi salvo ainda.",
  "rotation.kit.verify": "Digite o último grupo da sua nova Chave Secreta a partir do kit salvo.",
  "rotation.kit.confirm": "Confirmar kit",
  "rotation.kit.mistyped": "Esse não é o final da nova Chave Secreta. Confira o kit salvo.",
  "rotation.kit.verified": "Kit confirmado.",
  "rotation.confirm.title.password": "Altere a sua senha mestra",
  "rotation.confirm.title.secretKey": "Troque a sua Chave Secreta",
  "rotation.confirm.title.profile": "Reforce a proteção",
  "rotation.confirm.title.rekey": "Renove as chaves da sua conta",
  "rotation.confirm.body.rekey":
    "Isto desconecta os seus outros dispositivos e renova todas as chaves. Mantenha o Mosaic aberto até terminar.",
  "rotation.confirm.body.secretKey":
    "Isto desconecta os seus outros dispositivos da sincronização. Mantenha o Mosaic aberto até terminar.",
  "rotation.confirm.verb.password": "Alterar senha",
  "rotation.confirm.verb.secretKey": "Trocar Chave Secreta",
  "rotation.confirm.verb.profile": "Reforçar proteção",
  "rotation.confirm.verb.rekey": "Renovar chaves da conta",
  "rotation.run.title": "Renovando as suas chaves…",
  "rotation.run.keepOpen":
    "Mantenha o Mosaic aberto. Com muitos anexos, isto pode levar alguns minutos.",
  "rotation.progress.label": "Progresso",
  "rotation.progress.count": "{done} de {total}",
  "rotation.progress.done": "concluído",
  "rotation.phase.prepared": "Preparando",
  "rotation.phase.backendStarted": "Pedindo à sua conta que troque para as novas chaves",
  "rotation.phase.backendAccepted": "Sua conta aceitou as novas chaves",
  "rotation.phase.folderWritten": "A pasta de sincronização tem o novo arquivo de conta",
  "rotation.phase.localCommitted": "Este dispositivo agora usa as novas chaves",
  "rotation.phase.attachments": "Protegendo os anexos de novo",
  "rotation.phase.uploading": "Enviando os dados protegidos",
  "rotation.phase.done": "Concluído",
  "rotation.done.title.password": "Senha mestra alterada",
  "rotation.done.title.secretKey": "Chave Secreta trocada",
  "rotation.done.title.profile": "Proteção reforçada",
  "rotation.done.title.rekey": "Chaves da conta renovadas",
  "rotation.done.body.password":
    "Use a nova senha mestra daqui em diante. Seus outros dispositivos a pedirão uma vez.",
  "rotation.done.body.secretKey":
    "Guarde o novo Kit de Emergência em segurança e exclua o antigo. Seus outros dispositivos pedirão a nova Chave Secreta uma vez.",
  "rotation.done.body.profile":
    "O desbloqueio agora exige mais memória. Seus outros dispositivos pedirão a sua senha uma vez.",
  "rotation.done.body.rekey":
    "Guarde o novo Kit de Emergência em segurança e exclua o antigo. Seus outros dispositivos pedirão a nova Chave Secreta e a sua nova senha mestra uma vez.",
  "rotation.done.members":
    "Peça a todos com quem você compartilha um cofre que comparem o seu novo código de segurança. Até que façam isso, eles não recebem novas chaves de você.",
  "rotation.done.quick": "Configure o desbloqueio rápido de novo",
  "rotation.done.quick.body": "O desbloqueio rápido foi desativado.",
  "rotation.done.hardware": "Registre as chaves de segurança de novo",
  "rotation.done.hardware.body":
    "{count, plural, one {# chave de segurança foi desativada para desbloqueio.} other {# chaves de segurança foram desativadas para desbloqueio.}}",
  "rotation.done.unreachable.note":
    "Estes arquivos não puderam ser lidos e não foram criptografados de novo. Eles mantêm a chave de arquivo antiga até você removê-los ou adicioná-los de novo, então quem tiver as suas chaves antigas e uma cópia de um arquivo ainda poderia abri-lo.",
  "rotation.done.unreachable.file": "{item}: {file}",
  "rotation.done.unreachable.unknownItem": "Item desconhecido",
  "rotation.done.unreachable.unknownFile": "arquivo desconhecido",
  "rotation.done.unreachable.more":
    "{count, plural, one {e mais # arquivo} other {e mais # arquivos}}",
  "rotation.done.unfinished":
    "A última parte continua depois. Se o Mosaic for fechado antes de terminar, escolha Continuar troca de chaves na próxima vez.",
  "rotation.resume.title": "Troca de chaves não concluída",
  "rotation.resume.body.password":
    "Uma troca de chaves foi interrompida. Nada se perdeu. Escolha Continuar para digitar as suas senhas de novo, ou descarte-a.",
  "rotation.resume.body.secretKey":
    "A troca da sua Chave Secreta foi interrompida. Nada se perdeu. Continue para concluí-la.",
  "rotation.resume.body.rekey":
    "A renovação das chaves da sua conta foi interrompida. Nada se perdeu. Continue para concluí-la.",
  "rotation.resume.body.profile":
    "O reforço da proteção foi interrompido. Nada se perdeu. Continue para concluí-lo.",
  "rotation.resume.continue": "Continuar troca de chaves",
  "rotation.resume.discard": "Descartar…",
  "rotation.resume.running": "Continuando…",
  "rotation.resume.progress":
    "{done} de {total} concluídos. Mantenha o Mosaic aberto até terminar.",
  "rotation.resume.dialog.title": "Continuar troca de chaves",
  "rotation.resume.dialog.password": "Senha mestra que você tem agora",
  "rotation.resume.dialog.group": "Último grupo da nova Chave Secreta",
  "rotation.resume.dialog.body":
    "As chaves da sua conta podem já ter mudado. Para ter certeza de que foi você, digite a sua senha mestra e o último grupo da nova Chave Secreta do novo Kit de Emergência.",
  "rotation.resume.dialog.submit": "Continuar",
  "rotation.resume.discard.title": "Descartar esta troca de chaves?",
  "rotation.resume.discard.body":
    "A nova Chave Secreta e o novo Kit de Emergência desta tentativa são excluídos. Sua conta continua como está.",
  "rotation.resume.discard.confirm": "Descartar troca de chaves",
  "rotation.resume.abandon": "Desistir do restante…",
  "rotation.resume.stalled":
    "A continuação não terminou. Se continuar falhando, você pode parar aqui.",
  "rotation.abandon.title": "Parar de criptografar os seus arquivos de novo?",
  "rotation.abandon.body":
    "Suas novas chaves já estão em uso. O Mosaic para de criptografar de novo os anexos que ainda estão esperando. Eles mantêm a chave de arquivo antiga, então quem tiver as suas chaves antigas e uma cópia de um arquivo ainda poderia abri-lo.",
  "rotation.abandon.stays":
    "Tudo que já foi criptografado de novo continua como está. Remova ou adicione de novo os arquivos em espera depois para protegê-los com a nova chave.",
  "rotation.abandon.confirm": "Parar de criptografar",
  "rotation.abandon.keep": "Continuar tentando",
  "rotation.marker.title": "Aviso de renovação de chaves não confirmado",
  "rotation.marker.hint":
    "Se você não renovou as suas chaves em outro dispositivo, este aviso pode não ser genuíno. Você pode ignorá-lo.",
  "rotation.marker.open": "Ignorar este aviso…",
  "rotation.marker.open.label": "Ignorar este aviso: {name}",
  "rotation.marker.confirm":
    "O Mosaic encontrou um aviso de renovação de chaves que não conseguiu confirmar. Se você não renovou as chaves da sua conta em outro dispositivo, pode ignorá-lo.",
  "rotation.marker.ignore": "Ignorar aviso",
  "rotation.marker.keep": "Manter aviso",
  "rotation.ignored.title": "Avisos de renovação de chaves ignorados",
  "rotation.ignored.body":
    "Avisos que você escolheu ignorar. Restaure um se você de fato renovou as chaves da sua conta em outro dispositivo.",
  "rotation.ignored.row": "Aviso de renovação de chaves da sua pasta de sincronização",
  "rotation.ignored.restore": "Restaurar",
  "rotation.ignored.restore.label": "Restaurar o aviso {name}",
  "rotation.resume.done": "Troca de chaves concluída.",
  "rotation.recovery.title": "As chaves da sua conta foram renovadas em outro dispositivo",
  "rotation.recovery.body":
    "Um dispositivo seu renovou as chaves desta conta. Para continuar sincronizando, digite a sua senha mestra e a Chave Secreta do novo Kit de Emergência.",
  "rotation.recovery.password": "Senha mestra (a nova, se você a alterou lá)",
  "rotation.recovery.secretKey": "Chave Secreta do novo Kit de Emergência",
  "rotation.recovery.submit": "Usar as novas chaves",
  "rotation.recovery.warning":
    "Não renovou as chaves você mesmo? Então alguém com a sua senha e Chave Secreta antigas pode ter renovado. Não digite nada aqui; verifique antes os seus outros dispositivos.",
  "rotation.recovery.notice": "A sincronização está pausada até você fazer isto neste dispositivo.",
  "rotation.recovery.open": "Usar as novas chaves…",
  "rotation.recovery.keyHint":
    "Se as suas chaves foram renovadas em outro dispositivo, digite também a nova Chave Secreta do Kit de Emergência dele.",
  "rotation.recovery.done": "Este dispositivo agora usa as novas chaves.",
  "rotation.recovery.optional":
    "Chave Secreta do Kit de Emergência mais recente (somente se você a renovou)",
  "rotation.file.unknown":
    "Um arquivo de conta desconhecido apareceu na sua pasta de sincronização. Ele não pode ser verificado com as suas chaves: pode ser um resto antigo ou alguém alterou a sua pasta. Seus dados não são afetados.",
  "rotation.file.dismiss": "Dispensar",
  "rotation.file.conflict.title": "Dois dispositivos renovaram as chaves ao mesmo tempo",
  "rotation.file.conflict":
    "Outro dispositivo renovou as chaves da sua conta no mesmo momento que este, então os dois não concordam mais. Renove as chaves de novo a partir deste dispositivo para uni-los: as chaves deste dispositivo prevalecem e o outro pedirá a nova Chave Secreta.",
  "rotation.file.conflict.rekey": "Renovar as chaves de novo a partir deste dispositivo",
  "rotation.file.conflict.keep": "Ocultar aviso",
  "rotation.resume.body.vault":
    "A nova criptografia dos anexos de um cofre foi interrompida. Nada se perdeu. Continue para concluí-la.",
} satisfies Record<keyof typeof en, string>;
