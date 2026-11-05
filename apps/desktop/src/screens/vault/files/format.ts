/** Display helpers for attachments: sizes through the catalog and an icon per file type. */
import type { MessageKey, MessageValues } from "@mosaic/i18n";
import type { IconName } from "../../../components/icons.tsx";

type Translate = (key: MessageKey, values?: MessageValues) => string;

const KIB = 1024;

/** "812 B", "3.4 KB", "1.2 MB" (base 1024, at most one decimal). */
export function formatSize(bytes: number, t: Translate, locale?: string): string {
  const number = new Intl.NumberFormat(locale, { maximumFractionDigits: 1 });
  if (bytes < KIB) {
    return t("files.size.b", { value: number.format(bytes) });
  }
  if (bytes < KIB * KIB) {
    return t("files.size.kb", { value: number.format(bytes / KIB) });
  }
  return t("files.size.mb", { value: number.format(bytes / (KIB * KIB)) });
}

/** The icon of a file type; the MIME type is a hint from the file name or its first bytes. */
export function iconFor(mime: string): IconName {
  if (mime.startsWith("image/")) {
    return "image";
  }
  if (mime === "application/pdf" || mime.startsWith("text/") || mime === "application/json") {
    return "fileText";
  }
  return "file";
}
