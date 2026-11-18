/**
 * Brazilian Portuguese UI strings: import. Same keys as English (checked by the type).
 * ICU plural categories of Brazilian Portuguese: one, many, other (CLDR; `other` is required).
 */
import type { importMessages as en } from "../en/import.ts";

export const importMessages = {
  "import.format.enpass": "Enpass (.json)",
  "import.format.bitwarden": "Bitwarden (.json, sem criptografia)",
  "import.format.keepass": "KeePass (.kdbx)",
  "import.format.onePassword": "1Password (.1pux)",
  "import.format.cxf": "FIDO Credential Exchange (.json): chaves de acesso e senhas",
  "import.format.dashlane": "Dashlane (.zip)",
  "import.format.keeper": "Keeper (.json)",
  "import.format.keeperCsv": "Keeper (.csv)",
  "import.format.protonPass": "Proton Pass (.zip ou .json, sem criptografia)",
  "import.format.csv": "Arquivo CSV (.csv)",
  "import.keepassPassword": "Senha do banco de dados do KeePass",
  "import.preview.ready":
    "{count, plural, one {# item está pronto para importar.} other {# itens estão prontos para importar.}}",
  "import.preview.attachments": "{count, plural, one {# anexo} other {# anexos}}",
  "import.preview.duplicate":
    "Você já tem um item assim. Ele está desmarcado, para não ser importado duas vezes.",
  "import.preview.duplicateInFile":
    "Este arquivo tem o mesmo item duas vezes. A cópia está desmarcada.",
  "import.warning.unmappedFields":
    "Alguns campos não têm um campo correspondente no Mosaic. Eles foram mantidos como campos personalizados.",
  "import.warning.attachmentsSkipped":
    "Alguns anexos são maiores que 50 MiB, não cabem no seu 1 GiB de armazenamento de anexos ou estão ausentes da exportação. Adicione-os manualmente após a importação.",
  "import.warning.entriesSkipped":
    "Algumas entradas não puderam ser lidas e serão omitidas. Confira-as no seu gerenciador de senhas antigo.",
  "import.warning.otpUnreadable":
    "Algumas configurações de código de uso único não foram reconhecidas. Elas foram mantidas como campos ocultos; configure o código de novo a partir delas.",
  "import.warning.otpWeak":
    "Alguns códigos de uso único têm um segredo menor que 80 bits, então são mais fáceis de adivinhar. Foram importados mesmo assim; configure-os de novo com um novo segredo quando puder.",
  "import.warning.passkeysNotImported":
    "Este formato de exportação não contém as chaves privadas das chaves de acesso, então elas não podem ser importadas. O restante desses itens é importado.",
  "import.warning.passkeyUnreadable":
    "Algumas chaves de acesso não puderam ser lidas e foram omitidas.",
  "import.warning.passkeyUnsupported":
    "Algumas chaves de acesso usam um tipo de chave que o Mosaic não oferece suporte (só ES256 funciona) e foram omitidas.",
  "import.warning.passkeysSkipped":
    "Algumas chaves de acesso foram omitidas porque há demais (no máximo 100 por item e 10.000 por arquivo).",
  "import.warning.passkeysKeepOldApp":
    "Mantenha o seu app antigo até mover as chaves de acesso.",
  "import.entry.unmappedFields":
    "Alguns campos não têm um campo correspondente no Mosaic. Eles foram mantidos como campos personalizados.",
  "import.entry.otpUnreadable":
    "A configuração do código de uso único não foi reconhecida. Ela foi mantida como um campo oculto; configure o código de novo a partir dela.",
  "import.entry.otpWeak":
    "O segredo do código de uso único é menor que 80 bits, então o código é mais fácil de adivinhar. Foi importado mesmo assim; configure-o de novo com um novo segredo quando puder.",
  "import.entry.attachmentTooLarge":
    "Um anexo é maior que 50 MiB e foi omitido. Adicione-o manualmente após a importação.",
  "import.entry.attachmentMissing":
    "Um anexo está ausente da exportação. Adicione-o manualmente após a importação.",
  "import.entry.attachmentQuota":
    "Um anexo não cabe no seu 1 GiB de armazenamento de anexos e foi omitido.",
  "import.entry.passkeysNotImported":
    "Este formato de exportação não contém a chave privada da chave de acesso deste item, então ela não pode ser importada. Mantenha o seu app antigo até movê-la.",
  "import.entry.passkeyUnreadable":
    "Uma chave de acesso deste item não pôde ser lida e foi omitida. Mantenha o seu app antigo até movê-la.",
  "import.entry.passkeyUnsupported":
    "Uma chave de acesso deste item usa um tipo de chave que o Mosaic não oferece suporte (só ES256 funciona) e foi omitida. Mantenha o seu app antigo até movê-la.",
  "import.entry.historyTruncated": "Apenas as 32 senhas antigas mais recentes foram mantidas.",
  "import.entry.tagsDropped": "Etiquetas vazias ou longas demais foram omitidas.",
  "import.entry.archived": "Arquivado no seu app antigo. Recebe a etiqueta “archived”.",
  "import.entry.unknownCategory": "Tipo de item desconhecido. Ele é importado como uma nota segura.",
  "import.entry.titleCleaned":
    "O título tinha caracteres invisíveis ou que invertem o texto. Eles foram removidos.",
  "import.entry.passkeysLimit":
    "Este item tem mais chaves de acesso do que uma importação aceita (100 por item, 10.000 por arquivo). As chaves de acesso excedentes foram omitidas. Mantenha o seu app antigo até movê-las.",
  "import.entry.fieldsDropped":
    "Este item tem mais campos do que o Mosaic guarda. Os campos excedentes são omitidos.",
  "import.entry.notesTruncated":
    "As notas deste item são mais longas do que um item comporta. A parte que não coube foi omitida.",
  "import.entry.idnHost":
    "Este endereço usa caracteres especiais no nome de domínio e pode se parecer com outro site. Confira-o antes de confiar nele.",
  "import.entry.urlKeptAsText":
    "Este endereço da web não pôde ser usado como link. Ele foi mantido como texto.",
  "import.skip.malformed": "Esta entrada está danificada e não pode ser lida.",
  "import.skip.empty": "Esta entrada está vazia.",
  "import.skip.tooLarge": "Esta entrada é maior que 1 MiB, o máximo que um item comporta.",
  "import.result":
    "{imported, plural, one {# item importado.} other {# itens importados.}} {skipped, plural, =0 {} one {# omitido.} other {# omitidos.}}",
  "import.flow.title": "Importar",
  "import.flow.formatStep": "De onde você está importando?",
  "import.flow.chooseFile": "Escolha o arquivo de exportação",
  "import.flow.fileChosen": "Arquivo: {name}",
  "import.flow.password": "Senha do banco de dados do KeePass",
  "import.flow.preview": "Pré-visualização",
  "import.flow.previewBusy": "Lendo o arquivo. Isto pode levar um momento.",
  "import.flow.selectAll": "Selecionar tudo",
  "import.flow.target": "Importar para",
  "import.flow.commit": "Importar {count, plural, one {# item} other {# itens}}",
  "import.flow.entries": "Entradas",
  "import.flow.skipped": "Omitidos",
  "import.flow.attachments": "{count, plural, one {# anexo} other {# anexos}}",
  "import.flow.cancel": "Cancelar importação",
  "import.flow.noneSelected": "Selecione pelo menos um item para importar.",
  "import.flow.resultFailed":
    "{failed, plural, =0 {} one {# item não pôde ser salvo.} other {# itens não puderam ser salvos.}}",
  "import.flow.deleteReminder":
    "Exclua o arquivo de exportação agora. Ele guarda as suas senhas em texto simples.",
  "import.flow.dropHint":
    "Escolha o seu arquivo de exportação. Tudo é lido e criptografado neste dispositivo.",
} satisfies Record<keyof typeof en, string>;
