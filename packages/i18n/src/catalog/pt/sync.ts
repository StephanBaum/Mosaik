/**
 * Brazilian Portuguese UI strings: sync. Same keys as English (checked by the type).
 * ICU plural categories of Brazilian Portuguese: one, many, other (CLDR; `other` is required).
 */
import type { sync as en } from "../en/sync.ts";

export const sync = {
  "sync.status.signedOut": "Desconectado",
  "sync.platform.windows": "Windows",
  "sync.platform.macos": "macOS",
  "sync.platform.linux": "Linux",
  "settings.sync.title": "Sincronização",
  "settings.sync.body":
    "Mantenha os seus cofres atualizados em todos os seus dispositivos. Tudo é criptografado neste dispositivo antes de sair dele.",
  "settings.sync.notSetUp":
    "A sincronização está desativada. Seus cofres ficam armazenados apenas neste dispositivo.",
  "settings.sync.deviceName": "Nome deste dispositivo",
  "settings.sync.deviceName.default": "Mosaic no {platform}",
  "settings.sync.password": "Senha mestra",
  "settings.sync.code": "Código de 6 dígitos",
  "settings.sync.server.title": "Servidor Mosaic",
  "settings.sync.server.body":
    "Sincronize por um servidor Mosaic administrado por você ou por alguém em quem você confia.",
  "settings.sync.server.url": "Endereço do servidor",
  "settings.sync.server.urlHint": "Por exemplo, https://sync.example.com",
  "settings.sync.server.check": "Verificar servidor",
  "settings.sync.server.checking": "Verificando o servidor…",
  "settings.sync.server.registration.open":
    "Este servidor permite que qualquer pessoa crie uma conta de sincronização.",
  "settings.sync.server.registration.invite":
    "Este servidor exige um código de convite para criar uma conta de sincronização.",
  "settings.sync.server.registration.closed":
    "Este servidor não aceita novas contas de sincronização. Você ainda pode entrar em uma conta existente.",
  "settings.sync.server.choice.label": "Conta de sincronização",
  "settings.sync.server.choice.signIn": "Tenho uma conta de sincronização",
  "settings.sync.server.choice.create": "Criar uma",
  "settings.sync.server.invite": "Código de convite",
  "settings.sync.server.create": "Criar conta de sincronização",
  "settings.sync.server.signIn": "Entrar",
  "settings.sync.server.signInAgain": "Entrar de novo",
  "settings.sync.server.working": "Conectando…",
  "settings.sync.server.sees":
    "O servidor vê apenas dados criptografados, o e-mail da sua conta e quando você sincroniza. Ele nunca vê a sua senha mestra nem a sua Chave Secreta.",
  "settings.sync.twoFactor.title": "Digite o seu código",
  "settings.sync.twoFactor.body":
    "Esta conta de sincronização usa verificação em duas etapas. Digite o código de 6 dígitos do seu app autenticador.",
  "settings.sync.twoFactor.submit": "Verificar",
  "settings.sync.folder.title": "Sua própria pasta na nuvem",
  "settings.sync.folder.body":
    "Sincronize por uma pasta que o seu serviço de nuvem (Dropbox, OneDrive, iCloud Drive e outros) mantém atualizada.",
  "settings.sync.folder.choose": "Escolher pasta…",
  "settings.sync.folder.chooseOther": "Escolher outra pasta…",
  "settings.sync.folder.path": "Pasta",
  "settings.sync.folder.contents.empty": "A pasta está vazia. O Mosaic pode usá-la.",
  "settings.sync.folder.contents.mosaic": "Esta pasta já contém dados de sincronização do Mosaic.",
  "settings.sync.folder.contents.other":
    "Esta pasta contém outros arquivos. O Mosaic não grava nada ali. Escolha uma pasta vazia ou crie uma nova.",
  "settings.sync.folder.connect": "Conectar pasta",
  "settings.sync.folder.working": "Conectando…",
  "settings.sync.folder.sees":
    "Seu provedor de nuvem e qualquer pessoa com acesso à pasta veem apenas dados criptografados, o ID da conta e quando os arquivos mudam. Eles nunca veem a sua senha mestra nem a sua Chave Secreta.",
  "settings.sync.status.label": "Status",
  "settings.sync.lastSynced": "Última sincronização {when}",
  "settings.sync.neverSynced": "Ainda não sincronizado",
  "settings.sync.offline":
    "Você está offline. Suas alterações estão salvas neste dispositivo e serão sincronizadas quando você voltar a ficar online.",
  "settings.sync.now": "Sincronizar agora",
  "settings.sync.backend.server": "Servidor",
  "settings.sync.backend.account": "Conta de sincronização",
  "settings.sync.backend.folder": "Pasta",
  "settings.sync.backend.device": "Este dispositivo",
  "settings.sync.signedOut.title": "Este dispositivo foi desconectado",
  "settings.sync.signedOut.body":
    "O servidor encerrou a sessão deste dispositivo. Entre de novo para continuar sincronizando. Suas alterações estão seguras neste dispositivo.",
  "settings.sync.recover.title": "Sua senha mestra foi alterada",
  "settings.sync.recover.body":
    "O servidor ou a pasta tem uma senha mestra mais nova que a deste dispositivo. Digite a nova senha mestra para atualizar este dispositivo. Seus cofres e a sua Chave Secreta continuam os mesmos.",
  "settings.sync.recover.password": "Nova senha mestra",
  "settings.sync.recover.submit": "Atualizar este dispositivo",
  "settings.sync.disconnect": "Desconectar…",
  "settings.sync.disconnect.title": "Desconectar a sincronização?",
  "settings.sync.disconnect.body":
    "Este dispositivo para de sincronizar e sai do servidor. Seus cofres continuam neste dispositivo, e os seus outros dispositivos mantêm as próprias cópias.",
  "settings.sync.disconnect.confirm": "Desconectar",
  "settings.sync.devices.title": "Dispositivos",
  "settings.sync.devices.thisDevice": "Este dispositivo",
  "settings.sync.devices.unnamed": "Dispositivo sem nome",
  "settings.sync.devices.lastSeen": "Visto pela última vez {when}",
  "settings.sync.devices.neverSeen": "Ainda não visto",
  "settings.sync.devices.rename": "Renomear",
  "settings.sync.devices.renameNamed": "Renomear {name}",
  "settings.sync.devices.renameLabel": "Novo nome para este dispositivo",
  "settings.sync.devices.remove": "Remover",
  "settings.sync.devices.removeNamed": "Remover {name}",
  "settings.sync.devices.removeTitle": "Remover {name}?",
  "settings.sync.devices.removeBody":
    "Remover um dispositivo o desconecta da sincronização. Isso não tira a cópia dos seus cofres que ele já tem. Para bloquear um dispositivo de vez, altere a sua senha mestra e a Chave Secreta.",
  "settings.sync.devices.removeConfirm": "Remover dispositivo",
  "settings.sync.devices.folderNote":
    "Para bloquear um dispositivo, altere a sua senha mestra e a Chave Secreta.",
  "settings.sync.phone.title": "Configurar um celular",
  "settings.sync.phone.body":
    "Mostre um código que o seu celular lê, para você não ter de digitar a sua Chave Secreta. O celular ainda precisa da sua senha mestra.",
  "settings.sync.phone.button": "Configurar um celular",
  "settings.sync.phone.reauth":
    "O código contém a sua Chave Secreta, então exige a sua senha mestra.",
  "settings.sync.phone.dialogTitle": "Leia este código com o seu celular",
  "settings.sync.phone.steps":
    "No celular, abra o Mosaic, escolha configurá-lo e leia este código.",
  "settings.sync.phone.hint":
    "O código é ocultado após 60 segundos ou quando você sai desta janela. Não compartilhe uma foto dele.",
  "settings.sync.phone.canvas": "Código de configuração para o seu celular",
  "settings.sync.phone.hidden.time": "Os 60 segundos acabaram, então o código foi ocultado.",
  "settings.sync.phone.hidden.away": "Você saiu desta janela, então o código foi ocultado.",
  "settings.sync.phone.again": "Mostrar de novo",
  "settings.sync.phone.folderHint":
    "Seu celular entra no seu armazenamento na nuvem e abre esta mesma pasta.",
  "settings.sync.totp.title": "App autenticador",
  "settings.sync.totp.body":
    "Pede um código de um app autenticador quando alguém entra na sua conta de sincronização. Protege a conta no servidor, não os seus cofres: esses continuam criptografados com a sua senha mestra e a Chave Secreta.",
  "settings.sync.totp.on": "Os códigos de um app autenticador estão ativados.",
  "settings.sync.totp.off": "Os códigos de um app autenticador estão desativados.",
  "settings.sync.totp.enable": "Ativar…",
  "settings.sync.totp.disable": "Desativar…",
  "settings.sync.totp.beginTitle": "Ativar códigos de app autenticador",
  "settings.sync.totp.beginBody": "Digite a sua senha mestra para começar.",
  "settings.sync.totp.begin": "Continuar",
  "settings.sync.totp.setupBody":
    "Adicione esta chave ao seu app autenticador e digite o código de 6 dígitos que ele mostrar.",
  "settings.sync.totp.secret": "Chave de configuração",
  "settings.sync.totp.confirm": "Ativar",
  "settings.sync.totp.disableTitle": "Desativar códigos de app autenticador",
  "settings.sync.totp.disableBody": "Digite a sua senha mestra e um código atual.",
  "settings.sync.totp.disableConfirm": "Desativar",
  "settings.sync.totp.enabledToast": "Os códigos de um app autenticador estão ativados.",
  "settings.sync.totp.disabledToast": "Os códigos de um app autenticador estão desativados.",
  "settings.sync.key.title": "Chaves de segurança",
  "settings.sync.key.body":
    "Uma chave de segurança pode ser uma segunda etapa ao entrar na sua conta de sincronização. Ela protege a conta no servidor, não os seus cofres. Adicionar ou usar uma chave abre o seu navegador.",
  "settings.sync.key.none": "Nenhuma chave de segurança adicionada.",
  "settings.sync.key.added": "Adicionada {when}",
  "settings.sync.key.lastUsed": "Último uso {when}",
  "settings.sync.key.neverUsed": "Ainda não usada",
  "settings.sync.key.add": "Adicionar uma chave de segurança…",
  "settings.sync.key.addTitle": "Adicionar uma chave de segurança",
  "settings.sync.key.addBody":
    "Dê um nome à chave e digite a sua senha mestra. O Mosaic então abre o seu navegador, onde você usa a chave.",
  "settings.sync.key.label": "Nome da chave",
  "settings.sync.key.labelHint": "Por exemplo, “Chave da mesa”. Até 64 caracteres.",
  "settings.sync.key.addConfirm": "Continuar",
  "settings.sync.key.addedToast": "Chave de segurança adicionada.",
  "settings.sync.key.noBackup":
    "Uma chave de segurança seria a sua única segunda etapa. Se você a perder, talvez não consiga entrar na sua conta de sincronização de novo. Recomendamos uma reserva: uma segunda chave ou um código de um app autenticador.",
  "settings.sync.key.recommend":
    "Recomendamos dois métodos, por exemplo duas chaves, ou uma chave e um código de app autenticador.",
  "settings.sync.key.remove": "Remover",
  "settings.sync.key.removeLabel": "Remover {name}",
  "settings.sync.key.removeTitle": "Remover {name}?",
  "settings.sync.key.removeBody":
    "Você confirma uma vez com uma chave de segurança no navegador. Depois disso, esta chave não funciona mais para entrar.",
  "settings.sync.key.removeLast":
    "Esta é a sua última segunda etapa. A verificação em duas etapas é desativada, e a sua conta de sincronização passa a ser protegida apenas pela senha mestra e pela Chave Secreta.",
  "settings.sync.key.removeCode": "Código do app autenticador",
  "settings.sync.key.removeCodeHint":
    "Digite um código atual para remover a chave imediatamente. Deixe vazio para confirmar com uma chave de segurança no navegador.",
  "settings.sync.key.removeConfirm": "Remover chave",
  "settings.sync.key.removedToast": "Chave de segurança removida.",
  "settings.sync.key.confirmTitle": "Confirme com uma chave de segurança",
  "settings.sync.key.confirmBody":
    "Remover uma chave exige uma segunda etapa. Digite a sua senha mestra e confirme com uma chave no navegador.",
  "settings.sync.key.step.methods": "Verificando como esta conta entra…",
  "settings.sync.key.step.useKey": "Usar chave de segurança (abre o seu navegador)",
  "settings.sync.key.step.keyOnlyTitle": "Use a sua chave de segurança",
  "settings.sync.key.step.keyOnlyBody":
    "Esta conta é protegida por uma chave de segurança. O Mosaic abre uma página no seu navegador, onde você a usa.",
  "settings.sync.key.step.or": "Ou use um código",
  "settings.sync.key.wait.login": "Entre com a sua chave de segurança",
  "settings.sync.key.wait.register": "Adicione a sua chave de segurança",
  "settings.sync.key.wait.confirm": "Confirme com a sua chave de segurança",
  "settings.sync.key.wait.lead":
    "Seu navegador abriu uma página do Mosaic do seu servidor de sincronização. Digite este código lá e use a sua chave de segurança.",
  "settings.sync.key.wait.codeLabel": "Código para digitar no navegador",
  "settings.sync.key.wait.codeSpoken": "Código para digitar no navegador: {digits}",
  "settings.sync.key.wait.expires": "Esta etapa expira às {time}.",
  "settings.sync.key.wait.waiting": "Aguardando a sua chave de segurança…",
  "settings.sync.key.wait.done": "Concluído.",
  "settings.sync.key.wait.expired": "Esta etapa expirou. Comece-a de novo.",
  "settings.sync.key.wait.failed":
    "A etapa falhou: o código foi errado vezes demais ou a chave de segurança não foi aceita. Nada foi alterado. Comece de novo.",
  "settings.sync.key.wait.reopen": "Abrir a página do navegador de novo",
  "settings.sync.key.wait.retry": "Começar de novo",
  "sync.warning.region": "Avisos de sincronização",
  "sync.warning.unknownDevice": "outro dispositivo",
  "sync.warning.unknownVault": "um dos seus cofres",
  "sync.warning.freezeSuspected":
    "As alterações de {device} não chegam há mais de um dia. Verifique se o outro dispositivo está ligado e online.",
  "sync.warning.missingFiles":
    "Arquivos de {device} estão ausentes da pasta. Deixe o seu serviço de nuvem terminar de sincronizar e sincronize de novo.",
  "sync.warning.noCommonView":
    "Este dispositivo e {device} não viram os mesmos dados do servidor por dois dias. O servidor pode estar mostrando versões diferentes a cada um.",
  "sync.warning.rollbackDetected":
    "Seus dados de sincronização parecem mais antigos que os deste dispositivo. Verifique o servidor ou a pasta e depois escolha “Já verifiquei”.",
  "sync.warning.identityDuplicated":
    "Outra cópia do Mosaic está gravando com a identidade deste dispositivo, por exemplo após a restauração de um backup. A sincronização está pausada. Certifique-se de que só uma cópia está em uso e depois escolha “Já verifiquei”.",
  "sync.warning.serverInconsistent":
    "O servidor mostrou a este dispositivo dados diferentes dos de outro dispositivo. Verifique o servidor e depois escolha “Já verifiquei”.",
  "sync.warning.serverRollback":
    "A cópia do servidor de {vault} é mais antiga que a deste dispositivo. Verifique o servidor e depois escolha “Já verifiquei”.",
  "sync.warning.undecryptable":
    "{count, plural, one {# item recebido não pôde ser lido} other {# itens recebidos não puderam ser lidos}} neste dispositivo. Atualize o Mosaic em todos os seus dispositivos e sincronize de novo.",
  "sync.warning.hardwareVaultsLocal":
    "Os cofres protegidos por hardware ficam neste dispositivo. {count, plural, one {# cofre não é sincronizado} other {# cofres não são sincronizados}}.",
  "sync.warning.attachmentsNotRepaired":
    "{count, plural, one {# arquivo} other {# arquivos}} no servidor ou na pasta não puderam ser reparados a partir deste dispositivo. Abra o item em um dispositivo que ainda tenha o arquivo e adicione-o de novo.",
  "sync.warning.acknowledge": "Já verifiquei…",
  "sync.acknowledge.title": "Limpar os avisos de sincronização que precisam de verificação?",
  "sync.acknowledge.body":
    "Só prossiga depois de verificar o servidor ou a pasta você mesmo. Isto limpa todos os avisos que precisam de verificação, e a sincronização continua. Se os seus dados realmente foram revertidos ou alterados, o Mosaic não consegue saber depois.",
  "sync.acknowledge.eachDevice": "Faça isto em cada um dos seus dispositivos.",
  "toast.syncAttention":
    "A sincronização mesclou ou reparou alguns itens de outro dispositivo. Veja o que mudou.",
  "sync.revoked.rekey":
    "As chaves desta conta foram renovadas em outro dispositivo: entre com a nova Chave Secreta do seu Kit de Emergência mais recente.",
  "sync.revoked.passwordChange":
    "Sua senha mestra foi alterada em outro dispositivo: entre com a nova senha.",
  "sync.revoked.deviceRemoved":
    "Este dispositivo foi removido da sua conta em outro dispositivo. Entre de novo para adicioná-lo outra vez.",
  "sync.revoked.operator":
    "O operador do servidor encerrou esta sessão. Entre de novo para continuar sincronizando.",
} satisfies Record<keyof typeof en, string>;
