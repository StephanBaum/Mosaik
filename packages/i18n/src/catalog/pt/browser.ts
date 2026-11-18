/**
 * Brazilian Portuguese UI strings: browser. Same keys as English (checked by the type).
 * ICU plural categories of Brazilian Portuguese: one, many, other (CLDR; `other` is required).
 */
import type { browser as en } from "../en/browser.ts";

export const browser = {
  "browser.pair.title": "Um navegador quer se conectar ao Mosaic",
  "browser.pair.description":
    "Só permita se você acabou de clicar em Conectar na extensão do Mosaic e ela mostra este código.",
  "browser.pair.code": "Código",
  "browser.pair.browser": "Navegador",
  "browser.pair.label": "Nome deste navegador",
  "browser.pair.labelHint": "Você verá este nome em Configurações.",
  "browser.pair.deny": "Recusar",
  "browser.pair.allow": "Permitir",
  "browser.expires": "Fecha automaticamente em {seconds, plural, one {# segundo} other {# segundos}}",
  "browser.escDenies": "para recusar",
  "browser.hint.chrome": "Chrome",
  "browser.hint.edge": "Edge",
  "browser.hint.brave": "Brave",
  "browser.hint.opera": "Opera",
  "browser.hint.vivaldi": "Vivaldi",
  "browser.hint.chromium": "Chromium",
  "browser.hint.firefox": "Firefox",
  "browser.hint.other": "Navegador",
  "browser.family.chromium": "Navegador baseado em Chromium",
  "browser.family.firefox": "Firefox",
  "browser.escCancels": "para cancelar",
  "browser.save.titleNew": "Salvar este login no Mosaic?",
  "browser.save.titleUpdate": "Atualizar este login no Mosaic?",
  "browser.save.description":
    "{browser} se oferece para salvar um login. O Mosaic só o grava se você escolher Salvar.",
  "browser.save.descriptionUpdate":
    "{browser} oferece uma nova senha para este login. O Mosaic só a altera se você escolher Atualizar senha.",
  "browser.save.site": "Site",
  "browser.save.username": "Nome de usuário",
  "browser.save.noUsername": "Nenhum",
  "browser.save.password": "Senha",
  "browser.save.passwordHidden": "Oculta",
  "browser.save.target": "Salvar como",
  "browser.save.updateOption": "Atualizar “{title}”",
  "browser.save.newOption": "Um novo login",
  "browser.save.itemTitle": "Título",
  "browser.save.vault": "Cofre",
  "browser.save.allowHttp": "Permitir preenchimento nesta página sem criptografia",
  "browser.save.allowHttpHint":
    "O site não usa HTTPS. Qualquer pessoa na rede poderia ler o que você preencher.",
  "browser.save.cancel": "Cancelar",
  "browser.save.saveNew": "Salvar",
  "browser.save.update": "Atualizar senha",
  "browser.passkey.secure": "Conexão segura (https)",
  "browser.passkey.getTitle": "Entrar em {rpId} com uma chave de acesso",
  "browser.passkey.createTitle": "Salvar uma nova chave de acesso",
  "browser.passkey.excludedTitle": "Chave de acesso já salva",
  "browser.passkey.createBody": "Criar uma chave de acesso para {rpId} no Mosaic?",
  "browser.passkey.excludedBody": "Você já tem uma chave de acesso para {rpId} no Mosaic.",
  "browser.passkey.uvNote":
    "O site pediu verificação do usuário. Digite sua senha mestra para confirmar que é você.",
  "browser.passkey.password": "Senha mestra",
  "browser.passkey.account": "Conta",
  "browser.passkey.noName": "Sem nome",
  "browser.passkey.vault": "Cofre",
  "browser.passkey.signIn": "Entrar com chave de acesso",
  "browser.passkey.save": "Salvar chave de acesso",
  "browser.passkey.anotherDevice": "Usar outro dispositivo ou chave de segurança",
  "browser.passkey.cancel": "Cancelar",
  "browser.passkey.ok": "Entendi",
  "browser.passkey.escCloses": "para fechar",
  "browser.passkey.passwordNeeded": "Digite sua senha mestra para confirmar que é você.",
  "browser.settings.title": "Navegadores",
  "browser.settings.body":
    "Extensões de navegador que preenchem logins a partir deste app. Você aprova cada conexão aqui.",
  "browser.settings.empty":
    "Nenhum navegador conectado ainda. Instale a extensão do Mosaic e escolha Conectar nela.",
  "browser.settings.connected": "Conectado agora",
  "browser.settings.lastUsed": "Último uso {when}",
  "browser.settings.neverUsed": "Ainda não usado",
  "browser.settings.disconnect": "Desconectar",
  "browser.settings.disconnectNamed": "Desconectar {name}",
  "browser.settings.disconnectTitle": "Desconectar “{name}”?",
  "browser.settings.disconnectBody":
    "Este navegador não poderá preencher logins até você conectá-lo de novo pela extensão.",
  "browser.settings.endpointInUse":
    "Outro programa, ou outra janela do Mosaic, já usa a conexão para navegadores. Feche-o e reinicie o Mosaic.",
  "browser.settings.endpointUnavailable":
    "O Mosaic não conseguiu abrir a conexão para navegadores. Reinicie o Mosaic. Se continuar acontecendo, verifique se a sua pasta de usuário permite gravação.",
  "browser.settings.registryUnreadable":
    "Não é possível ler as conexões de navegador salvas. O Mosaic não as alterará até isso ser corrigido.",
  "browser.connector.title": "Uma conexão de navegador aponta para outro lugar",
  "browser.connector.body":
    "A conexão de {browser} inicia um programa diferente em vez do Mosaic: {path}. Pode ser um Mosaic mais antigo ou algo que se registrou por conta própria. Reparar a aponta de novo para este Mosaic.",
  "browser.connector.item": "{browser}: {path}",
  "browser.connector.repair": "Reparar",
  "browser.connector.repaired": "Conexão de navegador reparada.",
} satisfies Record<keyof typeof en, string>;
