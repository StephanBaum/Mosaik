/**
 * Japanese UI strings: sync. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { sync as en } from "../en/sync.ts";

export const sync = {
  "sync.status.signedOut": "サインアウト済み",
  "sync.platform.windows": "Windows",
  "sync.platform.macos": "macOS",
  "sync.platform.linux": "Linux",
  "settings.sync.title": "同期",
  "settings.sync.body":
    "すべてのデバイスで保管庫を最新の状態に保ちます。すべての内容は、このデバイスから送信される前に暗号化されます。",
  "settings.sync.notSetUp": "同期はオフです。保管庫はこのデバイスにのみ保存されます。",
  "settings.sync.deviceName": "このデバイスの名前",
  "settings.sync.deviceName.default": "{platform} 上の Mosaic",
  "settings.sync.password": "マスターパスワード",
  "settings.sync.code": "6桁のコード",
  "settings.sync.server.title": "Mosaic サーバー",
  "settings.sync.server.body": "ご自身または信頼できる人が運用する Mosaic サーバーで同期します。",
  "settings.sync.server.url": "サーバーアドレス",
  "settings.sync.server.urlHint": "例: https://sync.example.com",
  "settings.sync.server.check": "サーバーを確認",
  "settings.sync.server.checking": "サーバーを確認しています…",
  "settings.sync.server.registration.open": "このサーバーでは、誰でも同期アカウントを作成できます。",
  "settings.sync.server.registration.invite":
    "このサーバーで同期アカウントを作成するには、招待コードが必要です。",
  "settings.sync.server.registration.closed":
    "このサーバーは新しい同期アカウントを受け付けていません。既存のアカウントにはサインインできます。",
  "settings.sync.server.choice.label": "同期アカウント",
  "settings.sync.server.choice.signIn": "同期アカウントを持っています",
  "settings.sync.server.choice.create": "新規作成",
  "settings.sync.server.invite": "招待コード",
  "settings.sync.server.create": "同期アカウントを作成",
  "settings.sync.server.signIn": "サインイン",
  "settings.sync.server.signInAgain": "もう一度サインイン",
  "settings.sync.server.working": "接続しています…",
  "settings.sync.server.sees":
    "サーバーが把握するのは、暗号化されたデータ、アカウントのメールアドレス、同期した日時のみです。マスターパスワードやシークレットキーが知られることはありません。",
  "settings.sync.twoFactor.title": "コードを入力",
  "settings.sync.twoFactor.body":
    "この同期アカウントでは2段階認証が使われています。認証アプリに表示される6桁のコードを入力してください。",
  "settings.sync.twoFactor.submit": "確認",
  "settings.sync.folder.title": "ご自身のクラウドフォルダ",
  "settings.sync.folder.body":
    "クラウドサービス（Dropbox、OneDrive、iCloud Drive など）が最新に保つフォルダ経由で同期します。",
  "settings.sync.folder.choose": "フォルダを選択…",
  "settings.sync.folder.chooseOther": "別のフォルダを選択…",
  "settings.sync.folder.path": "フォルダ",
  "settings.sync.folder.contents.empty": "フォルダは空です。Mosaic で使用できます。",
  "settings.sync.folder.contents.mosaic": "このフォルダには、すでに Mosaic の同期データがあります。",
  "settings.sync.folder.contents.other":
    "このフォルダには他のファイルがあります。Mosaic はそこに何も書き込みません。空のフォルダを選ぶか、新しく作成してください。",
  "settings.sync.folder.connect": "フォルダを接続",
  "settings.sync.folder.working": "接続しています…",
  "settings.sync.folder.sees":
    "クラウドプロバイダーやフォルダにアクセスできる人が把握するのは、暗号化されたデータ、アカウント ID、ファイルが変更された日時のみです。マスターパスワードやシークレットキーが知られることはありません。",
  "settings.sync.status.label": "ステータス",
  "settings.sync.lastSynced": "最終同期: {when}",
  "settings.sync.neverSynced": "まだ同期していません",
  "settings.sync.offline":
    "オフラインです。変更はこのデバイスに保存されており、オンラインに戻ると同期されます。",
  "settings.sync.now": "今すぐ同期",
  "settings.sync.backend.server": "サーバー",
  "settings.sync.backend.account": "同期アカウント",
  "settings.sync.backend.folder": "フォルダ",
  "settings.sync.backend.device": "このデバイス",
  "settings.sync.signedOut.title": "このデバイスはサインアウトされています",
  "settings.sync.signedOut.body":
    "サーバーがこのデバイスのセッションを終了しました。同期を続けるには、もう一度サインインしてください。変更はこのデバイスに安全に保存されています。",
  "settings.sync.recover.title": "マスターパスワードが変更されました",
  "settings.sync.recover.body":
    "サーバーまたはフォルダに、このデバイスより新しいマスターパスワードがあります。このデバイスを最新の状態にするため、新しいマスターパスワードを入力してください。保管庫とシークレットキーは変わりません。",
  "settings.sync.recover.password": "新しいマスターパスワード",
  "settings.sync.recover.submit": "このデバイスを更新",
  "settings.sync.disconnect": "接続を解除…",
  "settings.sync.disconnect.title": "同期の接続を解除しますか？",
  "settings.sync.disconnect.body":
    "このデバイスは同期を停止し、サーバーからサインアウトします。保管庫はこのデバイスに残り、他のデバイスもそれぞれのコピーを保持します。",
  "settings.sync.disconnect.confirm": "接続を解除",
  "settings.sync.devices.title": "デバイス",
  "settings.sync.devices.thisDevice": "このデバイス",
  "settings.sync.devices.unnamed": "名前のないデバイス",
  "settings.sync.devices.lastSeen": "最終確認: {when}",
  "settings.sync.devices.neverSeen": "まだ確認されていません",
  "settings.sync.devices.rename": "名前を変更",
  "settings.sync.devices.renameNamed": "{name} の名前を変更",
  "settings.sync.devices.renameLabel": "このデバイスの新しい名前",
  "settings.sync.devices.remove": "削除",
  "settings.sync.devices.removeNamed": "{name} を削除",
  "settings.sync.devices.removeTitle": "{name} を削除しますか？",
  "settings.sync.devices.removeBody":
    "デバイスを削除すると、同期からサインアウトされます。そのデバイスがすでに持っている保管庫のコピーは削除されません。デバイスを完全に締め出すには、マスターパスワードとシークレットキーを変更してください。",
  "settings.sync.devices.removeConfirm": "デバイスを削除",
  "settings.sync.devices.folderNote":
    "デバイスを締め出すには、マスターパスワードとシークレットキーを変更してください。",
  "settings.sync.phone.title": "スマートフォンを設定",
  "settings.sync.phone.body":
    "スマートフォンでスキャンするコードを表示し、シークレットキーを入力する手間を省きます。スマートフォンでも、マスターパスワードは必要です。",
  "settings.sync.phone.button": "スマートフォンを設定",
  "settings.sync.phone.reauth":
    "このコードにはシークレットキーが含まれるため、マスターパスワードが必要です。",
  "settings.sync.phone.dialogTitle": "スマートフォンでこのコードをスキャン",
  "settings.sync.phone.steps":
    "スマートフォンで Mosaic を開いて設定を選び、このコードをスキャンしてください。",
  "settings.sync.phone.hint":
    "コードは60秒後、またはこのウィンドウを離れると非表示になります。写真を共有しないでください。",
  "settings.sync.phone.canvas": "スマートフォン用のセットアップコード",
  "settings.sync.phone.hidden.time": "60秒が経過したため、コードを非表示にしました。",
  "settings.sync.phone.hidden.away": "このウィンドウを離れたため、コードを非表示にしました。",
  "settings.sync.phone.again": "もう一度表示",
  "settings.sync.phone.folderHint":
    "スマートフォンがクラウドストレージにサインインし、同じフォルダを開きます。",
  "settings.sync.totp.title": "認証アプリ",
  "settings.sync.totp.body":
    "誰かが同期アカウントにサインインするとき、認証アプリのコードを求めます。保護するのはサーバー上のアカウントであり、保管庫ではありません。保管庫は、引き続きマスターパスワードとシークレットキーで暗号化されます。",
  "settings.sync.totp.on": "認証アプリのコードはオンです。",
  "settings.sync.totp.off": "認証アプリのコードはオフです。",
  "settings.sync.totp.enable": "オンにする…",
  "settings.sync.totp.disable": "オフにする…",
  "settings.sync.totp.beginTitle": "認証アプリのコードをオンにする",
  "settings.sync.totp.beginBody": "開始するには、マスターパスワードを入力してください。",
  "settings.sync.totp.begin": "続ける",
  "settings.sync.totp.setupBody":
    "このキーを認証アプリに追加してから、表示される6桁のコードを入力してください。",
  "settings.sync.totp.secret": "セットアップキー",
  "settings.sync.totp.confirm": "オンにする",
  "settings.sync.totp.disableTitle": "認証アプリのコードをオフにする",
  "settings.sync.totp.disableBody": "マスターパスワードと、現在のコードを入力してください。",
  "settings.sync.totp.disableConfirm": "オフにする",
  "settings.sync.totp.enabledToast": "認証アプリのコードをオンにしました。",
  "settings.sync.totp.disabledToast": "認証アプリのコードをオフにしました。",
  "settings.sync.key.title": "セキュリティキー",
  "settings.sync.key.body":
    "同期アカウントにサインインするときの2つ目のステップとして、セキュリティキーを使えます。保護するのはサーバー上のアカウントであり、保管庫ではありません。キーの追加や使用時には、ブラウザが開きます。",
  "settings.sync.key.none": "追加されたセキュリティキーはありません。",
  "settings.sync.key.added": "追加日: {when}",
  "settings.sync.key.lastUsed": "最終使用: {when}",
  "settings.sync.key.neverUsed": "未使用",
  "settings.sync.key.add": "セキュリティキーを追加…",
  "settings.sync.key.addTitle": "セキュリティキーを追加",
  "settings.sync.key.addBody":
    "キーに名前を付け、マスターパスワードを入力してください。その後、Mosaic がブラウザを開くので、そこでキーを使用します。",
  "settings.sync.key.label": "キーの名前",
  "settings.sync.key.labelHint": "例:「デスクのキー」。64文字まで。",
  "settings.sync.key.addConfirm": "続ける",
  "settings.sync.key.addedToast": "セキュリティキーを追加しました。",
  "settings.sync.key.noBackup":
    "セキュリティキーが、唯一の2つ目のステップになります。紛失すると、同期アカウントにもうサインインできなくなる可能性があります。バックアップとして、2つ目のキーまたは認証アプリのコードをおすすめします。",
  "settings.sync.key.recommend":
    "2つの方法をおすすめします。たとえば、キー2つ、またはキー1つと認証アプリのコードです。",
  "settings.sync.key.remove": "削除",
  "settings.sync.key.removeLabel": "{name} を削除",
  "settings.sync.key.removeTitle": "{name} を削除しますか？",
  "settings.sync.key.removeBody":
    "ブラウザでセキュリティキーを使って1回確認します。その後、このキーではサインインできなくなります。",
  "settings.sync.key.removeLast":
    "これは、残っている最後の2段階目の認証手段です。2段階認証がオフになり、同期アカウントはマスターパスワードとシークレットキーのみで保護されることになります。",
  "settings.sync.key.removeCode": "認証アプリのコード",
  "settings.sync.key.removeCodeHint":
    "現在のコードを入力すると、すぐにキーを削除できます。空のままにすると、代わりにブラウザでセキュリティキーを使って確認します。",
  "settings.sync.key.removeConfirm": "キーを削除",
  "settings.sync.key.removedToast": "セキュリティキーを削除しました。",
  "settings.sync.key.confirmTitle": "セキュリティキーで確認",
  "settings.sync.key.confirmBody":
    "キーを削除するには、2つ目のステップが必要です。マスターパスワードを入力し、ブラウザでキーを使って確認してください。",
  "settings.sync.key.step.methods": "このアカウントのサインイン方法を確認しています…",
  "settings.sync.key.step.useKey": "セキュリティキーを使用（ブラウザが開きます）",
  "settings.sync.key.step.keyOnlyTitle": "セキュリティキーを使用してください",
  "settings.sync.key.step.keyOnlyBody":
    "このアカウントはセキュリティキーで保護されています。Mosaic がブラウザでページを開くので、そこで使用してください。",
  "settings.sync.key.step.or": "またはコードを使用",
  "settings.sync.key.wait.login": "セキュリティキーでサインイン",
  "settings.sync.key.wait.register": "セキュリティキーを追加",
  "settings.sync.key.wait.confirm": "セキュリティキーで確認",
  "settings.sync.key.wait.lead":
    "ブラウザで、同期サーバーの Mosaic ページが開きました。そこでこのコードを入力してから、セキュリティキーを使用してください。",
  "settings.sync.key.wait.codeLabel": "ブラウザで入力するコード",
  "settings.sync.key.wait.codeSpoken": "ブラウザで入力するコード: {digits}",
  "settings.sync.key.wait.expires": "このステップは {time} に期限切れになります。",
  "settings.sync.key.wait.waiting": "セキュリティキーを待っています…",
  "settings.sync.key.wait.done": "完了しました。",
  "settings.sync.key.wait.expired": "このステップはタイムアウトしました。もう一度開始してください。",
  "settings.sync.key.wait.failed":
    "ステップに失敗しました。コードの誤りが多すぎたか、セキュリティキーが受け付けられませんでした。何も変更されていません。もう一度開始してください。",
  "settings.sync.key.wait.reopen": "ブラウザのページをもう一度開く",
  "settings.sync.key.wait.retry": "もう一度開始",
  "sync.warning.region": "同期の警告",
  "sync.warning.unknownDevice": "別のデバイス",
  "sync.warning.unknownVault": "いずれかの保管庫",
  "sync.warning.freezeSuspected":
    "{device} からの変更が1日以上届いていません。他のデバイスの電源が入っていて、オンラインになっているか確認してください。",
  "sync.warning.missingFiles":
    "{device} からのファイルがフォルダにありません。クラウドサービスの同期が完了するのを待ってから、もう一度同期してください。",
  "sync.warning.noCommonView":
    "このデバイスと {device} が、2日間、同じサーバーデータを確認できていません。サーバーが異なるバージョンを表示している可能性があります。",
  "sync.warning.rollbackDetected":
    "同期データがこのデバイスより古いようです。サーバーまたはフォルダを確認してから、「確認しました」を選んでください。",
  "sync.warning.identityDuplicated":
    "バックアップの復元後などに、別の Mosaic がこのデバイスの ID で書き込みを行っています。同期は一時停止中です。使用しているコピーが1つだけであることを確認し、「確認しました」を選んでください。",
  "sync.warning.serverInconsistent":
    "サーバーが、このデバイスと他のデバイスに異なるデータを表示しました。サーバーを確認してから、「確認しました」を選んでください。",
  "sync.warning.serverRollback":
    "サーバーにある {vault} のコピーが、このデバイスのものより古くなっています。サーバーを確認してから、「確認しました」を選んでください。",
  "sync.warning.undecryptable":
    "受信した{count, plural, other {#件の項目}}を、このデバイスでは読み取れませんでした。すべてのデバイスで Mosaic を更新してから、もう一度同期してください。",
  "sync.warning.hardwareVaultsLocal":
    "ハードウェアで保護された保管庫は、このデバイスにとどまります。{count, plural, other {#個の保管庫}}は同期されません。",
  "sync.warning.attachmentsNotRepaired":
    "サーバーまたはフォルダ上の{count, plural, other {#個のファイル}}を、このデバイスから修復できませんでした。ファイルがまだあるデバイスで項目を開き、もう一度追加してください。",
  "sync.warning.acknowledge": "確認しました…",
  "sync.acknowledge.title": "確認が必要な同期の警告を消去しますか？",
  "sync.acknowledge.body":
    "ご自身でサーバーまたはフォルダを確認してから進めてください。確認が必要なすべての警告が消去され、同期が再開されます。データが実際に巻き戻された、または改ざんされていた場合でも、Mosaic には後から判断できません。",
  "sync.acknowledge.eachDevice": "この操作は、お使いのデバイスごとに行ってください。",
  "toast.syncAttention":
    "同期により、別のデバイスの一部の項目が統合または修復されました。変更内容を確認してください。",
  "sync.revoked.rekey":
    "このアカウントの鍵が別のデバイスで更新されました。最新の緊急キットにある新しいシークレットキーでサインインしてください。",
  "sync.revoked.passwordChange":
    "マスターパスワードが別のデバイスで変更されました。新しいパスワードでサインインしてください。",
  "sync.revoked.deviceRemoved":
    "このデバイスは別のデバイスでアカウントから削除されました。もう一度サインインして、追加し直してください。",
  "sync.revoked.operator":
    "サーバーの運営者がこのセッションを終了しました。同期を続けるには、もう一度サインインしてください。",
} satisfies Record<keyof typeof en, string>;
