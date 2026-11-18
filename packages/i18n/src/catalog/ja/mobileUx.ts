/**
 * Japanese UI strings: mobileUx. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { mobileUx as en } from "../en/mobileUx.ts";

export const mobileUx = {
  "mobile.sync.warningsBanner": "同期で問題が見つかりました。設定を開いて確認してください。",
  "mobile.provider.errorTitle": "Mosaic は続行できません",
  "mobile.onboarding.details.keyWiped":
    "アプリを離れたため、Mosaic はスキャンしたシークレットキーを削除しました。もう一度コードをスキャンするか、キーを入力してください。",
  "mobile.common.showField": "{field} を表示",
  "mobile.common.hideField": "{field} を非表示",
  "mobile.error.storeInUse":
    "このスマートフォンの別の場所で Mosaic が使用中です。しばらく待ってから、もう一度お試しください。",
  "mobile.error.noAccount": "このスマートフォンでは Mosaic がまだ設定されていません。Mosaic を開いて設定してください。",
  "mobile.error.wrongCredentials":
    "マスターパスワードまたはシークレットキーが正しくありません。入力ミスやキーボードの言語を確認して、もう一度お試しください。",
  "mobile.error.invalidInput":
    "入力内容に無効なものがあります。入力を確認して、もう一度お試しください。",
  "mobile.error.vaultUnavailable":
    "現在この保管庫を開けません。同期してからもう一度お試しいただくか、Mosaic のロックをもう一度解除してください。",
  "mobile.unlock.quickOptionTouchId": "次回から Touch ID でロック解除",
  "mobile.detail.readCode": "コードを読み取る",
  "mobile.detail.labelValue": "{label}: {value}",
  "mobile.autofill.warningWithName": "先にアドレスを確認してください | {name}",
  "mobile.autofill.linkEntry": "{login} をリンクして入力",
  "mobile.link.imported": "このアプリ用に読み込み済み",
  "mobile.link.confirmTitle": "{login} を {app} にリンクしますか？",
  "mobile.link.confirmSubtitle": "アプリ ID {appId} · 署名 {fingerprint}",
  "mobile.link.confirmFallbackHint": "リンク後、もう一度フィールドをタップして入力してください。",
  "mobile.link.importedNotVerified": "読み込んだログインです。このアプリはウェブサイトに確認されていません。",
} satisfies Record<keyof typeof en, string>;
