/**
 * Brazilian Portuguese UI strings: vault. Same keys as English (checked by the type).
 * ICU plural categories of Brazilian Portuguese: one, many, other (CLDR; `other` is required).
 */
import type { vault as en } from "../en/vault.ts";

export const vault = {
  "vault.itemCount": "{count, plural, =0 {Nenhum item} one {# item} other {# itens}}",
  "vault.deleteConfirm":
    "Excluir “{title}”? Você pode restaurá-lo da Lixeira por 30 dias.",
  "vault.defaultName": "Pessoal",
  "vault.unnamed": "Cofre sem nome",
  "vault.vaults": "Cofres",
  "vault.allItems": "Todos os itens",
  "vault.items": "Itens",
  "vault.search": "Pesquisar no seu cofre",
  "vault.untitled": "Sem título",
  "vault.noResults":
    "Nada corresponde a “{query}”. Confira a grafia ou pesquise por um site.",
  "vault.new": "Novo item",
  "vault.selectItem": "Selecione um item para ver os detalhes.",
  "vault.trashEmpty": "A Lixeira está vazia. Os itens excluídos ficam aqui por 30 dias.",
  "vault.favorite": "Favorito",
  "vault.unfavorite": "Remover dos favoritos",
  "vault.moveTo": "Mover para o cofre",
  "vault.allVaults": "Todos os cofres",
  "vault.category.favorites": "Favoritos",
  "vault.category.logins": "Logins",
  "vault.category.passkeys": "Chaves de acesso",
  "vault.category.codes": "Códigos de uso único",
  "vault.category.cards": "Cartões",
  "vault.category.notes": "Notas seguras",
  "vault.category.ssh": "Chaves SSH",
  "vault.flag.otp": "2FA",
  "vault.moveToNamed": "Mover para {name}",
  "vault.emptyTitle": "Seu cofre está vazio",
  "vault.emptyBody":
    "Adicione um login, uma chave de acesso, um cartão ou uma nota. Tudo é criptografado neste dispositivo antes de sincronizar.",
  "vault.addFirst": "Adicione o seu primeiro item",
  "vault.paletteTip.before": "Pressione",
  "vault.paletteTip.after": "a qualquer momento para buscar ou executar um comando.",
  "trash.retention": "Os itens excluídos são mantidos por 30 dias.",
  "trash.rowMeta":
    "Excluído em {deletedAt, date, medium} · {days, plural, one {resta # dia} other {restam # dias}}",
  "trash.detailMeta": "{kind} · {vault} · Excluído em {deletedAt, date, medium}",
  "trash.purgeConfirm.title": "Excluir “{title}” permanentemente?",
  "trash.purgeConfirm.body": "Isto não pode ser desfeito.",
  "vault.hardware.placeholderName": "Códigos (chave de segurança)",
  "vault.hardware.title": "Este cofre exige a sua chave de segurança",
  "vault.hardware.body":
    "Os códigos aqui continuam seguros mesmo que alguém descubra a sua senha mestra e a Chave Secreta, mas você não pode vê-los sem a sua chave de segurança.",
  "vault.hardware.open": "Usar chave de segurança",
  "vault.hardware.lockedNote": "Bloqueado. Seus outros cofres continuam desbloqueados.",
  "vault.hardware.lockNow": "Bloquear este cofre",
  "vault.hardware.closesIn": "Bloqueia em {time}",
  "vault.hardware.backupTitle": "Registre uma chave de reserva",
  "vault.hardware.backupBody":
    "Você tem 1 chave registrada. Recomendamos duas: se você perder a sua única chave, não poderá abrir este cofre de novo.",
  "vault.hardware.createOption": "Proteger com uma chave de segurança (somente códigos)",
  "vault.hardware.tradeoff":
    "Os códigos deste cofre exigem a sua chave de segurança toda vez, então uma senha mestra roubada sozinha não basta, mas perder todas as chaves registradas faz perder estes códigos.",
  "vault.hardware.removeNote":
    "Remover uma chave não altera a chave deste cofre. Se uma chave foi roubada, mova os códigos para um novo cofre.",
  "vault.hardware.moveNote":
    "Mover para este cofre um código que você já usa não protege as cópias anteriores: o item antigo, o histórico dele, backups e exportações. Se puder, emita um novo segredo no provedor e adicione esse.",
  "vault.hardware.pinBody":
    "Digite o PIN da sua chave, se ela tiver um, e toque nela quando piscar.",
  "vault.hardware.keys": "Chaves",
  "vault.hardware.keysLabel": "Chaves deste cofre",
  "vault.hardware.eyebrow": "Códigos protegidos por hardware",
  "vault.hardware.selfLocked":
    "Este cofre se bloqueou sozinho. Use a sua chave de segurança para abri-lo de novo.",
  "vault.hardware.closingSoon":
    "Este cofre bloqueia em menos de 30 segundos. Use a sua chave de segurança para abri-lo de novo depois.",
  "vault.hardware.keysTitle": "Chaves deste cofre",
  "vault.hardware.codesOnly": "Somente códigos de uso único",
  "vault.create": "Novo cofre",
  "vault.create.name": "Nome do cofre",
  "vault.create.submit": "Criar cofre",
  "vault.menu": "Ações do cofre",
  "vaultKey.menu": "Renovar chave do cofre…",
  "vaultKey.title": "Renovar a chave de {vault}",
  "vaultKey.body":
    "O Mosaic cria uma nova chave para este cofre e a repassa a cada dispositivo e pessoa que tem acesso. As cópias que eles já têm continuam legíveis para eles; a nova chave protege o que mudar daqui em diante. Use isto depois que alguém perdeu o acesso ou um dispositivo foi perdido.",
  "vaultKey.attachments": "Renovar também as chaves dos anexos",
  "vaultKey.attachments.hint":
    "Dá também uma nova chave a cada arquivo deste cofre. Leva mais tempo com muitos arquivos ou arquivos grandes.",
  "vaultKey.reauth": "Renovar a chave de um cofre exige a sua senha mestra.",
  "vaultKey.submit": "Renovar chave",
  "vaultKey.working": "Renovando a chave…",
  "vaultKey.progress": "Renovando as chaves dos anexos: {done} de {total}.",
  "vaultKey.done": "A chave do cofre foi renovada.",
  "vaultKey.doneAttachments":
    "A chave do cofre foi renovada, e as chaves dos anexos também.",
  "vaultKey.unfinished":
    "A chave do cofre foi renovada. A renovação das chaves dos anexos continua depois. Se o Mosaic for fechado antes, escolha Continuar troca de chaves na próxima vez.",
  "vaultKey.unreachable":
    "{count, plural, one {# anexo ainda não está neste dispositivo e mantém a chave antiga.} other {# anexos ainda não estão neste dispositivo e mantêm a chave antiga.}}",
  "vaultKey.close": "Fechar",
  "vault.meta.title": "Os detalhes do cofre foram encurtados",
  "vault.meta.body":
    "Dois dispositivos alteraram {fields} de “{vault}” ao mesmo tempo, e juntos ficaram longos demais para armazenar. O Mosaic manteve uma versão ou deixou o campo vazio. Confira e digite de novo se necessário.",
  "vault.edit.title": "Editar cofre",
  "vault.meta.edit": "Editar cofre",
  "vault.meta.field.name": "o nome",
  "vault.meta.field.icon": "o ícone",
  "vault.meta.field.description": "a descrição",
  "vault.meta.dismiss": "Dispensar",
} satisfies Record<keyof typeof en, string>;
