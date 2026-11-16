/**
 * Japanese UI strings: otp. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { otp as en } from "../en/otp.ts";

export const otp = {
  "otp.section.label": "ワンタイムコード（2FA）",
  "otp.section.empty": "サイトの2段階認証の設定ページに表示されるコードを追加してください。",
  "otp.section.required": "この項目を保存するには、ワンタイムコードを追加してください。",
  "otp.source.screen": "画面をスキャン",
  "otp.source.clipboard": "スクリーンショットを貼り付け",
  "otp.source.file": "画像を選択…",
  "otp.source.manual": "スキャンできない場合はキーを入力",
  "otp.source.screenHint":
    "Mosaic が一時的に身を引いて画面を1回だけ確認し、コード以外は何も保持しません。",
  "otp.source.clipboardHint":
    "{platform, select, mac {Cmd+Ctrl+Shift+4 を押して QR コードを選択し、ここに貼り付けてください。} windows {Win+Shift+S を押して QR コードを選択し、ここに貼り付けてください。} other {QR コードのスクリーンショットをコピーして、ここに貼り付けてください。}}",
  "otp.dialog.title": "ワンタイムコードを設定",
  "otp.working.screen": "画面上の QR コードを探しています…",
  "otp.working.image": "QR コードを読み取っています…",
  "otp.manual.title": "セットアップキーを入力",
  "otp.manual.input": "セットアップキーまたは otpauth:// リンク",
  "otp.manual.inputHint":
    "サイトの QR コードの近くに、多くは「スキャンできない場合」や「手動で入力」の下に表示されています。スペースは無視されます。",
  "otp.manual.issuer": "サービス",
  "otp.manual.account": "アカウント",
  "otp.manual.advanced": "詳細設定",
  "otp.manual.algorithm": "アルゴリズム",
  "otp.manual.digits": "桁数",
  "otp.manual.period": "新しいコードの間隔",
  "otp.manual.periodValue": "{seconds} 秒",
  "otp.manual.advancedHint":
    "サイトから指示がある場合にのみ変更してください。ほとんどのサイトは SHA-1、6桁、30秒を使っています。",
  "otp.manual.continue": "続ける",
  "otp.confirm.title": "このコードで合っていますか？",
  "otp.confirm.issuer": "サービス",
  "otp.confirm.account": "アカウント",
  "otp.confirm.details":
    "{scheme, select, hotp {カウンター方式、{digits}桁} steam {Steam Guard、5文字} other {{digits}桁、{period}秒ごとに新しいコード}}",
  "otp.confirm.algorithmValue":
    "{algorithm, select, sha256 {SHA-256} sha512 {SHA-512} other {SHA-1}}",
  "otp.confirm.codeLabel": "現在のコード",
  "otp.confirm.codeHint": "設定を完了するためにサイトからコードを求められたら、これを入力してください。",
  "otp.confirm.hotpNote":
    "カウンター方式のコードは、サイトとカウンターがずれないよう、保存後に表示されます。",
  "otp.confirm.weak":
    "このシークレットは通常より短いです。使用はできますが、このサイトの2段階認証の保護は通常より弱くなります。",
  "otp.confirm.use": "このコードを使う",
  "otp.confirm.back": "もう一度スキャン",
  "otp.confirm.copy": "コードをコピー",
  "otp.summary.noIssuer": "ワンタイムコード",
  "otp.summary.pending": "この項目と一緒に保存されます。",
  "otp.summary.removed": "保存すると、ワンタイムコードは削除されます。",
  "otp.action.replace": "置き換える",
  "otp.action.remove": "ワンタイムコードを削除",
  "otp.action.keep": "残す",
  "otp.action.cancelSetup": "追加しない",
  "otp.storage.label": "コードの保管場所",
  "otp.storage.item": "この項目内",
  "otp.storage.itemTradeoff":
    "便利ですが、この保管庫を開ける人は、パスワードと2要素目の両方を入手できてしまいます。",
  "otp.storage.hardware": "{vault} 内（セキュリティキーが必要）",
  "otp.storage.hardwareClosed": "先にセキュリティキーで {vault} を開いてください。",
  "otp.toast.clipboardCleared": "スクリーンショットを読み取りました。Mosaic がクリップボードを消去しました。",
  "otp.toast.savedInVault": "コードを {vault} に保存しました。",
  "otp.clipboardCopies":
    "{platform, select, windows {クリップボード履歴や Snipping Tool にスクリーンショットのコピーが残っている場合は、そこから削除してください。} mac {スクリーンショットをファイルとしても保存した場合は、削除してください。} other {クリップボードマネージャーにスクリーンショットのコピーが残っている場合は、そこから削除してください。}}",
} satisfies Record<keyof typeof en, string>;
