/**
 * Brazilian Portuguese UI strings: files. Same keys as English (checked by the type).
 * ICU plural categories of Brazilian Portuguese: one, many, other (CLDR; `other` is required).
 */
import type { files as en } from "../en/files.ts";

export const files = {
  "files.title": "Arquivos",
  "files.add": "Adicionar arquivo…",
  "files.empty": "Nenhum arquivo ainda.",
  "files.previewLoading": "Preparando pré-visualização…",
  "files.size.b": "{value} B",
  "files.size.kb": "{value} KB",
  "files.size.mb": "{value} MB",
  "files.addedAt": "Adicionado em {at, date, medium}",
  "files.preview": "Pré-visualizar {name}",
  "files.saveAs": "Salvar {name} como…",
  "files.saveAsShort": "Salvar como…",
  "files.remove": "Remover {name}",
  "files.close": "Fechar",
  "files.previewAlt": "Pré-visualização de {name}",
  "files.previewOnlyImages":
    "A pré-visualização funciona para imagens PNG e JPEG. Use Salvar como para abrir outros arquivos.",
  "files.notDownloaded": "Este arquivo ainda não está neste dispositivo.",
  "files.editorNote": "Os arquivos são salvos assim que você os adiciona ou remove.",
  "files.removeConfirm.title": "Remover {name}?",
  "files.removeConfirm.body":
    "O arquivo é excluído deste item em todos os dispositivos que o sincronizam. Isto não pode ser desfeito.",
  "files.removeConfirm.confirm": "Remover arquivo",
  "files.removeConfirm.keep": "Manter arquivo",
  "toast.fileAdded": "Arquivo adicionado.",
  "toast.fileSaved": "Arquivo salvo.",
  "toast.fileRemoved": "Arquivo removido.",
} satisfies Record<keyof typeof en, string>;
