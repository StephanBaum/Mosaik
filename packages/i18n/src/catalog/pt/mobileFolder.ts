/**
 * Brazilian Portuguese UI strings: mobileFolder. Same keys as English (checked by the type).
 * ICU plural categories of Brazilian Portuguese: one, many, other (CLDR; `other` is required).
 */
import type { mobileFolder as en } from "../en/mobileFolder.ts";

export const mobileFolder = {
  "mobile.sync.never": "Ainda não sincronizado",
  "mobile.sync.last": "Sincronizado {time}",
  "mobile.sync.localOnly":
    "Este cofre está apenas neste celular. A sincronização não está configurada.",
  "mobile.sync.pending":
    "{count, plural, one {# alteração aguardando envio} other {# alterações aguardando envio}}",
  "mobile.sync.now": "Sincronizar agora",
  "mobile.sync.working": "Sincronizando",
  "mobile.sync.warnings": "Avisos de sincronização",
  "mobile.sync.warning.freezeSuspected":
    "As alterações de outro dispositivo não chegam há algum tempo. Verifique se o outro dispositivo está ligado e online.",
  "mobile.sync.warning.noCommonView":
    "Este celular e outro dispositivo não viram os mesmos dados do servidor por dois dias. O servidor pode estar mostrando versões diferentes a cada um. Sincronize o seu outro dispositivo e depois sincronize aqui.",
  "mobile.sync.warning.rollbackDetected":
    "Seus dados de sincronização parecem mais antigos do que os deste celular. Abra o Mosaic em outro dispositivo e verifique se as suas últimas alterações estão lá. Depois escolha “Já verifiquei”.",
  "mobile.sync.warning.identityDuplicated":
    "Outra cópia do Mosaic está gravando com a identidade deste celular, por exemplo após a restauração de um backup. A sincronização está pausada. Certifique-se de que só uma cópia está em uso e depois escolha “Já verifiquei”.",
  "mobile.sync.warning.serverInconsistent":
    "O servidor mostrou a este celular dados diferentes dos de outro dispositivo. Abra o Mosaic em outro dispositivo e verifique se as suas últimas alterações estão lá. Depois escolha “Já verifiquei”.",
  "mobile.sync.warning.serverRollback":
    "A cópia do servidor de um dos seus cofres é mais antiga que a deste celular. Abra o Mosaic em outro dispositivo e verifique se as suas últimas alterações estão lá. Depois escolha “Já verifiquei”.",
  "mobile.sync.warning.sharingAttention":
    "Um cofre compartilhado precisa de atenção: há uma troca de chave aguardando ou um membro não pôde ser verificado. Abra o Mosaic no seu computador para ver qual. Depois escolha “Já verifiquei”.",
  "mobile.sync.warning.undecryptable":
    "{count, plural, one {# item recebido não pôde ser lido} other {# itens recebidos não puderam ser lidos}} neste celular. Atualize o Mosaic em todos os seus dispositivos e sincronize de novo.",
  "mobile.sync.warning.accountRollback":
    "O servidor mostra uma chave de conta mais antiga do que a deste celular. Nada foi alterado. Abra o Mosaic em outro dispositivo e verifique se as suas últimas alterações estão lá. Depois escolha “Já verifiquei”.",
  "mobile.sync.acknowledge": "Já verifiquei…",
  "mobile.sync.acknowledgeTitle": "Limpar os avisos de sincronização?",
  "mobile.sync.acknowledgeBody":
    "Só prossiga depois de verificar o servidor você mesmo. Isto limpa todos os avisos, e a sincronização continua. Se os seus dados realmente foram revertidos ou alterados, o Mosaic não consegue saber depois. Faça isto em cada um dos seus dispositivos.",
  "mobile.sync.acknowledgeConfirm": "Já verifiquei",
  "mobile.folder.intro": "Esta conta sincroniza pela sua pasta do {provider}.",
  "mobile.folder.signIn": "Entrar no {provider}",
  "mobile.folder.scope":
    "O {provider} pedirá acesso a todos os seus arquivos. O Mosaic só abre a pasta que você escolher e nunca toca em mais nada.",
  "mobile.folder.waiting": "Conclua o login no navegador e volte aqui.",
  "mobile.folder.serverLabel": "Endereço do Nextcloud ou WebDAV",
  "mobile.folder.serverHint": "https://cloud.example.com",
  "mobile.folder.serverConfirm":
    "O Mosaic entrará em {server}. Só continue se este for o seu próprio servidor.",
  "mobile.folder.locating": "Procurando a sua pasta do Mosaic…",
  "mobile.folder.browseTitle": "Escolha a sua pasta do Mosaic",
  "mobile.folder.browseHint":
    "É a pasta que o seu computador sincroniza, com um arquivo chamado mosaic-sync.json.",
  "mobile.folder.currentFolder": "Pasta atual: {path}",
  "mobile.folder.noSubfolders": "Não há pastas aqui.",
  "mobile.folder.useThis": "Usar esta pasta",
  "mobile.folder.up": "Subir um nível",
  "mobile.folder.chooseProvider": "Onde a sua conta sincroniza?",
  "mobile.folder.noDrive":
    "Google Drive e iCloud Drive ainda não funcionam no Android. Use Dropbox, OneDrive, Nextcloud ou a sincronização do Mosaic.",
  "mobile.folder.useFolder": "Sincronizar por uma pasta na nuvem",
  "mobile.folder.providerDropbox": "Dropbox",
  "mobile.folder.providerOneDrive": "OneDrive",
  "mobile.folder.providerWebDav": "Nextcloud ou WebDAV",
  "mobile.folder.appPasswordToggle": "Usar uma senha de app",
  "mobile.folder.davLabel": "Endereço da pasta WebDAV",
  "mobile.folder.loginLabel": "Nome de usuário",
  "mobile.folder.appPasswordLabel": "Senha de app",
  "mobile.folder.appPasswordHint":
    "Crie uma senha de app nas configurações de segurança do seu servidor. Não use a sua senha principal.",
  "mobile.folder.appPasswordAction": "Entrar com a senha de app",
  "mobile.folder.pickAccount": "A qual conta este celular está se juntando?",
  "mobile.folder.accountOption": "Conta {id}",
  "mobile.folder.linked": "Login feito. Sincronizando de novo.",
  "mobile.folder.settingsLine": "Sincroniza pelo {provider}: {path}",
  "mobile.folder.relink": "Entrar de novo",
  "mobile.folder.msRevokeHint":
    "Para remover por completo o acesso do Mosaic, remova-o na página da sua conta Microsoft, em Aplicativos.",
  "mobile.reset.revokeDropbox":
    "A conta foi removida. Não foi possível encerrar o acesso do Mosaic no Dropbox: remova-o no Dropbox, em Configurações, Aplicativos conectados.",
  "mobile.reset.revokeWebdav":
    "A conta foi removida. Não foi possível excluir a senha de app no seu servidor: remova-a lá, em Configurações, Segurança, Dispositivos e sessões.",
  "mobile.reset.title": "Remover esta conta deste celular",
  "mobile.reset.body":
    "Isto exclui a cópia do cofre, a Chave Secreta e o login na nuvem guardados neste celular. Seus dados na pasta da nuvem ou no servidor continuam. Você poderá entrar de novo depois com a sua Chave Secreta e a senha mestra.",
  "mobile.reset.action": "Remover conta…",
  "mobile.reset.confirm": "Remover conta",
  "mobile.reset.phraseLabel": "Confirmação",
  "mobile.reset.typePhrase": "Digite {phrase} para confirmar",
} satisfies Record<keyof typeof en, string>;
