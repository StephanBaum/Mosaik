/**
 * Japanese UI strings: mobileFolder. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { mobileFolder as en } from "../en/mobileFolder.ts";

export const mobileFolder = {
  "mobile.sync.never": "まだ同期していません",
  "mobile.sync.last": "同期日時: {time}",
  "mobile.sync.localOnly": "この保管庫はこのスマートフォンにのみあります。同期は設定されていません。",
  "mobile.sync.pending": "{count, plural, other {#件の変更がアップロード待ちです}}",
  "mobile.sync.now": "今すぐ同期",
  "mobile.sync.working": "同期中",
  "mobile.sync.warnings": "同期の警告",
  "mobile.sync.warning.freezeSuspected":
    "他のデバイスからの変更がしばらく届いていません。他のデバイスの電源が入っていて、オンラインになっているか確認してください。",
  "mobile.sync.warning.noCommonView":
    "このスマートフォンと他のデバイスが、2日間、同じサーバーデータを確認できていません。サーバーが異なるバージョンを表示している可能性があります。先に他のデバイスで同期してから、こちらで同期してください。",
  "mobile.sync.warning.rollbackDetected":
    "同期データがこのスマートフォンより古いようです。他のデバイスで Mosaic を開き、最新の変更があるか確認してください。その後、「確認しました」を選んでください。",
  "mobile.sync.warning.identityDuplicated":
    "バックアップの復元後などに、別の Mosaic がこのスマートフォンの ID で書き込みを行っています。同期は一時停止中です。使用しているコピーが1つだけであることを確認し、「確認しました」を選んでください。",
  "mobile.sync.warning.serverInconsistent":
    "サーバーが、このスマートフォンと他のデバイスに異なるデータを表示しました。他のデバイスで Mosaic を開き、最新の変更があるか確認してください。その後、「確認しました」を選んでください。",
  "mobile.sync.warning.serverRollback":
    "サーバーにある保管庫のコピーの1つが、このスマートフォンのものより古くなっています。他のデバイスで Mosaic を開き、最新の変更があるか確認してください。その後、「確認しました」を選んでください。",
  "mobile.sync.warning.sharingAttention":
    "共有保管庫の確認が必要です。鍵の変更が待機中か、メンバーを確認できませんでした。どちらかは、パソコンで Mosaic を開くと確認できます。その後、「確認しました」を選んでください。",
  "mobile.sync.warning.undecryptable":
    "受信した{count, plural, other {#件の項目}}を、このスマートフォンでは読み取れませんでした。すべてのデバイスで Mosaic を更新してから、もう一度同期してください。",
  "mobile.sync.warning.accountRollback":
    "サーバーが、このスマートフォンにあるものより古いアカウントの鍵を表示しています。何も変更されていません。他のデバイスで Mosaic を開き、最新の変更があるか確認してください。その後、「確認しました」を選んでください。",
  "mobile.sync.acknowledge": "確認しました…",
  "mobile.sync.acknowledgeTitle": "同期の警告を消去しますか？",
  "mobile.sync.acknowledgeBody":
    "ご自身でサーバーを確認してから進めてください。すべての警告が消去され、同期が再開されます。データが実際に巻き戻された、または改ざんされていた場合でも、Mosaic には後から判断できません。この操作は、お使いのデバイスごとに行ってください。",
  "mobile.sync.acknowledgeConfirm": "確認しました",
  "mobile.folder.intro": "このアカウントは、{provider} のフォルダ経由で同期します。",
  "mobile.folder.signIn": "{provider} にサインイン",
  "mobile.folder.scope":
    "{provider} は、すべてのファイルへのアクセスを求めます。Mosaic は選択したフォルダのみを開き、それ以外には一切触れません。",
  "mobile.folder.waiting": "ブラウザでサインインを完了してから、ここに戻ってください。",
  "mobile.folder.serverLabel": "Nextcloud または WebDAV のアドレス",
  "mobile.folder.serverHint": "https://cloud.example.com",
  "mobile.folder.serverConfirm":
    "Mosaic は {server} にサインインします。ご自身のサーバーである場合にのみ続けてください。",
  "mobile.folder.locating": "Mosaic のフォルダを探しています…",
  "mobile.folder.browseTitle": "Mosaic のフォルダを選択",
  "mobile.folder.browseHint":
    "パソコンが同期しているフォルダで、mosaic-sync.json というファイルがあります。",
  "mobile.folder.currentFolder": "現在のフォルダ: {path}",
  "mobile.folder.noSubfolders": "ここにはフォルダがありません。",
  "mobile.folder.useThis": "このフォルダを使用",
  "mobile.folder.up": "1つ上へ",
  "mobile.folder.chooseProvider": "アカウントはどこで同期しますか？",
  "mobile.folder.noDrive":
    "Google ドライブと iCloud Drive は、まだ Android では使えません。Dropbox、OneDrive、Nextcloud、または Mosaic の同期を使用してください。",
  "mobile.folder.useFolder": "代わりにクラウドフォルダで同期",
  "mobile.folder.providerDropbox": "Dropbox",
  "mobile.folder.providerOneDrive": "OneDrive",
  "mobile.folder.providerWebDav": "Nextcloud または WebDAV",
  "mobile.folder.appPasswordToggle": "代わりにアプリパスワードを使用",
  "mobile.folder.davLabel": "WebDAV フォルダのアドレス",
  "mobile.folder.loginLabel": "ユーザー名",
  "mobile.folder.appPasswordLabel": "アプリパスワード",
  "mobile.folder.appPasswordHint":
    "サーバーのセキュリティ設定でアプリパスワードを作成してください。メインのパスワードは使用しないでください。",
  "mobile.folder.appPasswordAction": "アプリパスワードでサインイン",
  "mobile.folder.pickAccount": "このスマートフォンはどのアカウントに参加しますか？",
  "mobile.folder.accountOption": "アカウント {id}",
  "mobile.folder.linked": "サインインしました。同期を再開します。",
  "mobile.folder.settingsLine": "{provider} 経由で同期: {path}",
  "mobile.folder.relink": "もう一度サインイン",
  "mobile.folder.msRevokeHint":
    "Mosaic のアクセスを完全に削除するには、Microsoft アカウントのページの「アプリ」から削除してください。",
  "mobile.reset.revokeDropbox":
    "アカウントを削除しました。Dropbox での Mosaic のアクセスを終了できませんでした。Dropbox の「設定」の「接続済みアプリ」から削除してください。",
  "mobile.reset.revokeWebdav":
    "アカウントを削除しました。サーバー上のアプリパスワードを削除できませんでした。サーバーの「設定」、「セキュリティ」、「デバイスとセッション」から削除してください。",
  "mobile.reset.title": "このスマートフォンからアカウントを削除",
  "mobile.reset.body":
    "このスマートフォンに保存されている保管庫のコピー、シークレットキー、クラウドへのサインイン情報を削除します。クラウドフォルダやサーバーにあるデータは残ります。後からシークレットキーとマスターパスワードで、もう一度参加できます。",
  "mobile.reset.action": "アカウントを削除…",
  "mobile.reset.confirm": "アカウントを削除",
  "mobile.reset.phraseLabel": "確認",
  "mobile.reset.typePhrase": "確認のため {phrase} と入力してください",
} satisfies Record<keyof typeof en, string>;
