/**
 * Japanese UI strings: insights. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { insights as en } from "../en/insights.ts";

export const insights = {
  "insights.title": "セキュリティ",
  "insights.intro":
    "チェックはこのデバイス上で実行されます。最終チェック: {checkedAt, date, medium} {checkedAt, time, short}、対象は{count, plural, other {#件の項目}}です。",
  "insights.checkAgain": "もう一度チェック",
  "insights.summary": "サマリー",
  "insights.fixFirst": "まず対処してください",
  "insights.showAll": "{count, plural, other {#件の検出事項}}をすべて表示",
  "insights.empty": "問題は見つかりませんでした。このページを開くたびに、Mosaic が再チェックします。",
  "insights.kind.weak": "脆弱",
  "insights.kind.weak.description": "推測されやすい",
  "insights.kind.reused": "使い回し",
  "insights.kind.reused.description": "{groups, plural, other {#グループ}}",
  "insights.kind.old": "古い（1年以上）",
  "insights.kind.old.description": "1年以上変更されていない",
  "insights.kind.missingTotp": "2FA 未設定",
  "insights.kind.missingTotp.description": "サイトは対応していますが、設定されていません",
  "insights.kind.insecureUrl": "保護されていない HTTP",
  "insights.kind.insecureUrl.description": "パスワードが暗号化されずに送信されます",
  "insights.kind.breached": "漏えいあり",
  "insights.kind.breached.description": "任意の漏えいチェックで検出されました",
  "insights.finding.breached":
    "このパスワードは既知のデータ漏えいに含まれています。使用しているすべての場所で、今すぐ変更してください。",
  "insights.finding.weak": "推測されやすいパスワードです。生成したパスワードに変更してください。",
  "insights.finding.reused":
    "他の{count, plural, other {#件の項目}}でも使用されています。サイトごとに別のパスワードを使ってください。",
  "insights.finding.old": "1年以上変更されていません。このアカウントをまだ使っている場合は変更してください。",
  "insights.finding.missingTotp":
    "{site} は2段階認証コードに対応しています。サイト側でオンにして、コードを Mosaic に保存してください。",
  "insights.finding.insecureUrl":
    "保護されていない HTTP を使用しています: {url}。サイトが対応している場合は https:// に切り替えてください。",
  "insights.severity.high": "高",
  "insights.severity.medium": "中",
  "insights.severity.low": "低",
  "insights.action.changePassword": "パスワードを変更",
  "insights.action.setUpTotp": "2FA を設定",
  "insights.action.reviewUrl": "確認",
  "insights.attribution.twoFactor": "2FA データ: 2FA Directory (2fa.directory)、MIT ライセンス。",
  "insights.counts": "{count, plural, =0 {問題なし} other {#件の問題}}",
  "insights.documentation": "オンにする方法: {url}",
  "insights.actionFor": "{action}: {title}",
} satisfies Record<keyof typeof en, string>;
