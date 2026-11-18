/**
 * Japanese UI strings: importGuide. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { importGuide as en } from "../en/importGuide.ts";

export const importGuide = {
  "import.grid.title": "パスワードは今どこにありますか？",
  "import.grid.browsers": "ブラウザ",
  "import.grid.managers": "パスワードマネージャー",
  "import.grid.other": "その他",
  "import.source.chrome": "Chrome",
  "import.source.edge": "Edge",
  "import.source.brave": "Brave",
  "import.source.opera": "Opera",
  "import.source.vivaldi": "Vivaldi",
  "import.source.arc": "Arc",
  "import.source.firefox": "Firefox",
  "import.source.safari": "Safari",
  "import.source.applePasswords": "Apple パスワード",
  "import.source.onePassword": "1Password",
  "import.source.bitwarden": "Bitwarden",
  "import.source.keepass": "KeePass",
  "import.source.keepassxcCsv": "KeePassXC (CSV)",
  "import.source.enpass": "Enpass",
  "import.source.lastpass": "LastPass",
  "import.source.dashlane": "Dashlane",
  "import.source.keeper": "Keeper",
  "import.source.nordpass": "NordPass",
  "import.source.protonPass": "Proton Pass",
  "import.source.roboform": "RoboForm",
  "import.source.otherFile": "その他の CSV またはファイル",
  "import.guide.chrome.1":
    "Chrome でこのアドレスを開きます。コピーして、アドレスバーに貼り付けてください。",
  "import.guide.chrome.2":
    "「パスワードをエクスポート」の横にある「ファイルをダウンロード」をクリックし、コンピューターのパスワードで確認します。",
  "import.guide.chrome.3": "Chrome が「Chrome Passwords.csv」を「ダウンロード」フォルダに保存します。",
  "import.guide.edge.1": "Edge でこのアドレスを開きます。コピーして、アドレスバーに貼り付けてください。",
  "import.guide.edge.2":
    "「保存されたパスワード」の横にある「⋯」をクリックし、「パスワードのエクスポート」を選んで確認します。",
  "import.guide.edge.3": "Edge が CSV ファイルを「ダウンロード」フォルダに保存します。",
  "import.guide.brave.1": "Brave でこのアドレスを開きます。コピーして、アドレスバーに貼り付けてください。",
  "import.guide.brave.2":
    "「パスワードをエクスポート」の横にある「ファイルをダウンロード」をクリックし、コンピューターのパスワードで確認します。",
  "import.guide.brave.3": "Brave が CSV ファイルを「ダウンロード」フォルダに保存します。",
  "import.guide.opera.1": "Opera でこのアドレスを開きます。コピーして、アドレスバーに貼り付けてください。",
  "import.guide.opera.2":
    "「パスワードをエクスポート」の横にある「ファイルをダウンロード」をクリックし、コンピューターのパスワードで確認します。",
  "import.guide.opera.3": "Opera が CSV ファイルを「ダウンロード」フォルダに保存します。",
  "import.guide.vivaldi.1":
    "Vivaldi でこのアドレスを開きます。コピーして、アドレスバーに貼り付けてください。",
  "import.guide.vivaldi.2":
    "「パスワードをエクスポート」の横にある「ファイルをダウンロード」をクリックし、コンピューターのパスワードで確認します。",
  "import.guide.vivaldi.3": "Vivaldi が CSV ファイルを「ダウンロード」フォルダに保存します。",
  "import.guide.arc.1": "Arc でこのアドレスを開きます。コピーして、アドレスバーに貼り付けてください。",
  "import.guide.arc.2":
    "「パスワードをエクスポート」の横にある「ファイルをダウンロード」をクリックし、コンピューターのパスワードで確認します。",
  "import.guide.arc.3": "Arc が CSV ファイルを「ダウンロード」フォルダに保存します。",
  "import.guide.firefox.1":
    "Firefox でこのアドレスを開きます。コピーして、アドレスバーに貼り付けてください。",
  "import.guide.firefox.2":
    "右上の「⋯」をクリックし、「パスワードをエクスポート…」を選んで確認します。",
  "import.guide.firefox.3": "「logins.csv」を「ダウンロード」フォルダに保存します。",
  "import.guide.safari.1":
    "Safari →「設定」→「パスワード」（macOS 14 以前。macOS 15 以降では Apple パスワードを使用）。",
  "import.guide.safari.2": "リストの下にある「⋯」をクリックし、「すべてのパスワードを書き出す…」を選びます。",
  "import.guide.safari.3": "CSV ファイルを「ダウンロード」フォルダに保存します。",
  "import.guide.applePasswords.1": "パスワードアプリを開きます。",
  "import.guide.applePasswords.2": "「ファイル」→「すべてのパスワードをファイルに書き出す…」を選びます。",
  "import.guide.applePasswords.3": "CSV ファイルを「ダウンロード」フォルダに保存します。",
  "import.guide.onePassword.1": "1Password アプリを開き、ロックを解除します。",
  "import.guide.onePassword.2": "「ファイル」→「書き出す」→ 対象のアカウントを選び、パスワードを入力します。",
  "import.guide.onePassword.3": "「1PUX」を選び、「データを書き出す」をクリックします。",
  "import.guide.bitwarden.1": "ウェブ保管庫またはアプリで、「ツール」→「保管庫をエクスポート」を選びます。",
  "import.guide.bitwarden.2": "ファイル形式は「.json」を選びます（「暗号化」は選びません）。",
  "import.guide.bitwarden.3": "マスターパスワードで確認します。",
  "import.guide.keepass.1": "データベースファイル（.kdbx）をそのまま使います。",
  "import.guide.keepass.2": "パスワードは次のステップで入力します。パスワードはこのデバイスから出ません。",
  "import.guide.keepassxcCsv.1": "KeePassXC: 「データベース」→「エクスポート」→「CSV ファイル…」を選びます。",
  "import.guide.keepassxcCsv.2":
    "「ダウンロード」フォルダに保存します。.kdbx ファイルのほうが多くの情報を保持できるため、可能であれば「KeePass」を選んでください。",
  "import.guide.enpass.1": "Enpass: 「メニュー」→「ファイル」→「エクスポート」を選びます。",
  "import.guide.enpass.2":
    "保管庫を選び、形式に「.json」を選んで、マスターパスワードを入力します。",
  "import.guide.enpass.3": "「ダウンロード」フォルダに保存します。",
  "import.guide.lastpass.1": "lastpass.com にサインインし、「詳細オプション」→「エクスポート」を開きます。",
  "import.guide.lastpass.2": "マスターパスワードを入力します。CSV ファイルがダウンロードされます。",
  "import.guide.lastpass.3":
    "ブラウザにテキストが表示された場合は、そのページを「lastpass.csv」として保存してください。",
  "import.guide.dashlane.1": "ブラウザで Dashlane を開き、「マイアカウント」→「設定」→「データをエクスポート」を選びます。",
  "import.guide.dashlane.2": "「CSV」を選びます。Dashlane が ZIP ファイルをダウンロードします。",
  "import.guide.dashlane.3": "ZIP ファイルはそのままにしておきます。",
  "import.guide.keeper.1": "Keeper ウェブ保管庫で、「設定」→「エクスポート」を選びます。",
  "import.guide.keeper.2": "「JSON」を選んでエクスポートします（カスタムフィールドと2FAコードが保持されます）。",
  "import.guide.nordpass.1": "NordPass で、「設定」→「項目をエクスポート」を選びます。",
  "import.guide.nordpass.2":
    "マスターパスワードで確認し、CSV ファイルを「ダウンロード」フォルダに保存します。",
  "import.guide.protonPass.1": "Proton Pass で、「設定」→「エクスポート」を選びます。",
  "import.guide.protonPass.2":
    "「ZIP」を選び、暗号化はオフにします。Mosaic は PGP で暗号化されたエクスポートを読み取れません。",
  "import.guide.protonPass.3": "ロックを解除して、「ダウンロード」フォルダに保存します。",
  "import.guide.roboform.1": "RoboForm で、「設定」→「アカウントとデータ」→「エクスポート」を選びます。",
  "import.guide.roboform.2": "「CSV」と「ログイン」を選び、「ダウンロード」フォルダに保存します。",
  "import.guide.otherFile.1": "ヘッダー行のある CSV ファイル、または上記の形式のいずれかでエクスポートします。",
  "import.guide.otherFile.2": "ファイルを選択してください。Mosaic が列を対応付けます。",
  "import.guide.copyUrl": "アドレスをコピー",
  "import.guide.urlCopied": "アドレスをコピーしました。",
  "import.guide.choose": "エクスポートしたファイルを選択",
  "import.guide.dropHint": "…またはここにファイルを1つドロップします。このデバイス上でのみ読み取られます。",
  "import.guide.back": "戻る",
  "import.detect.detected": "認識しました: {source}",
  "import.detect.mismatch":
    "このファイルは {source} のエクスポートのようです。Mosaic はそのように読み取ります。",
  "import.detected.chromium": "Chrome、Edge、または他の Chromium ブラウザ",
  "import.detected.firefox": "Firefox",
  "import.detected.safari": "Safari または Apple パスワード",
  "import.detected.bitwardenCsv": "Bitwarden (CSV)",
  "import.detected.bitwarden": "Bitwarden",
  "import.detected.enpass": "Enpass",
  "import.detected.keepass": "KeePass",
  "import.detected.keepassxcCsv": "KeePassXC (CSV)",
  "import.detected.onePassword": "1Password",
  "import.detected.cxf": "FIDO Credential Exchange ファイル",
  "import.detected.lastpass": "LastPass",
  "import.detected.dashlane": "Dashlane",
  "import.detected.keeper": "Keeper",
  "import.detected.nordpass": "NordPass",
  "import.detected.protonPass": "Proton Pass",
  "import.detected.roboform": "RoboForm",
  "import.detected.csv": "CSV ファイル",
  "import.lost.title": "引き継げないもの",
  "import.lost.passkeys":
    "パスキー: {source} はエクスポートにパスキーを含めません。新しいパスキーを設定し終えるまで、{source} は残しておいてください。",
  "import.lost.attachments":
    "添付ファイル: {source} はエクスポートに添付ファイルを含めません。手動で追加してください。",
  "import.lost.history": "古いパスワード（履歴）: このエクスポートには含まれていません。",
  "import.lost.roboformExtras":
    "個人情報、セキュアノート、ブックマーク: 別途エクスポートしてください。ログインのファイルには含まれていません。",
  "import.lost.entriesSkipped":
    "{count, plural, other {#件のエントリを読み取れず、除外されます。}}",
  "import.lost.fieldsDropped":
    "Mosaic が保持できる数より多くのフィールドを持つ項目があります。超過分のフィールドは除外されます。",
  "import.lost.notesTruncated":
    "1つの項目に収まる長さを超えるメモがあります。収まらなかった部分は除外されます。",
  "import.lost.attachmentsSkipped":
    "見つからない、大きすぎる、または添付ファイルの保存容量に収まらない添付ファイルがあります。手動で追加してください。",
  "import.cleanup.title": "後片付け",
  "import.cleanup.delete": "{name} を今すぐ削除",
  "import.cleanup.deleted": "{name} を上書きして削除しました。",
  "import.cleanup.deleteFailed": "{name} を削除できませんでした。ご自身で削除してください。",
  "import.cleanup.keepFile":
    "読み込まれなかった項目、または完全には引き継げなかった項目があります。不足している内容を確認し終えるまで {name} を残しておき、その後ご自身で削除してください。",
  "import.cleanup.ssdNote":
    "SSD や同期フォルダでは、バージョン履歴、バックアップ、ゴミ箱などに古いコピーが残る場合があります。そちらも確認してください。",
  "import.cleanup.synced":
    "このファイルは {service} のフォルダにあります。{service} に以前のバージョンが残っている場合があるため、そちらも削除してください。",
  "import.synced.oneDrive": "OneDrive",
  "import.synced.dropbox": "Dropbox",
  "import.synced.googleDrive": "Google ドライブ",
  "import.synced.icloud": "iCloud",
  "import.synced.box": "Box",
  "import.checklist.title": "以前のアプリを手放す前に",
  "import.checklist.browserSaving":
    "{source} のパスワード保存と自動サインインをオフにして、Mosaic と並んで保存を提案されないようにしてください。",
  "import.checklist.firefoxSaving":
    "about:preferences#privacy を開き、「パスワードを保存するか確認する」のチェックを外して、Firefox が Mosaic と並んで保存を提案しないようにしてください。",
  "import.checklist.safariSaving":
    "Safari →「設定」→「自動入力」で「ユーザ名とパスワード」のチェックを外して、Safari が Mosaic と並んで入力しないようにしてください。",
  "import.checklist.oldExtension": "入力するアプリが1つだけになるよう、{source} のブラウザ拡張機能をオフにしてください。",
  "import.checklist.deleteOld":
    "Mosaic ですべてを確認したら、{source} のデータを削除してください。",
} satisfies Record<keyof typeof en, string>;
