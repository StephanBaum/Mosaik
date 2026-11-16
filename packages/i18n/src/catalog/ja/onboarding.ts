/**
 * Japanese UI strings: onboarding. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { onboarding as en } from "../en/onboarding.ts";

export const onboarding = {
  "onboarding.secretKey.title": "あなたのシークレットキー",
  "onboarding.secretKey.body":
    "保管庫のロックを解除するには、シークレットキーとマスターパスワードの両方が必要です。どちらも、私たちには復元できません。",
  "onboarding.secretKey.verify": "保存したことを確認するため、シークレットキーの最後のグループを入力してください。",
  "onboarding.secretKey.mistyped":
    "一致しません。緊急キットの最後のグループを確認してください。",
  "onboarding.step": "ステップ {current} / {total}",
  "onboarding.create.title": "アカウントを作成",
  "onboarding.create.body":
    "保管庫のロックを解除するには、マスターパスワードと、このデバイスで生成されるシークレットキーの両方が必要です。どちらもサーバーに送信されることはありません。",
  "onboarding.create.email": "メールアドレス",
  "onboarding.create.password": "マスターパスワード",
  "onboarding.create.passwordHint": "関連のない4つの単語の組み合わせがおすすめです。Mosaic 専用にしてください。",
  "onboarding.create.confirm": "マスターパスワードを確認",
  "onboarding.create.mismatch":
    "2つのパスワードが一致しません。両方の欄に同じマスターパスワードを入力してください。",
  "onboarding.create.submit": "アカウントを作成",
  "onboarding.create.working": "アカウントを作成しています。数秒かかります。",
  "onboarding.kit.accountId": "アカウント ID",
  "onboarding.kit.secretKey": "シークレットキー",
  "onboarding.kit.lastGroup": "シークレットキーの最後のグループ",
  "onboarding.kit.confirm": "確認",
  "kit.title": "緊急キットを保存",
  "kit.body":
    "緊急キットは、シークレットキーを記載した PDF です。マスターパスワードと合わせて、新しいデバイスで保管庫を開くために使います。このコンピューターから離れた安全な場所に保管してください。",
  "kit.secretKey": "あなたのシークレットキー",
  "kit.save": "緊急キットを保存（PDF）",
  "kit.saved": "保存しました。ファイルは安全な場所に保管してください。",
  "kit.cancelled": "まだ保存されていません。続けるには緊急キットを保存してください。",
  "kit.recover.alreadySaved": "すでに保存しました",
  "kit.recover.title": "緊急キットをもう一度保存",
  "kit.recover.body":
    "緊急キットがまだ確認されていません。マスターパスワードを入力してもう一度保存し、シークレットキーの最後のグループを確認してください。",
  "onboarding.import.title": "パスワードを引き継ぐ",
  "onboarding.import.body":
    "Chrome、Edge、Firefox、Safari などのブラウザや、1Password、Bitwarden、LastPass、Dashlane、Keeper、NordPass、Proton Pass、RoboForm、KeePass、Enpass から引き継げます。所要時間は数分です。後から設定で行うこともできます。",
  "onboarding.import.start": "他のアプリから読み込む",
  "onboarding.import.skip": "今はスキップ",
  "onboarding.done.title": "準備が整いました",
  "onboarding.done.body": "保管庫の準備ができました。ロックボタンでいつでもロックできます。",
  "onboarding.browser.title": "ブラウザ拡張機能をインストール",
  "onboarding.browser.body":
    "ブラウザでログインを入力します。拡張機能は、クリックしたときやキーボードショートカットを使ったときにのみ入力します。",
  "onboarding.browser.steps": "ブラウザを接続する方法",
  "onboarding.browser.step1": "ブラウザに Mosaic 拡張機能をインストールします。",
  "onboarding.browser.step2": "拡張機能を開き、「接続」を選びます。",
  "onboarding.browser.step3":
    "コードが一致していることを確認し、ここで開くウィンドウで「許可」を選びます。",
  "onboarding.browser.notInStores":
    "拡張機能はまだブラウザのストアにありません。後から「設定」→「ブラウザ」で接続できます。",
  "onboarding.browser.getFor": "{browser} 版を入手",
  "onboarding.browser.connectedList": "接続済みのブラウザ",
  "onboarding.browser.waiting": "拡張機能を待っています。接続済みのブラウザはまだありません。",
  "onboarding.browser.connected": "{name} を接続しました",
  "onboarding.browser.done": "完了",
  "onboarding.browser.skip": "今はスキップ",
  "onboarding.progress": "セットアップの進行状況",
  "onboarding.stepName.create": "アカウント作成",
  "onboarding.stepName.kit": "緊急キット",
  "onboarding.stepName.verify": "キットを確認",
  "onboarding.stepName.import": "読み込み（任意）",
  "onboarding.stepName.browser": "ブラウザ拡張機能（任意）",
  "kit.preview": "緊急キットのプレビュー",
  "kit.continueHint": "続けるには緊急キットを保存してください。",
  "kit.warning":
    "このキットとマスターパスワードがなければ、誰にも保管庫を復元できません。私たちにもできません。",
  "onboarding.join.link": "すでに Mosaic を使っています",
  "onboarding.join.title": "このデバイスをアカウントに追加",
  "onboarding.join.body":
    "マスターパスワードと、緊急キットに記載のシークレットキーでサインインします。その後、保管庫がこのデバイスにダウンロードされ、復号されます。",
  "onboarding.join.tabs": "同期データはどこにありますか？",
  "onboarding.join.tab.server": "Mosaic サーバー",
  "onboarding.join.tab.folder": "クラウドフォルダ",
  "onboarding.join.serverUrl": "サーバーアドレス",
  "onboarding.join.email": "同期アカウントのメールアドレス",
  "onboarding.join.password": "マスターパスワード",
  "onboarding.join.secretKey": "シークレットキー",
  "onboarding.join.secretKeyHint": "緊急キットに記載されています。M1 で始まります。",
  "onboarding.join.deviceName": "このデバイスの名前",
  "onboarding.join.chooseFolder": "フォルダを選択…",
  "onboarding.join.folder": "フォルダ",
  "onboarding.join.noAccounts":
    "このフォルダには Mosaic のアカウントがありません。もう一方のデバイスが同期しているフォルダを選んでください。",
  "onboarding.join.account": "アカウント",
  "onboarding.join.accountOption": "アカウント {id}",
  "onboarding.join.submit": "このデバイスを追加",
  "onboarding.join.working": "サインインして保管庫をダウンロードしています…",
  "onboarding.join.twoFactor.title": "コードを入力",
  "onboarding.join.twoFactor.body":
    "同期アカウントでは2段階認証が使われています。認証アプリに表示される6桁のコードを入力してください。",
  "onboarding.join.twoFactor.code": "6桁のコード",
  "onboarding.join.twoFactor.submit": "確認",
  "onboarding.join.back": "代わりに新しいアカウントを作成",
} satisfies Record<keyof typeof en, string>;
