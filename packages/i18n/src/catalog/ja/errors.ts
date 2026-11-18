/**
 * Japanese UI strings: errors. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { errors as en } from "../en/errors.ts";

export const errors = {
  "error.notImplemented": "この機能はまだ利用できません。Mosaic の今後のバージョンで提供予定です。",
  "error.noAccount":
    "このデバイスには、まだアカウントがありません。アカウントを作成するか、緊急キットでサインインしてください。",
  "error.accountExists": "このデバイスには、すでに Mosaic のアカウントがあります。ロックを解除してください。",
  "error.locked": "Mosaic はロックされています。続けるには、ロックを解除してください。",
  "error.alreadyUnlocked": "Mosaic はすでにロック解除されています。中断したところから続けられます。",
  "error.wrongCredentials":
    "マスターパスワードまたはシークレットキーが正しくありません。Caps Lock とキーボードの配列を確認して、もう一度お試しください。",
  "error.secretKeyRequired":
    "このデバイスにはシークレットキーがありません。緊急キットを見ながら入力してください。",
  "error.secretKeyMalformed":
    "シークレットキーの形式ではありません。シークレットキーは「M1-」で始まり、緊急キットに記載されています。",
  "error.secretKeyMistyped":
    "シークレットキーに入力ミスがあります。緊急キットと1文字ずつ照らし合わせてください。",
  "error.emptyPassword": "マスターパスワードが空です。続けるには入力してください。",
  "error.quickUnlockUnavailable":
    "このデバイスではクイックロック解除が設定されていません。マスターパスワードでロックを解除してください。",
  "error.storeInUse":
    "このコンピューターで、すでに Mosaic が開かれています。もう一方のウィンドウを閉じてから、もう一度お試しください。",
  "error.storeUnavailable":
    "Mosaic は、このデバイスの保管庫ファイルを開けませんでした。ディスクが使用可能で空き容量があるか確認してから、Mosaic を再起動してください。",
  "error.vaultNotFound": "その保管庫はもう存在しません。別のデバイスで削除された可能性があります。",
  "error.vaultUnavailable":
    "この保管庫は、このセッションでは開けません。ハードウェアキーでロックを解除するか、同期してからもう一度お試しください。",
  "error.itemNotFound":
    "その項目はもう存在しません。別のデバイスで移動または削除された可能性があります。",
  "error.itemDeleted":
    "その項目は完全に削除されたため、開けません。別の項目を選んでください。",
  "error.sameVault": "その項目は、すでにその保管庫にあります。別の保管庫を選んでください。",
  "error.noOtp": "この項目にはワンタイムコードがありません。項目を編集して追加してください。",
  "error.fieldEmpty":
    "その欄は空のため、コピーや表示するものがありません。項目を編集して入力してください。",
  "error.invalidInput":
    "このフォームに無効な内容があります。強調表示されている欄を確認して、もう一度お試しください。",
  "error.sshKeyInvalid":
    "OpenSSH の秘密鍵ではありません。BEGIN と END の行を含め、キー全体を貼り付けてください。",
  "error.reauthRequired": "この変更にはマスターパスワードが必要です。確認のため入力してください。",
  "error.attachmentTooLarge": "そのファイルは 50 MiB を超えています。もっと小さいファイルを選んでください。",
  "error.attachmentQuota":
    "ファイルの合計が、利用できる 1 GiB を超えてしまいます。添付ファイルをいくつか削除してから、もう一度お試しください。",
  "error.attachmentNotFound": "このファイルは、もうこの項目に含まれていません。項目を再読み込みしてください。",
  "error.attachmentUnavailable":
    "このファイルは、このデバイスでは開けません。まだ同期されていないか、破損している可能性があります。",
  "error.attachmentFileUnreadable": "そのファイルを読み取れません。別のファイルを選んでください。",
  "error.attachmentSaveFailed": "ファイルをそこに保存できませんでした。別のフォルダを選んでください。",
  "error.attachmentPreviewUnsupported": "このファイルはプレビューできません。「別名で保存」から開いてください。",
  "error.attachmentPreviewTooLarge": "この画像は大きすぎてプレビューできません。「別名で保存」から開いてください。",
  "error.clipboardUnavailable":
    "Mosaic はクリップボードを使用できませんでした。クリップボードを使用している可能性のあるアプリを閉じてから、もう一度お試しください。",
  "error.importFileUnreadable":
    "Mosaic はそのファイルを読み取れませんでした。ファイルが存在し、他のアプリで開かれていないか確認してから、もう一度お試しください。",
  "error.importDeleteNotAllowed":
    "先にファイルを読み込んでください。Mosaic は、項目が保管庫に入った後でのみ、エクスポートしたファイルを削除します。",
  "error.importDeleteFailed":
    "ファイルを削除できませんでした。他のアプリで開かれているか、読み取り専用の可能性があります。ご自身で削除してから、ゴミ箱を空にしてください。",
  "error.importFormatInvalid":
    "そのファイルは、選択した形式ではありません。合う形式を選ぶか、ファイルをもう一度エクスポートしてください。",
  "error.importPreviewExpired": "この読み込みはキャンセルされました。最初からやり直すには、もう一度ファイルを選択してください。",
  "error.sshRequestExpired":
    "その SSH リクエストはすでに終了しています。まだ必要な場合は、コマンドをもう一度実行してください。",
  "error.sshAgentUnavailable":
    "SSH エージェントを起動できませんでした。他の SSH エージェントが動作していないか確認してから、もう一度オンにしてください。",
  "error.unlockDelayed": "続けて間違えた回数が多すぎます。数秒待ってから、もう一度お試しください。",
  "error.tooManyAttempts":
    "間違いが多すぎたため、Mosaic をロックしました。続けるには、マスターパスワードでロックを解除してください。",
  "error.sshAllowTooSoon":
    "リクエストを確認するには操作が早すぎました。キーとプログラムを確認してから、もう一度「許可」を選んでください。",
  "error.browserRequestExpired":
    "そのブラウザのリクエストはすでに終了しています。まだ必要な場合は、拡張機能からもう一度リクエストしてください。",
  "error.browserAllowTooSoon":
    "リクエストを確認するには操作が早すぎました。詳細を確認してから、もう一度選んでください。",
  "error.browserPairingNotFound":
    "そのブラウザは、もう接続されていません。設定を開き直して、現在の一覧を確認してください。",
  "error.browserHostMissing":
    "この Mosaic にはブラウザコネクタがインストールされていないため、修復するものがありません。ブラウザを接続するには、インストーラーで Mosaic をインストールしてください。",
  "error.agentOriginInvalid":
    "エージェントがキーを送信できるのは、https:// で始まり、サイトのみを示すアドレス（例: https://api.example.com）に対してだけです。アドレスを確認してください。",
  "error.agentBindingInvalid":
    "Mosaic は、この設定をエージェント用に使用できません。欄、サイト、キーの送信方法を確認してください。",
  "error.agentValueUnusable":
    "この値は短すぎるか、改行を含んでいるため、エージェントは使用できません。キーは、1行で12文字以上必要です。",
  "error.agentVaultShared":
    "エージェントが使えるのは、自分のもので共有されていない保管庫にある項目のみです。先に項目を自分の保管庫に移動してください。",
  "error.agentAccessOff": "エージェントのアクセスはオフです。「設定」の「エージェント」でオンにしてください。",
  "error.agentRequestExpired":
    "そのエージェントのリクエストはすでに終了しています。まだ必要な場合は、エージェントがもう一度リクエストできます。",
  "error.agentAllowTooSoon":
    "リクエストを確認するには操作が早すぎました。詳細を確認してから、もう一度選んでください。",
  "error.agentPairingNotFound":
    "そのエージェントは、もう接続されていません。設定を開き直して、現在の一覧を確認してください。",
  "error.agentEndpointUnavailable":
    "Mosaic はエージェントが使用する接続を開けませんでした。他の Mosaic を閉じてから、もう一度お試しください。",
  "error.kitNotVerified":
    "先に新しい緊急キットを保存し、その最後のグループを確認してください。それまで、Mosaic は何も変更しません。",
  "error.kitGroupMismatch":
    "新しいシークレットキーの最後のグループではありません。新しい緊急キットを確認してください。",
  "error.rotationInProgress":
    "別の鍵の変更がまだ開いています。「鍵の更新を続行」で完了するか、先に破棄してください。",
  "error.rotationNone": "続行する鍵の変更はありません。",
  "error.rotationNotFound":
    "そのパスワード変更はもう開いていません。別のデバイスまたはブラウザで閉じられた可能性があります。項目を開き直して、状況を確認してください。",
  "error.rotationLimit":
    "この項目のパスワード変更が、すでに4件開いています。先に項目内で1件を完了するか閉じてください。",
  "error.rotationOpen":
    "このログインには、開いているパスワード変更があります。ゴミ箱から復元し、項目で「うまくいった」または「うまくいかなかった」を選んでください。その後で完全に削除できます。",
  "error.rotationNotSupported":
    "Mosaic はこの変更を案内できません。https のアドレスがあるログインが必要です。サイトで、ご自身でパスワードを変更してください。",
  "error.historyNotFound":
    "その以前のパスワードは、もう履歴にありません。項目を開き直して、現在の一覧を確認してください。",
  "error.originNotRelated":
    "そのページはこのログインのサイトに属していないため、Mosaic は追加しませんでした。追加すべき場合は、項目のアドレスを編集してください。",
  "error.breachConsentRequired":
    "漏えいチェックはオフです。先にオンにして、送信される内容に同意してください。",
  "error.breachRunning":
    "漏えいチェックがすでに実行中です。完了を待つか、先に停止してください。",
  "error.rotationTooFar":
    "この変更はすでにアカウントに反映されているため、破棄できません。続行して完了してください。",
  "error.rotationJumpTooLarge":
    "このデバイスは他のデバイスから遅れすぎていて、追いつけません。設定し直してください。このデバイスからアカウントを削除し、緊急キットでもう一度参加してください。",
  "error.rotationNotApplied":
    "この変更はアカウントに反映されなかったため、何も変更されていません。やり直すか、破棄してください。",
  "error.secretKeyUnchanged":
    "そのシークレットキーは、もう使えません。変更を行ったデバイスで保存した緊急キットのキーを使用してください。",
  "error.passwordUnchanged": "新しいマスターパスワードは、現在のものと異なる必要があります。",
  "error.profileDowngrade":
    "そのレベルは、アカウントが現在使っているものより強くありません。より高いレベルを選んでください。",
  "error.profileTooSlow":
    "そのレベルでは、このデバイスでのロック解除に2秒以上かかります。より低いレベルを選ぶか、そのままアップグレードしてください。",
  "error.rekeyNeedsCleanSync":
    "鍵の再作成には、同期が最新で、警告がない状態であることが必要です。今すぐ同期し、警告があれば確認してから、もう一度お試しください。",
  "error.rekeyMemberKeyMismatch":
    "共有保管庫の全員の安全コードをまだ照合していないため、その保管庫は更新できません。保管庫の共有設定で全員のコードを照合してから、もう一度お試しください。",
  "error.rekeyBusy": "Mosaic の処理中に、保管庫が変更され続けました。少し待ってから、もう一度お試しください。",
  "error.rekeyOutcomeUnknown":
    "アカウントが新しい鍵を受け付けたか、Mosaic は確認できませんでした。「鍵の更新を続行」を選んで確認してください。",
  "error.rekeyNeedsServer":
    "鍵の再作成には同期サーバーまたはフォルダが必要ですが、接続できません。接続を確認して、もう一度お試しください。",
  "error.rekeyNoAccountFile":
    "このパスワードとシークレットキーで開けるものが、同期フォルダにありません。新しい緊急キットを確認してください。",
  "error.syncAccountRekeyed":
    "アカウントの鍵が別のデバイスで更新されました。続けるには、新しい緊急キットのシークレットキーを使用してください。",
  "error.syncEpochRollback":
    "同期先が、このデバイスが持っているものより古いアカウントの鍵を表示しています。何も変更されていません。",
  "error.syncRekeyRefused":
    "同期先が示す鍵は、このデバイスの鍵の次のものではありません。何も変更されていません。他のデバイスを確認してください。",
  "error.pickerStale": "このリストは古くなっています。もう一度欄をクリックしてください。",
  "error.pickerTooSoon": "少し待ってから、もう一度確認してください。",
  "error.programFillUnavailable": "このデバイスでは、プログラムへの入力は利用できません。",
  "error.busy": "Mosaic が直前のステップをまだ完了していません。少し待ってから、もう一度お試しください。",
  "error.internal":
    "予期しない問題でこの操作が停止しました。もう一度お試しください。繰り返される場合は、Mosaic を再起動してください。",
  "error.importTooLarge":
    "このファイルは大きすぎるか複雑すぎて、読み込めません。一度にエクスポートする項目を減らしてから、もう一度お試しください。",
  "error.importUnsupported":
    "Mosaic はこの形式のバージョンを読み取れません。最新の形式でもう一度エクスポートしてから、お試しください（KeePass の場合: AES または ChaCha20 を使った KDBX 4）。",
  "error.importEncrypted":
    "このエクスポートはパスワードで保護されています。暗号化なしでもう一度エクスポートし、読み込んでから、ファイルを削除してください。",
  "error.sshAgentEndpointInUse":
    "別の SSH エージェントがすでに動作しています（Windows では、OpenSSH Authentication Agent サービス）。それを停止してから、Mosaic の SSH エージェントをもう一度オンにしてください。",
  "error.cancelled": "キャンセルしました。何も変更されていません。",
  "error.quickUnlockExpired":
    "クイックロック解除の有効期限が切れました。マスターパスワードでロックを解除し、「設定」でもう一度オンにしてください。",
  "error.quickUnlockInvalidated":
    "このデバイスでクイックロック解除が動作しなくなり、オフになりました。マスターパスワードでロックを解除してください。",
  "error.passwordRequired":
    "今回は、マスターパスワードを入力してください。その後、クイックロック解除が再び使えます。",
  "error.hardwareKeyUnsupported": "このセキュリティキーまたはシステムは、Mosaic では使用できません。",
  "error.hardwareKeyNotFound": "セキュリティキーが見つかりません。挿入してから、もう一度お試しください。",
  "error.hardwareKeyTimeout": "時間内にキーに触れませんでした。もう一度お試しください。",
  "error.hardwareKeyPinRequired": "セキュリティキーの PIN を入力してください。",
  "error.hardwareKeyPinInvalid": "PIN が正しくありません。何度か間違えると、キーが自動的にロックされます。",
  "error.hardwareKeyPinBlocked":
    "PIN の誤りが多すぎたため、このキーはロックされました。製造元のツールでリセットしてください。ただし、登録内容は失われます。",
  "error.hardwareKeyNoPin": "先に、製造元のツールでこのキーに PIN を設定してください。",
  "error.hardwareKeyNotRecognized":
    "このキーは、ここには登録されていません。別のキーまたはマスターパスワードをお試しください。",
  "error.hardwareKeyLimit": "登録できるキーの上限（8個）に達しています。先に1つ削除してください。",
  "error.hardwareKeyLast":
    "これは保管庫の唯一のキーです。削除する前に、別のキーを登録してください。",
  "error.itemTypeNotAllowed": "この保管庫に保存できるのは、ワンタイムコードのみです。",
  "error.otpScanUnsupported":
    "このシステムでは画面のスキャンは利用できません。代わりにスクリーンショットを貼り付けてください。",
  "error.otpQrNotFound": "QR コードが見つかりません。コード全体が見えていることを確認してから、もう一度お試しください。",
  "error.otpQrNotOtp":
    "この QR コードは、2段階認証のセットアップコードではありません。サイトの2段階認証設定ページにあるコードを探してください。",
  "error.otpQrMigration":
    "これは Google Authenticator のエクスポートコードで、Mosaic ではまだ読み取れません。各アカウントを、それぞれのサイトで設定し直すか、セットアップキーを入力してください。",
  "error.otpQrMultiple":
    "複数のセットアップコードが見えています。使いたいものだけを表示してから、もう一度お試しください。",
  "error.otpQrInvalid":
    "このセットアップコードは、Mosaic が対応していない設定を使用しています。代わりにセットアップキーを入力してください。",
  "error.otpUriInvalid": "有効な otpauth:// リンクではありません。リンク全体をもう一度コピーしてください。",
  "error.otpKeyInvalid":
    "このキーは無効です。使えるのは、英字の A〜Z と、数字の 2〜7 のみです。入力ミスがないか確認してください。",
  "error.otpImageUnreadable": "Mosaic はこの画像を読み取れません。PNG または JPEG のファイルを使用してください。",
  "error.otpImageTooLarge": "この画像は大きすぎます。QR コードだけのスクリーンショットを使用してください。",
  "error.otpClipboardNoImage":
    "クリップボードに画像がありません。QR コードのスクリーンショットを撮って、ここに貼り付けてください。",
  "error.otpPendingExpired": "スキャンしたコードは、もう利用できません。もう一度スキャンしてください。",
  "error.syncNotConfigured": "このデバイスでは、まだ同期が設定されていません。先に「設定」で設定してください。",
  "error.syncAlreadyConfigured":
    "同期はすでに設定されています。別のサーバーやフォルダを使うには、先に接続を解除してください。",
  "error.syncServerUnreachable":
    "Mosaic はサーバーに接続できませんでした。アドレスとインターネット接続を確認してから、もう一度お試しください。",
  "error.syncServerInvalid":
    "Mosaic のサーバーではないようです。アドレスを確認してください。https://sync.example.com のような形式です。",
  "error.syncServerInsecure":
    "Mosaic は、途中で誰にもデータを読み取られたり変更されたりしないよう、https のサーバーにのみ接続します。https:// のアドレスを使用してください。",
  "error.syncServerMismatch": "これは、このアカウントが同期しているサーバーではありません。アドレスを確認してください。",
  "error.syncServerIncompatible":
    "このサーバーのバージョンは、お使いの Mosaic と合いません。Mosaic を更新するか、サーバーの管理者に更新を依頼してください。",
  "error.syncRegistrationClosed":
    "このサーバーは、新しい同期アカウントを受け付けていません。既存のアカウントでサインインするか、サーバーの管理者に問い合わせてください。",
  "error.syncInviteInvalid":
    "この招待コードは無効か、すでに使用されています。コードを確認するか、サーバーの管理者に新しいコードを依頼してください。",
  "error.syncAccountExists":
    "このサーバーには、すでに同期アカウントがあります。代わりに「サインイン」を選んでください。",
  "error.syncAccountMismatch":
    "そのサーバーの同期アカウントは、別の Mosaic アカウントのものです。アドレスとメールアドレスを確認してください。",
  "error.syncTwoFactorRequired":
    "この同期アカウントには、認証アプリのコードが必要です。6桁のコードを入力してください。",
  "error.syncTwoFactorInvalid":
    "そのコードは正しくないか、有効期限が切れています。認証アプリに表示されている現在の6桁のコードを入力してください。",
  "error.syncSignedOut":
    "サーバーがこのデバイスをサインアウトさせました。同期を続けるには、もう一度サインインしてください。変更はこのデバイスに安全に保存されています。",
  "error.syncRateLimited": "試行回数が多すぎます。数分待ってから、もう一度お試しください。",
  "error.syncQuotaExceeded":
    "サーバーまたはフォルダに、データを保存する空きがありません。空き容量を確保するか、サーバーの管理者に増量を依頼してください。",
  "error.syncFolderUnavailable":
    "Mosaic はこのフォルダを使用できません。フォルダが存在すること、クラウドサービスの同期が完了していること、Mosaic が書き込めることを確認してください。",
  "error.syncFolderNotEmpty":
    "このフォルダには他のファイルがあるため、Mosaic はそこに書き込みません。空のフォルダを選ぶか、新しく作成してください。",
  "error.syncFolderNoAccount":
    "このフォルダには、このアカウントの同期データがありません。もう一方のデバイスが同期しているフォルダを選んでください。",
  "error.syncFolderDamaged":
    "このフォルダ内の同期ファイルの一部が破損しています。クラウドサービスの同期が完了するのを待つか、バックアップからフォルダを復元してから、もう一度お試しください。",
  "error.syncPasswordOutcomeUnknown":
    "サーバーがマスターパスワードを変更したか、確認できませんでした。どちらが有効か同期で確認できるまで、両方のパスワードを保管してください。",
  "error.syncWebauthnUnavailable":
    "お使いのサーバーでは、現在この操作にセキュリティキーを使用できないか、このアカウントにキーがありません。代わりに認証アプリのコードを使用してください。",
  "error.syncWebauthnLimit":
    "このアカウントには、サーバーが許可する上限のセキュリティキーがすでに登録されています。使っていないものを1つ削除してから、もう一度お試しください。",
  "error.syncHandoffExpired": "ブラウザでのステップがタイムアウトしました。もう一度開始してください。",
  "error.syncHandoffFailed":
    "ブラウザで、誤ったコードまたは失敗した確認が多すぎました。ステップをもう一度開始してください。",
  "error.syncPasswordNeedsServer":
    "マスターパスワードの変更には同期サーバーが必要ですが、接続できません。インターネット接続を確認して、もう一度お試しください。",
  "error.syncDeviceIsCurrent": "使用中のデバイスは削除できません。代わりに「接続を解除」を使用してください。",
  "error.syncChoiceExpired": "フォルダの選択がタイムアウトしました。もう一度フォルダを選んでください。",
  "error.syncFreshLoginExpired": "パスワードの確認がタイムアウトしました。もう一度マスターパスワードを入力してください。",
  "error.syncPasswordRollback": "サーバーが、より古いマスターパスワードを提示しました。サーバーを確認してください。",
  "error.syncPasswordChanged":
    "マスターパスワードが、サーバー上またはフォルダ内で変更されました。下に新しいものを入力してください。",
  "error.sharingNotHosted":
    "共有には Mosaic サーバーが必要です。先に「同期」設定でサーバーを接続してください。",
  "error.setupCodeNeedsServer": "スマートフォンが参加できるのは、同期しているアカウントのみです。",
  "error.sharingContactUnverified":
    "先にこの人の安全コードを照合してください。Mosaic は、確認済みの人とのみ共有します。",
  "error.sharingKeyChanged":
    "この人の鍵が変更されたか、コードが一致しません。もう一度この人を検索し、新しいコードを照合してから進んでください。",
  "error.sharingRoleForbidden":
    "この保管庫でのあなたの役割では、その操作はできません。オーナーまたは管理者に依頼してください。",
  "error.sharingInviteExists":
    "この人とこの保管庫への招待が、すでに待機中です。新しく送る場合は、先に取り消してください。",
  "error.sharingInviteStale":
    "鍵の変更などにより、その間に招待が変更されました。もう一度開いてください。",
  "error.sharingInviteExpired":
    "この招待は期限切れか、取り消されました。送信者に、もう一度招待を依頼してください。",
  "error.sharingInviteUnreadable":
    "この招待を開けません。破損しているか、このアカウント宛てではない可能性があります。送信者に、もう一度招待を依頼してください。",
  "error.sharingInviteNotInRoster":
    "保管庫のメンバーリストがこの招待と一致しないため、Mosaic は開きませんでした。送信者に、もう一度招待を依頼してください。",
  "error.sharingOwnerCannotLeave":
    "オーナーは保管庫から退出できません。代わりに、他の人を削除してください。",
  "error.sharingAlreadyMember": "この人は、すでにこの保管庫のメンバーです。",
  "error.sharingReinviteBlocked":
    "この人を最近削除しました。保管庫の鍵が変更された後に、もう一度招待できます。同期してから、もう一度お試しください。",
  "error.sharingInviteLimit": "待機中の招待が多すぎます。返事を待つか、いくつか取り消してください。",
  "error.sharingVaultKnown":
    "このデバイスには、その保管庫がすでにあるため、Mosaic はその鍵を受け付けませんでした。送信者に、どの保管庫を共有したか確認してください。",
  "error.syncHasSharedVaults":
    "このデバイスに共有保管庫がある間は、サーバーを切り替えられません。先に共有保管庫から退出するか、オーナーに削除してもらってください。",
  "error.emergencyWaiting":
    "リクエストが待機中のため、今は変更できません。リクエストを拒否するか、待ってください。",
  "error.emergencyCooldown":
    "前回のリクエストは拒否されました。1日の待機期間が過ぎれば、もう一度リクエストできます。",
  "error.emergencyNotGranted":
    "申請するものがありません。開いているリクエストがないか、相手があなたにアクセスを許可していません。",
  "error.shortcutUnavailable":
    "このショートカットは、別のプログラムがすでに使用しているか、Mosaic が登録できません。別の組み合わせを選んでください。",
  "error.shortcutUnsupported":
    "お使いのシステムでは、アプリがグローバルショートカットを設定できません。代わりに、ウィンドウから Mosaic を開いてください。",
} satisfies Record<keyof typeof en, string>;
