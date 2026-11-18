/**
 * Brazilian Portuguese UI strings: settings. Same keys as English (checked by the type).
 * ICU plural categories of Brazilian Portuguese: one, many, other (CLDR; `other` is required).
 */
import type { settings as en } from "../en/settings.ts";

export const settings = {
  "settings.title": "Configurações",
  "settings.security": "Segurança",
  "settings.autoLock": "Bloquear após inatividade",
  "settings.autoLock.never": "Nunca",
  "settings.autoLock.minutes": "{minutes, plural, one {# minuto} other {# minutos}}",
  "settings.lockOnSleep": "Bloquear quando o computador entrar em suspensão",
  "settings.lockOnScreenLock": "Bloquear quando a tela for bloqueada",
  "settings.screenLock.unsupported":
    "O Mosaic não consegue saber quando a tela é bloqueada neste sistema. Ele ainda bloqueia após inatividade e quando o computador volta da suspensão.",
  "settings.lockTrigger.required":
    "Mantenha pelo menos um bloqueio ativado. O Mosaic não consegue detectar uma tela bloqueada neste sistema.",
  "settings.clipboard": "Limpar a área de transferência após",
  "settings.clipboard.never": "Nunca",
  "settings.clipboard.seconds": "{seconds, plural, one {# segundo} other {# segundos}}",
  "settings.appearance": "Aparência",
  "settings.theme": "Tema",
  "settings.theme.system": "Seguir o sistema",
  "settings.theme.light": "Claro",
  "settings.theme.dark": "Escuro",
  "settings.language": "Idioma",
  "settings.language.system": "Seguir o sistema",
  "settings.language.machineNote":
    "Traduzido com ajuda de IA. Se algo soar errado, avise-nos.",
  "settings.import": "Importar",
  "settings.import.body":
    "Traga os seus logins de um navegador ou de outro gerenciador de senhas. O Mosaic guia você na exportação e ajuda a excluir o arquivo depois.",
  "settings.import.open": "Importar senhas",
  "settings.ssh": "Agente SSH",
  "settings.ssh.enable": "Usar as minhas chaves SSH do Mosaic",
  "settings.ssh.hint":
    "Desativado por padrão. O agente só funciona enquanto o Mosaic está desbloqueado e pergunta a você antes de cada uso.",
  "settings.ssh.status":
    "{count, plural, =0 {Nenhuma chave disponibilizada} one {# chave disponibilizada} other {# chaves disponibilizadas}}",
  "settings.ssh.endpoint": "Defina SSH_AUTH_SOCK como",
  "settings.ssh.skipped": "Chaves que não são disponibilizadas",
  "settings.kit": "Kit de Emergência",
  "settings.kit.save": "Salvar o Kit de Emergência de novo",
  "settings.saved": "Configurações salvas.",
  "reauth.title": "Confirme com a sua senha mestra",
  "reauth.body":
    "Esta alteração torna o Mosaic menos seguro, então exige a sua senha mestra.",
  "reauth.bodyKit":
    "Sua Chave Secreta está prestes a ser exibida em um arquivo, então o Mosaic precisa da sua senha mestra.",
  "reauth.password": "Senha mestra",
  "reauth.submit": "Confirmar",
  "settings.intro":
    "Como o Mosaic bloqueia, limpa segredos e protege as suas chaves neste dispositivo.",
  "settings.section.autoLock": "Bloqueio automático",
  "settings.section.autoLock.body": "Quando o Mosaic se bloqueia sem você pedir.",
  "settings.section.clipboard": "Área de transferência",
  "settings.section.clipboard.body": "Os segredos copiados são limpos após este tempo.",
  "settings.appearance.body": "O tema e o idioma do Mosaic.",
  "settings.ssh.body":
    "Deixe programas entrarem com chaves SSH do seu cofre. Você aprova cada uso.",
  "settings.kit.body":
    "Salve uma nova cópia do seu Kit de Emergência. Isto exige a sua senha mestra.",
  "settings.quick": "Desbloqueio rápido",
  "settings.quick.body": "Formas mais rápidas de voltar, além da sua senha mestra.",
  "settings.quick.windowsHello": "Windows Hello",
  "settings.quick.touchId": "Touch ID",
  "settings.quick.biometric.body":
    "Desbloqueie com o rosto, a impressão digital ou o PIN. A sua senha mestra ainda é necessária após reiniciar.",
  "settings.quick.biometric.bodyMac":
    "Desbloqueie com a impressão digital. A sua senha mestra ainda é necessária após reiniciar.",
  "settings.quick.expires":
    "O Mosaic pedirá a sua senha mestra de novo em {date, date, medium}.",
  "settings.quick.maxAge": "Pedir a senha mestra pelo menos a cada",
  "settings.quick.maxAgeDays": "{days, plural, one {# dia} other {# dias}}",
  "settings.quick.biometricLinux": "O desbloqueio biométrico não está disponível no Linux.",
  "settings.quick.biometricUnavailable": "Não configurado neste dispositivo.",
  "settings.quick.keys": "Chaves de segurança",
  "settings.quick.keysCount":
    "{count, plural, =0 {Nenhuma registrada} one {# registrada} other {# registradas}}",
  "settings.quick.keysUnsupported":
    "As chaves de segurança ainda não são compatíveis com este sistema.",
  "settings.quick.keyRow": "Chave de segurança · {tag} · adicionada em {addedAt, date, medium}",
  "settings.quick.addKey": "Adicionar chave de segurança",
  "settings.quick.addKeyBody":
    "Insira a chave que você quer adicionar e digite o PIN dela. Depois toque nela quando piscar.",
  "settings.quick.manageKeys": "Gerenciar",
  "settings.quick.removeKey": "Remover",
  "settings.quick.removeKeyNamed": "Remover chave de segurança {tag}",
  "settings.quick.removeKeyConfirm": "Remover chave de segurança {tag}?",
  "settings.quick.keyRisk":
    "Qualquer pessoa com esta chave e o PIN dela pode abrir o seu cofre sem a sua senha mestra.",
  "reauth.bodyQuick":
    "Isto facilita a entrada no Mosaic, então exige a sua senha mestra.",
  "reauth.bodyKey": "Adicionar uma chave a este cofre exige a sua senha mestra.",
  "settings.passkeyExport": "Exportar chaves de acesso",
  "settings.passkeyExport.body":
    "Salve as chaves de acesso de um cofre em um arquivo FIDO CXF, para movê-las para outro app.",
  "settings.passkeyExport.vault": "Cofre",
  "settings.passkeyExport.warning":
    "O arquivo contém as suas chaves de acesso sem criptografia. Qualquer pessoa com o arquivo pode entrar como você. Exclua-o após importar.",
  "settings.passkeyExport.none": "Este cofre não tem chaves de acesso para exportar.",
  "settings.passkeyExport.button": "Exportar chaves de acesso",
  "settings.passkeyExport.reauth":
    "O arquivo conterá as suas chaves de acesso sem criptografia, então exportar exige a sua senha mestra.",
  "settings.passkeyExport.saved":
    "{count, plural, one {# chave de acesso exportada.} other {# chaves de acesso exportadas.}} Exclua o arquivo após importar.",
  "reauth.bodyMove":
    "Mover um item para fora de um cofre protegido por chave de segurança exige a sua senha mestra.",
  "toast.settingsReset":
    "As suas configurações de segurança foram alteradas fora do Mosaic e foram redefinidas para padrões seguros.",
  "settings.shortcut.title": "Atalho de busca rápida",
  "settings.shortcut.body":
    "Pressione esta combinação em qualquer programa para abrir a busca do Mosaic.",
  "settings.shortcut.label": "Atalho",
  "settings.shortcut.hint":
    "Clique no campo e pressione as teclas que quiser: Ctrl, Alt ou ⌘ (em um Mac), mais uma tecla.",
  "settings.shortcut.save": "Salvar atalho",
  "settings.shortcut.reset": "Usar o padrão",
  "settings.shortcut.saved": "Atalho salvo: {shortcut}.",
  "settings.shortcut.invalid":
    "Esse não é um atalho que o Mosaic possa usar. Pressione Ctrl, Alt ou ⌘ (em um Mac) junto com mais uma tecla.",
  "settings.shortcut.unsupported":
    "Seu sistema não permite que apps definam um atalho global, então isto não pode ser alterado aqui.",
} satisfies Record<keyof typeof en, string>;
