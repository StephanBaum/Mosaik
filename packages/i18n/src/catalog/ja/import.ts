/**
 * Japanese UI strings: import. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { importMessages as en } from "../en/import.ts";

export const importMessages = {
  "import.format.enpass": "Enpass (.json)",
  "import.format.bitwarden": "Bitwarden (.json、暗号化なし)",
  "import.format.keepass": "KeePass (.kdbx)",
  "import.format.onePassword": "1Password (.1pux)",
  "import.format.cxf": "FIDO Credential Exchange (.json): パスキーとパスワード",
  "import.format.dashlane": "Dashlane (.zip)",
  "import.format.keeper": "Keeper (.json)",
  "import.format.keeperCsv": "Keeper (.csv)",
  "import.format.protonPass": "Proton Pass (.zip または .json、暗号化なし)",
  "import.format.csv": "CSV ファイル (.csv)",
  "import.keepassPassword": "KeePass データベースのパスワード",
  "import.preview.ready": "{count, plural, other {#件の項目を読み込めます。}}",
  "import.preview.attachments": "{count, plural, other {#件の添付ファイル}}",
  "import.preview.duplicate":
    "これと同じ項目がすでにあります。二重に読み込まれないよう、チェックを外してあります。",
  "import.preview.duplicateInFile": "このファイル内に同じ項目が2つあります。コピーはチェックを外してあります。",
  "import.warning.unmappedFields":
    "Mosaic に対応するフィールドがないものがあります。カスタムフィールドとして保持されました。",
  "import.warning.attachmentsSkipped":
    "50 MiB を超える、1 GiB の添付ファイル保存容量に収まらない、またはエクスポートに含まれていない添付ファイルがあります。読み込み後に手動で追加してください。",
  "import.warning.entriesSkipped":
    "読み取れなかったエントリがあり、除外されます。以前のパスワードマネージャーで確認してください。",
  "import.warning.otpUnreadable":
    "認識できなかったワンタイムコードの設定があります。非表示フィールドとして保持されたので、そこからコードをもう一度設定してください。",
  "import.warning.otpWeak":
    "シークレットが80ビットより短いワンタイムコードがあり、コードが推測されやすくなっています。そのまま読み込まれました。可能なときに、新しいシークレットで設定し直してください。",
  "import.warning.passkeysNotImported":
    "このエクスポート形式にはパスキーの秘密鍵が含まれていないため、読み込めません。それらの項目のその他の内容は読み込まれます。",
  "import.warning.passkeyUnreadable": "読み取れなかったパスキーがあり、除外されました。",
  "import.warning.passkeyUnsupported":
    "Mosaic が対応していない鍵タイプ（ES256 のみ対応）のパスキーがあり、除外されました。",
  "import.warning.passkeysSkipped":
    "数が多すぎるため、除外されたパスキーがあります（1項目あたり最大100個、1ファイルあたり最大10,000個）。",
  "import.warning.passkeysKeepOldApp": "パスキーを移し終えるまで、以前のアプリは残しておいてください。",
  "import.entry.unmappedFields":
    "Mosaic に対応するフィールドがないものがあります。カスタムフィールドとして保持されました。",
  "import.entry.otpUnreadable":
    "ワンタイムコードの設定を認識できませんでした。非表示フィールドとして保持されたので、そこからコードをもう一度設定してください。",
  "import.entry.otpWeak":
    "ワンタイムコードのシークレットが80ビットより短く、コードが推測されやすくなっています。そのまま読み込まれました。可能なときに、新しいシークレットで設定し直してください。",
  "import.entry.attachmentTooLarge":
    "50 MiB を超える添付ファイルがあり、除外されました。読み込み後に手動で追加してください。",
  "import.entry.attachmentMissing":
    "エクスポートに含まれていない添付ファイルがあります。読み込み後に手動で追加してください。",
  "import.entry.attachmentQuota":
    "1 GiB の添付ファイル保存容量に収まらない添付ファイルがあり、除外されました。",
  "import.entry.passkeysNotImported":
    "このエクスポート形式にはこの項目のパスキーの秘密鍵が含まれていないため、読み込めません。移し終えるまで、以前のアプリは残しておいてください。",
  "import.entry.passkeyUnreadable":
    "この項目のパスキーを読み取れず、除外されました。移し終えるまで、以前のアプリは残しておいてください。",
  "import.entry.passkeyUnsupported":
    "この項目のパスキーは Mosaic が対応していない鍵タイプ（ES256 のみ対応）のため、除外されました。移し終えるまで、以前のアプリは残しておいてください。",
  "import.entry.historyTruncated": "古いパスワードは、最新の32件のみ保持されました。",
  "import.entry.tagsDropped": "空のタグや長すぎるタグは除外されました。",
  "import.entry.archived": "以前のアプリではアーカイブ済みです。「archived」タグが付きます。",
  "import.entry.unknownCategory": "不明な項目タイプです。セキュアノートとして読み込まれます。",
  "import.entry.titleCleaned":
    "タイトルに、不可視文字またはテキストの向きを反転させる文字が含まれていたため、削除されました。",
  "import.entry.passkeysLimit":
    "この項目のパスキーが、1回の読み込みで扱える数（1項目あたり100個、1ファイルあたり10,000個）を超えています。超過分のパスキーは除外されました。移し終えるまで、以前のアプリは残しておいてください。",
  "import.entry.fieldsDropped":
    "この項目には、Mosaic が保持できる数より多くのフィールドがあります。超過分のフィールドは除外されます。",
  "import.entry.notesTruncated":
    "この項目のメモは、1つの項目に収まる長さを超えています。収まらなかった部分は除外されました。",
  "import.entry.idnHost":
    "このアドレスはドメイン名に特殊な文字を使っており、別のサイトに見える可能性があります。信頼する前に確認してください。",
  "import.entry.urlKeptAsText": "このウェブアドレスはリンクとして使用できませんでした。テキストとして保持されました。",
  "import.skip.malformed": "このエントリは破損しており、読み取れません。",
  "import.skip.empty": "このエントリは空です。",
  "import.skip.tooLarge": "このエントリは、1つの項目に収まる上限の 1 MiB を超えています。",
  "import.result":
    "{imported, plural, other {#件の項目を読み込みました。}} {skipped, plural, =0 {} other {#件は除外されました。}}",
  "import.flow.title": "読み込み",
  "import.flow.formatStep": "どこから読み込みますか？",
  "import.flow.chooseFile": "エクスポートしたファイルを選択",
  "import.flow.fileChosen": "ファイル: {name}",
  "import.flow.password": "KeePass データベースのパスワード",
  "import.flow.preview": "プレビュー",
  "import.flow.previewBusy": "ファイルを読み取っています。少し時間がかかる場合があります。",
  "import.flow.selectAll": "すべて選択",
  "import.flow.target": "読み込み先",
  "import.flow.commit": "{count, plural, other {#件の項目}}を読み込む",
  "import.flow.entries": "エントリ",
  "import.flow.skipped": "除外",
  "import.flow.attachments": "{count, plural, other {#件の添付ファイル}}",
  "import.flow.cancel": "読み込みをキャンセル",
  "import.flow.noneSelected": "読み込む項目を1つ以上選択してください。",
  "import.flow.resultFailed": "{failed, plural, =0 {} other {#件の項目を保存できませんでした。}}",
  "import.flow.deleteReminder":
    "エクスポートしたファイルは今すぐ削除してください。パスワードが平文で含まれています。",
  "import.flow.dropHint":
    "エクスポートしたファイルを選択してください。すべてこのデバイス上で読み取られ、暗号化されます。",
} satisfies Record<keyof typeof en, string>;
