/**
 * Japanese UI strings: browser. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { browser as en } from "../en/browser.ts";

export const browser = {
  "browser.pair.title": "ブラウザが Mosaic への接続を求めています",
  "browser.pair.description":
    "Mosaic 拡張機能で「接続」をクリックしたばかりで、このコードが表示されている場合にのみ許可してください。",
  "browser.pair.code": "コード",
  "browser.pair.browser": "ブラウザ",
  "browser.pair.label": "このブラウザの名前",
  "browser.pair.labelHint": "この名前は設定に表示されます。",
  "browser.pair.deny": "拒否",
  "browser.pair.allow": "許可",
  "browser.expires": "{seconds, plural, other {#秒}}後に自動で閉じます",
  "browser.escDenies": "で拒否",
  "browser.hint.chrome": "Chrome",
  "browser.hint.edge": "Edge",
  "browser.hint.brave": "Brave",
  "browser.hint.opera": "Opera",
  "browser.hint.vivaldi": "Vivaldi",
  "browser.hint.chromium": "Chromium",
  "browser.hint.firefox": "Firefox",
  "browser.hint.other": "ブラウザ",
  "browser.family.chromium": "Chromium ベースのブラウザ",
  "browser.family.firefox": "Firefox",
  "browser.escCancels": "でキャンセル",
  "browser.save.titleNew": "このログインを Mosaic に保存しますか？",
  "browser.save.titleUpdate": "Mosaic のこのログインを更新しますか？",
  "browser.save.description":
    "{browser} がログインの保存を提案しています。Mosaic は「保存」を選んだ場合にのみ書き込みます。",
  "browser.save.descriptionUpdate":
    "{browser} がこのログインの新しいパスワードを提案しています。Mosaic は「パスワードを更新」を選んだ場合にのみ変更します。",
  "browser.save.site": "ウェブサイト",
  "browser.save.username": "ユーザー名",
  "browser.save.noUsername": "なし",
  "browser.save.password": "パスワード",
  "browser.save.passwordHidden": "非表示",
  "browser.save.target": "保存先",
  "browser.save.updateOption": "「{title}」を更新",
  "browser.save.newOption": "新しいログイン",
  "browser.save.itemTitle": "タイトル",
  "browser.save.vault": "保管庫",
  "browser.save.allowHttp": "この暗号化されていないページでの入力を許可",
  "browser.save.allowHttpHint":
    "このサイトは HTTPS を使用していません。同じネットワーク上の誰でも、入力内容を読み取れる可能性があります。",
  "browser.save.cancel": "キャンセル",
  "browser.save.saveNew": "保存",
  "browser.save.update": "パスワードを更新",
  "browser.passkey.secure": "安全な接続（https）",
  "browser.passkey.getTitle": "パスキーで {rpId} にサインイン",
  "browser.passkey.createTitle": "新しいパスキーを保存",
  "browser.passkey.excludedTitle": "パスキーはすでに保存されています",
  "browser.passkey.createBody": "{rpId} のパスキーを Mosaic に作成しますか？",
  "browser.passkey.excludedBody": "{rpId} のパスキーはすでに Mosaic にあります。",
  "browser.passkey.uvNote":
    "サイトからユーザー確認を求められています。本人確認のため、マスターパスワードを入力してください。",
  "browser.passkey.password": "マスターパスワード",
  "browser.passkey.account": "アカウント",
  "browser.passkey.noName": "名前なし",
  "browser.passkey.vault": "保管庫",
  "browser.passkey.signIn": "パスキーでサインイン",
  "browser.passkey.save": "パスキーを保存",
  "browser.passkey.anotherDevice": "別のデバイスまたはセキュリティキーを使用",
  "browser.passkey.cancel": "キャンセル",
  "browser.passkey.ok": "OK",
  "browser.passkey.escCloses": "で閉じる",
  "browser.passkey.passwordNeeded": "本人確認のため、マスターパスワードを入力してください。",
  "browser.settings.title": "ブラウザ",
  "browser.settings.body":
    "このアプリからログインを入力するブラウザ拡張機能です。接続のたびに、ここで承認します。",
  "browser.settings.empty":
    "接続済みのブラウザはまだありません。Mosaic 拡張機能をインストールし、その中で「接続」を選んでください。",
  "browser.settings.connected": "現在接続中",
  "browser.settings.lastUsed": "最終使用: {when}",
  "browser.settings.neverUsed": "未使用",
  "browser.settings.disconnect": "接続を解除",
  "browser.settings.disconnectNamed": "{name} の接続を解除",
  "browser.settings.disconnectTitle": "「{name}」の接続を解除しますか？",
  "browser.settings.disconnectBody":
    "拡張機能からもう一度接続するまで、このブラウザはログインを入力できなくなります。",
  "browser.settings.endpointInUse":
    "別のプログラムまたは別の Mosaic ウィンドウが、すでにブラウザ用の接続を使用しています。それを閉じてから、Mosaic を再起動してください。",
  "browser.settings.endpointUnavailable":
    "Mosaic はブラウザ用の接続を開けませんでした。Mosaic を再起動してください。それでも解決しない場合は、ユーザーフォルダに書き込めるか確認してください。",
  "browser.settings.registryUnreadable":
    "保存されているブラウザ接続を読み取れません。これが解決するまで、Mosaic は変更しません。",
  "browser.connector.title": "ブラウザの接続先が別のものになっています",
  "browser.connector.body":
    "{browser} の接続は、Mosaic ではなく別のプログラムを起動します: {path}。古い Mosaic か、自身を登録した別のものかもしれません。「修復」を行うと、この Mosaic に向け直します。",
  "browser.connector.item": "{browser}: {path}",
  "browser.connector.repair": "修復",
  "browser.connector.repaired": "ブラウザの接続を修復しました。",
} satisfies Record<keyof typeof en, string>;
