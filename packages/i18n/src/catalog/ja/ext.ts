/**
 * Japanese UI strings: ext. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { ext as en } from "../en/ext.ts";

export const ext = {
  "ext.manifest.name": "Mosaic",
  "ext.manifest.description":
    "Mosaic の保管庫からログインを入力します。マスターパスワードはデスクトップアプリの中にとどまります。",
  "ext.command.fill": "Mosaic でログインを入力",
  "ext.command.generate": "Mosaic でパスワードを生成",
  "ext.save.badgeTitle": "ログインを保存しますか？",
  "ext.loading": "読み込み中…",
  "ext.site.secure": "現在のサイト、安全な接続（https）: {host}",
  "ext.site.insecure": "現在のサイト、暗号化なし（http）: {host}",
  "ext.site.none": "このタブにはウェブサイトがありません",
  "ext.nav.label": "表示",
  "ext.nav.logins": "ログインと検索",
  "ext.nav.generator": "パスワードジェネレーター",
  "ext.footer.open": "Mosaic を開く",
  "ext.footer.more": "その他のオプション",
  "ext.footer.connected": "Mosaic に接続済み",
  "ext.footer.locked": "Mosaic はロックされています",
  "ext.footer.offline": "未接続",
  "ext.more.saveOffers": "ログインの保存を提案",
  "ext.more.saveOffersHint":
    "拡張機能が送信されたログインを検知できるよう、すべてのウェブサイトへのアクセスが必要です。Mosaic アプリで確認するまで、何も保存されません。",
  "ext.passkeys.toggle": "パスキーに Mosaic を使用",
  "ext.passkeys.hint":
    "パスキーのリクエストに応答するため、すべてのウェブサイトへのアクセスが必要です。リクエストごとに Mosaic アプリで確認します。",
  "ext.passkeys.permissionDenied":
    "パスキーのリクエストに応答するには、Mosaic がウェブサイトにアクセスできる必要がありますが、許可されていません。",
  "ext.more.disconnect": "Mosaic との接続を解除",
  "ext.more.disconnectTitle": "このブラウザの接続を解除しますか？",
  "ext.more.disconnectBody":
    "もう一度接続するまで、このブラウザはログインを入力できません。新しい接続は Mosaic アプリで承認します。",
  "ext.action.cancel": "キャンセル",
  "ext.neverSave.menu": "保存を提案しないサイト…",
  "ext.neverSave.title": "保存を提案しないサイト",
  "ext.neverSave.intro":
    "Mosaic は、これらのサイトではログインの保存を提案しません。再び提案されるようにするには、サイトを削除してください。",
  "ext.neverSave.empty":
    "まだサイトはありません。Mosaic がログインの保存を提案したときに「このサイトでは今後しない」を選ぶと追加できます。",
  "ext.neverSave.remove": "削除",
  "ext.neverSave.removeLabel": "{origin} をリストから削除",
  "ext.neverSave.removed": "{origin} を削除しました。Mosaic は、そこでもう一度ログインの保存を提案できます。",
  "ext.neverSave.close": "閉じる",
  "ext.connect.checking": "Mosaic に接続しています…",
  "ext.connect.hostMissing.title": "このブラウザでは Mosaic が設定されていません",
  "ext.connect.hostMissing.body":
    "このコンピューターに Mosaic アプリをインストールしてから、ブラウザを再起動してください。",
  "ext.connect.hostMissing.dev": "開発者向け: {command} を実行してから、ブラウザを再起動してください。",
  "ext.connect.hostForbidden.title": "このブラウザは接続を許可されていません",
  "ext.connect.hostForbidden.body":
    "Mosaic が通信するのは、公式の Mosaic 拡張機能のみです。ブラウザのストアからインストールしてください。",
  "ext.connect.desktopNotRunning.title": "このコンピューターで Mosaic を開いてください",
  "ext.connect.desktopNotRunning.body":
    "拡張機能は Mosaic アプリを通して動作します。アプリを起動してから、もう一度確認してください。",
  "ext.connect.check": "もう一度確認",
  "ext.connect.notPaired.title": "Mosaic に接続",
  "ext.connect.notPaired.body":
    "このブラウザを、このコンピューター上の Mosaic アプリに接続します。アプリで承認してください。",
  "ext.connect.start": "接続",
  "ext.connect.pairing.title": "Mosaic で確認",
  "ext.connect.pairing.body": "Mosaic に同じコードが表示されていることを確認し、そこで「許可」をクリックしてください。",
  "ext.connect.pairing.code": "コード",
  "ext.connect.pairing.waiting": "Mosaic アプリでの応答を待っています…",
  "ext.connect.protocolMismatch.title": "Mosaic を更新してください",
  "ext.connect.protocolMismatch.body":
    "拡張機能と Mosaic アプリのバージョンが異なります。両方を最新版に更新してください。",
  "ext.connect.error.title": "Mosaic に接続できません",
  "ext.connect.error.body": "Mosaic アプリが起動しているか確認してから、もう一度お試しください。",
  "ext.locked.title": "Mosaic はロックされています",
  "ext.locked.body": "{host} で入力するには、Mosaic アプリでロックを解除してください。",
  "ext.locked.bodyNoSite": "ここで使うには、Mosaic アプリでロックを解除してください。",
  "ext.locked.open": "Mosaic を開く",
  "ext.locked.note": "マスターパスワードを入力するのは、Mosaic アプリの中だけです。",
  "ext.logins.forSite": "{host} 用",
  "ext.logins.count": "{count, plural, other {#件のログイン}}",
  "ext.logins.sameSite": "同じサイト",
  "ext.logins.none": "{host} に保存されたログインはありません。",
  "ext.logins.noneHint": "Mosaic は完全なアドレスを比較するため、似せたサイトが一致することはありません。",
  "ext.logins.unsupported": "Mosaic はこのページには入力できません。保管庫を検索してログインをコピーしてください。",
  "ext.logins.shortcut": "{shortcut} で最初の一致を入力します。",
  "ext.logins.frameNote":
    "Mosaic が入力するのは {host} のみで、他のサイトの埋め込みフレーム内には入力しません。",
  "ext.item.untitled": "無題",
  "ext.item.noUsername": "ユーザー名なし",
  "ext.item.twoFactor": "2FA",
  "ext.item.fill": "入力",
  "ext.item.fillLabel": "{name} を入力",
  "ext.item.copyMenu": "{name} のコピーオプション",
  "ext.item.copyUsername": "ユーザー名をコピー",
  "ext.item.copyPassword": "パスワードをコピー",
  "ext.item.copyCode": "コードをコピー",
  "ext.item.blocked": "ブロックされました: このページは暗号化されておらず、このログインでは許可されていません。",
  "ext.search.label": "その他の項目",
  "ext.search.placeholder": "保管庫を検索",
  "ext.search.note":
    "結果はこのポップアップ内にとどまります。ページが受け取るのは、入力する1件のログインのみです。",
  "ext.search.none": "見つかりませんでした。",
  "ext.search.copy": "コピー",
  "ext.search.copyOnly": "コピーのみ: このサイトには保存されていません",
  "ext.shortcut.copyUser": "{platform, select, mac {⌘C} other {Ctrl+C}}",
  "ext.shortcut.copyPassword": "{platform, select, mac {⇧⌘C} other {Ctrl+Shift+C}}",
  "ext.shortcut.copyCode": "{platform, select, mac {⌥⌘C} other {Ctrl+Alt+C}}",
  "ext.search.results": "{count, plural, other {#件の結果}}",
  "ext.frame.cross":
    "フォーカスのある欄は、別のサイトのフレーム内にあります。Mosaic はそこには入力しません。",
  "ext.frameConfirm.title": "埋め込みフレーム内に入力しますか？",
  "ext.frameConfirm.body":
    "この欄はページ自体ではなく、{host} のフレーム内にあります。このフレームを信頼できる場合にのみ入力してください。",
  "ext.frameConfirm.confirm": "フレーム内に入力",
  "ext.warning.idn.title": "アドレスを確認してください",
  "ext.warning.idn.body":
    "このアドレスには特殊な文字が使われており、別のサイトに見える可能性があります。サインインする前に、アドレスバーを確認してください。",
  "ext.warning.lookalike.body":
    "このサイトは {saved} ではありません。似ていますが、これはフィッシングによくある手口です。このタブを閉じるか、ご自身で {saved} を入力してください。",
  "ext.warning.youAreOn": "現在のサイト",
  "ext.warning.writtenAs": "表記",
  "ext.warning.savedFor": "保存済みのサイト",
  "ext.warning.insecure.title": "このページは暗号化されていません",
  "ext.warning.insecure.body":
    "このページは http を使用しています。同じネットワーク上の他の人に、パスワードを読み取られる可能性があります。Mosaic がここで入力するのは、許可したログインに対してのみです。",
  "ext.otp.title": "ワンタイムコードを入力",
  "ext.otp.next": "{name} のパスワードを入力しました。次は2段階認証コードです。",
  "ext.otp.for": "{name} のコード",
  "ext.otp.fill": "コードを入力",
  "ext.otp.copy": "コードをコピー",
  "ext.otp.note": "コードは Mosaic から直接入力欄に入ります。このポップアップには表示されません。",
  "ext.save.titleNew": "{host} のこのログインを保存しますか？",
  "ext.save.titleUpdate": "{host} の保存済みログインを更新しますか？",
  "ext.save.generated": "Mosaic が {host} 用に生成したパスワードです。",
  "ext.save.updates": "「{title}」を更新します。",
  "ext.save.username": "ユーザー名",
  "ext.save.noUsername": "なし",
  "ext.save.password": "パスワード",
  "ext.save.passwordHidden": "非表示です。Mosaic アプリで確認できます。",
  "ext.save.insecure": "このページは暗号化されていません（http）。",
  "ext.save.action": "Mosaic に保存…",
  "ext.save.notNow": "今はしない",
  "ext.save.never": "このサイトでは今後しない",
  "ext.save.waiting": "Mosaic アプリで確認してください。確認するまで、何も保存されません。",
  "ext.save.saved": "Mosaic に保存しました。",
  "ext.save.updated": "Mosaic で更新しました。",
  "ext.save.cancelled": "保存しませんでした。",
  "ext.save.dismissed": "破棄しました。パスワードは保持されません。",
  "ext.save.neverDone": "Mosaic は、{host} では今後ログインの保存を提案しません。",
  "ext.save.permissionAsk":
    "Mosaic がブラウザにウェブサイトへのアクセスを求めています。オンにするには許可してください。",
  "ext.generator.title": "{host} 用のパスワードを生成",
  "ext.generator.titleGeneric": "パスワードを生成",
  "ext.generator.value": "生成されたパスワード",
  "ext.generator.regenerate": "別のパスワードを生成",
  "ext.generator.type": "パスワードのタイプ",
  "ext.generator.random": "ランダム",
  "ext.generator.passphrase": "パスフレーズ",
  "ext.generator.include": "含める文字",
  "ext.generator.rules.page": "このページのパスワードルールに従います",
  "ext.generator.rules.siteQuirks": "このサイトの既知のルールに従います",
  "ext.generator.fill": "入力",
  "ext.generator.fillHint":
    "入力するには、ページの新しいパスワード欄をクリックしてから、このポップアップをもう一度開いてください。",
  "ext.generator.copy": "コピー",
  "ext.generator.expired": "そのパスワードはすでに保持されていないため、新しいものを生成しました。",
  "ext.generator.filled": "入力しました。Mosaic が保存を提案します。",
  "ext.copied": "コピーしました。{seconds, plural, other {#秒}}後に消去されます。",
  "ext.copiedNoClear": "コピーしました。",
  "ext.filled": "入力しました。",
  "ext.filledCode": "コードを入力しました。",
  "ext.error.invalidRequest":
    "Mosaic はそのリクエストを理解できませんでした。拡張機能と Mosaic アプリを更新してください。",
  "ext.error.notPaired": "このブラウザは Mosaic に接続されていません。もう一度接続してください。",
  "ext.error.protocolVersion":
    "拡張機能と Mosaic アプリのバージョンが異なります。両方を更新してください。",
  "ext.error.busy": "Mosaic は別のリクエストを処理中です。少し待ってから、もう一度お試しください。",
  "ext.error.locked": "Mosaic はロックされています。Mosaic アプリでロックを解除してください。",
  "ext.error.rateLimited":
    "短時間にリクエストが多すぎます。少し待ってから、もう一度お試しください。",
  "ext.error.originMismatch":
    "このログインはこのウェブサイト用に保存されていないため、Mosaic はここには入力しません。",
  "ext.error.insecureOrigin":
    "このページは暗号化されておらず、このログインでは入力が許可されていません。何も入力されませんでした。",
  "ext.error.crossOriginFrame":
    "その欄は別のサイトのフレーム内にあります。Mosaic はそこには入力しません。",
  "ext.error.itemNotFound": "その項目はもう存在しません。移動または削除された可能性があります。",
  "ext.error.vaultUnavailable": "その保管庫は閉じています。先に Mosaic アプリで開いてください。",
  "ext.error.noOtp": "この項目にはワンタイムコードがありません。",
  "ext.error.otpNotFillable":
    "このコードは使用のたびにカウントが進む方式（HOTP）のため、ここから入力やコピーはできません。Mosaic アプリで使用してください。",
  "ext.error.fieldEmpty": "その欄は空のため、入力やコピーするものがありません。",
  "ext.error.clipboardUnavailable":
    "Mosaic はクリップボードを使用できませんでした。もう一度お試しいただくか、Mosaic アプリでコピーしてください。",
  "ext.error.cancelled": "Mosaic アプリでキャンセルされました。準備ができたら、もう一度お試しください。",
  "ext.error.passkeyNotFound": "このサイトのパスキーは Mosaic にありません。",
  "ext.error.passkeyExcluded": "このアカウントのパスキーは、すでに Mosaic にあります。",
  "ext.error.rpIdMismatch":
    "このサイトはそのドメインのパスキーを使用できないため、Mosaic は応答しませんでした。",
  "ext.error.passkeyUnsupported":
    "Mosaic は、このサイトが求めたオプションではパスキーを作成できません。",
  "ext.error.pairingDenied":
    "Mosaic アプリで接続が拒否されました。「接続」を選んでもう一度試し、「許可」を選んでください。",
  "ext.error.pairingExpired":
    "接続リクエストがタイムアウトしました。「接続」を選んで、最初からやり直してください。",
  "ext.error.pairingConflict": "別の接続リクエストが開いていました。「接続」を選んで、もう一度お試しください。",
  "ext.error.pairingBlocked": "拒否された接続が多すぎます。10分待ってから、もう一度お試しください。",
  "ext.error.pairingLimit":
    "このコンピューターに接続されているブラウザが多すぎます。Mosaic の設定で1つ接続を解除してください。",
  "ext.error.internal":
    "Mosaic で予期しない問題が発生しました。もう一度お試しください。繰り返される場合は、Mosaic アプリを再起動してください。",
  "ext.error.desktopNotRunning": "このコンピューターで Mosaic が起動していません。起動してから、もう一度お試しください。",
  "ext.error.desktopNotAuthentic":
    "応答したアプリは、このブラウザが接続している Mosaic アプリではありません。何も送信されていません。",
  "ext.error.timeout":
    "Mosaic が時間内に応答しませんでした。Mosaic アプリでダイアログが開いている場合は、そこで応答してください。",
  "ext.error.frameTooLarge": "Mosaic に送信するにはデータが大きすぎます。",
  "ext.error.hostMissing":
    "このブラウザには Mosaic のヘルパーがインストールされていません。Mosaic アプリをインストールしてから、ブラウザを再起動してください。",
  "ext.error.hostForbidden": "このブラウザまたは拡張機能は、Mosaic との通信を許可されていません。",
  "ext.error.hostExited": "Mosaic との接続が閉じられました。もう一度お試しください。",
  "ext.error.unsupportedPage": "Mosaic はこの種類のページには入力できません。",
  "ext.error.noFields": "ここにはログイン欄が見つかりません。欄をクリックしてから、もう一度お試しください。",
  "ext.error.noMatch": "このサイトに保存されたログインは Mosaic にありません。",
  "ext.error.frameNeedsConfirmation":
    "この欄はフレーム内にあります。Mosaic のポップアップを開いてフレームを確認すると、入力できます。",
  "ext.error.frameOpaque": "この欄はサンドボックス化されたフレーム内にあります。Mosaic はそこには入力しません。",
  "ext.error.originChanged":
    "Mosaic の入力中にページが変わりました。何も入力されませんでした。もう一度お試しください。",
  "ext.error.pageHidden": "ページが表示されていないため、Mosaic は入力しませんでした。",
  "ext.error.permissionDenied":
    "ログインを検知するには、Mosaic がウェブサイトにアクセスできる必要がありますが、許可されていません。",
  "ext.error.generatedExpired": "そのパスワードは Mosaic にもう保持されていません。新しく生成してください。",
  "ext.rotation.card.title": "パスワードの変更",
  "ext.rotation.card.for": "{name} 用",
  "ext.rotation.card.started": "開始: {when}",
  "ext.rotation.card.delivered": "新しいパスワードは、すでに一度入力されています。",
  "ext.rotation.card.note":
    "新しいパスワードが表示されるのは Mosaic アプリ内のみです。Mosaic がフォームを送信することはありません。",
  "ext.rotation.card.list": "進行中のパスワード変更",
  "ext.rotation.change": "パスワードを変更…",
  "ext.rotation.changeLabel": "{name} のパスワードを変更…",
  "ext.rotation.openPage": "変更ページを開く",
  "ext.rotation.fillChange": "変更フォームに入力",
  "ext.rotation.fillPending": "新しいパスワードを入力してサインイン",
  "ext.rotation.worked": "うまくいった",
  "ext.rotation.failed": "うまくいかなかった",
  "ext.rotation.fresh": "新しいパスワードでやり直す",
  "ext.rotation.waiting": "Mosaic アプリでの操作を待っています…",
  "ext.rotation.started": "開始しました。Mosaic アプリで「このパスワードを使う」を選んでください。",
  "ext.rotation.startCancelled": "何も開始されませんでした。",
  "ext.rotation.filled": "変更フォームに入力しました。内容を確認して、ご自身で送信してください。",
  "ext.rotation.opened": "変更ページを開きました。",
  "ext.rotation.confirmed": "完了しました。新しいパスワードが Mosaic に保存されました。",
  "ext.rotation.keptOld": "Mosaic は古いパスワードを維持しました。",
  "ext.rotation.resultCancelled": "何も変更されていません。変更は開いたままです。",
  "ext.rotation.offer.title": "関連するアドレス",
  "ext.rotation.offer.body":
    "このページは {host} にあり、{title} には保存されていません。Mosaic がそこで入力できるよう、追加しますか？",
  "ext.rotation.offer.add": "{host} を追加",
  "ext.rotation.offer.added": "追加しました。変更がここに表示されるようになります。",
  "ext.rotation.offer.cancelled": "追加しませんでした。",
  "ext.error.rotationNotFound":
    "そのパスワード変更はもう開いていません。ログインから新しく開始してください。",
  "ext.error.originNotRelated":
    "そのアドレスはこのログインと同じサイトにないため、Mosaic は追加しませんでした。",
  "ext.error.rotationLimit":
    "このログインには、未完了の変更がすでに4件あります。先に Mosaic アプリで1件を完了するかキャンセルしてください。",
  "ext.error.changeFormNotFound":
    "Mosaic はここでパスワード変更フォームを見つけられませんでした。アカウントのパスワードページを開いてから、もう一度お試しください。",
  "ext.error.changeFormAmbiguous":
    "Mosaic はどの欄がどれか判断できませんでした。Mosaic から新しいパスワードをコピーして、ご自身で貼り付けてください。",
} satisfies Record<keyof typeof en, string>;
