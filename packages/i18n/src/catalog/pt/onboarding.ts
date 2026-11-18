/**
 * Brazilian Portuguese UI strings: onboarding. Same keys as English (checked by the type).
 * ICU plural categories of Brazilian Portuguese: one, many, other (CLDR; `other` is required).
 */
import type { onboarding as en } from "../en/onboarding.ts";

export const onboarding = {
  "onboarding.secretKey.title": "Sua Chave Secreta",
  "onboarding.secretKey.body":
    "Sua Chave Secreta e a sua senha mestra juntas desbloqueiam o seu cofre. Não podemos recuperar nenhuma das duas.",
  "onboarding.secretKey.verify":
    "Digite o último grupo da sua Chave Secreta para confirmar que você a salvou.",
  "onboarding.secretKey.mistyped":
    "Isso não confere. Confira o último grupo no seu Kit de Emergência.",
  "onboarding.step": "Etapa {current} de {total}",
  "onboarding.create.title": "Crie a sua conta",
  "onboarding.create.body":
    "Sua senha mestra e uma Chave Secreta que geramos neste dispositivo desbloqueiam juntas o seu cofre. Nenhuma das duas é enviada ao servidor.",
  "onboarding.create.email": "E-mail",
  "onboarding.create.password": "Senha mestra",
  "onboarding.create.passwordHint":
    "Quatro palavras sem relação entre si funcionam bem. Use-a apenas para o Mosaic.",
  "onboarding.create.confirm": "Confirmar senha mestra",
  "onboarding.create.mismatch":
    "As duas senhas não conferem. Digite a mesma senha mestra nos dois campos.",
  "onboarding.create.submit": "Criar conta",
  "onboarding.create.working": "Criando a sua conta. Isto leva alguns segundos.",
  "onboarding.kit.accountId": "ID da conta",
  "onboarding.kit.secretKey": "Chave Secreta",
  "onboarding.kit.lastGroup": "Último grupo da sua Chave Secreta",
  "onboarding.kit.confirm": "Confirmar",
  "kit.title": "Salve o seu Kit de Emergência",
  "kit.body":
    "O kit é um PDF com a sua Chave Secreta. Junto com a sua senha mestra, ele abre o seu cofre em um novo dispositivo. Guarde-o em um lugar seguro, longe deste computador.",
  "kit.secretKey": "Sua Chave Secreta",
  "kit.save": "Salvar Kit de Emergência (PDF)",
  "kit.saved": "Salvo. Guarde o arquivo em um lugar seguro.",
  "kit.cancelled": "Ainda não salvo. Salve o kit para continuar.",
  "kit.recover.alreadySaved": "Já salvei",
  "kit.recover.title": "Salve o seu Kit de Emergência de novo",
  "kit.recover.body":
    "Você ainda não confirmou o seu Kit de Emergência. Digite a sua senha mestra para salvá-lo de novo e depois confirme o último grupo da sua Chave Secreta.",
  "onboarding.import.title": "Traga as suas senhas",
  "onboarding.import.body":
    "De Chrome, Edge, Firefox, Safari e outros navegadores, ou de 1Password, Bitwarden, LastPass, Dashlane, Keeper, NordPass, Proton Pass, RoboForm, KeePass e Enpass. Leva alguns minutos. Você também pode fazer isso depois, em Configurações.",
  "onboarding.import.start": "Importar de outro app",
  "onboarding.import.skip": "Pular por enquanto",
  "onboarding.done.title": "Tudo pronto",
  "onboarding.done.body": "Seu cofre está pronto. Bloqueie-o a qualquer momento com o botão Bloquear.",
  "onboarding.browser.title": "Instale a extensão do navegador",
  "onboarding.browser.body":
    "Preencha logins no seu navegador. A extensão só preenche quando você clica nela ou usa o atalho de teclado.",
  "onboarding.browser.steps": "Como conectar um navegador",
  "onboarding.browser.step1": "Instale a extensão do Mosaic no seu navegador.",
  "onboarding.browser.step2": "Abra a extensão e escolha Conectar.",
  "onboarding.browser.step3":
    "Confira se o código é o mesmo e escolha Permitir na janela que abrir aqui.",
  "onboarding.browser.notInStores":
    "A extensão ainda não está nas lojas dos navegadores. Você pode conectá-la depois em Configurações → Navegadores.",
  "onboarding.browser.getFor": "Obter para {browser}",
  "onboarding.browser.connectedList": "Navegadores conectados",
  "onboarding.browser.waiting": "Aguardando a extensão. Nenhum navegador conectado ainda.",
  "onboarding.browser.connected": "{name} conectado",
  "onboarding.browser.done": "Concluir",
  "onboarding.browser.skip": "Pular por enquanto",
  "onboarding.progress": "Progresso da configuração",
  "onboarding.stepName.create": "Criar conta",
  "onboarding.stepName.kit": "Kit de Emergência",
  "onboarding.stepName.verify": "Confirmar o kit",
  "onboarding.stepName.import": "Importar (opcional)",
  "onboarding.stepName.browser": "Extensão do navegador (opcional)",
  "kit.preview": "Pré-visualização do Kit de Emergência",
  "kit.continueHint": "Salve o kit para continuar.",
  "kit.warning":
    "Sem este kit e a sua senha mestra, ninguém pode recuperar o seu cofre, nem mesmo nós.",
  "onboarding.join.link": "Já uso o Mosaic",
  "onboarding.join.title": "Adicione este dispositivo à sua conta",
  "onboarding.join.body":
    "Entre com a sua senha mestra e a Chave Secreta do seu Kit de Emergência. Seus cofres são então baixados e descriptografados neste dispositivo.",
  "onboarding.join.tabs": "Onde estão os seus dados de sincronização?",
  "onboarding.join.tab.server": "Servidor Mosaic",
  "onboarding.join.tab.folder": "Pasta na nuvem",
  "onboarding.join.serverUrl": "Endereço do servidor",
  "onboarding.join.email": "E-mail da conta de sincronização",
  "onboarding.join.password": "Senha mestra",
  "onboarding.join.secretKey": "Chave Secreta",
  "onboarding.join.secretKeyHint": "Do seu Kit de Emergência. Ela começa com M1.",
  "onboarding.join.deviceName": "Nome deste dispositivo",
  "onboarding.join.chooseFolder": "Escolher pasta…",
  "onboarding.join.folder": "Pasta",
  "onboarding.join.noAccounts":
    "Esta pasta não contém nenhuma conta do Mosaic. Escolha a pasta com a qual o seu outro dispositivo sincroniza.",
  "onboarding.join.account": "Conta",
  "onboarding.join.accountOption": "Conta {id}",
  "onboarding.join.submit": "Adicionar este dispositivo",
  "onboarding.join.working": "Entrando e baixando os seus cofres…",
  "onboarding.join.twoFactor.title": "Digite o seu código",
  "onboarding.join.twoFactor.body":
    "Sua conta de sincronização usa verificação em duas etapas. Digite o código de 6 dígitos do seu app autenticador.",
  "onboarding.join.twoFactor.code": "Código de 6 dígitos",
  "onboarding.join.twoFactor.submit": "Verificar",
  "onboarding.join.back": "Criar uma nova conta",
} satisfies Record<keyof typeof en, string>;
