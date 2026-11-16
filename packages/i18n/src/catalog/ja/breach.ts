/**
 * Japanese UI strings: breach. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { breach as en } from "../en/breach.ts";

export const breach = {
  "breach.card.title": "漏えいチェック",
  "breach.card.off": "オフです。オンにしない限り、Mosaic が Have I Been Pwned に接続することはありません。",
  "breach.card.about":
    "パスワードが既知のデータ漏えいに含まれていないか確認します。パスワードごとに、変換したコードのうち先頭5文字だけがこのデバイスから送信されます。",
  "breach.mode.label": "既知のデータ漏えいと照合",
  "breach.mode.off": "オフ",
  "breach.mode.manual": "手動で実行したときのみ",
  "breach.mode.weekly": "ロック解除後に週1回",
  "breach.card.checkNow": "今すぐチェック",
  "breach.card.stop": "チェックを停止",
  "breach.card.turnOff": "オフにする",
  "breach.progress.starting": "パスワードをチェックしています…",
  "breach.progress.step": "チェック中… {done} / {total}",
  "breach.result.never": "まだチェックしていません。",
  "breach.result.last":
    "最終チェック: {date, date, medium}。{found, plural, =0 {漏えいで見つかったパスワードはありません。} other {漏えいで見つかったパスワードが#件あります。}}",
  "breach.result.finished":
    "{checked, plural, other {#件のパスワード}}をチェックしました。{found, plural, =0 {漏えいで見つかったものはありません。} other {#件が漏えいで見つかりました。}}",
  "breach.result.upToDate": "すべて最近チェック済みです。新たに調べる項目はありません。",
  "breach.error.offline":
    "Have I Been Pwned に接続できませんでした。前回の結果を表示しています。オンラインになってからもう一度お試しください。",
  "breach.error.rateLimited":
    "Have I Been Pwned から、アクセスの間隔をあけるよう求められました。数分後にもう一度お試しください。",
  "breach.error.protocol":
    "Have I Been Pwned から Mosaic が信頼できない応答が返ったため、一部のパスワードはチェックされていません。しばらくしてからもう一度お試しください。",
  "breach.error.cancelled": "チェックを停止しました。完了した結果は保持されます。",
  "breach.consent.title": "パスワードを既知のデータ漏えいと照合しますか？",
  "breach.consent.body":
    "Mosaic は各パスワードを元に戻せないコード（ハッシュ）に変換し、そのコードの先頭5文字だけを Have I Been Pwned (api.pwnedpasswords.com) に送信します。このサービスが把握するのは、IP アドレスとその5文字の断片のみです。パスワード、メールアドレス、パスワードを使うサイトが知られることはありません。結果はこのデバイスにのみ保存されます。",
  "breach.consent.detail":
    "同じ5文字の断片を持つパスワードは他にも多数あるため、回答からあなたのパスワードが分かることはありません。ただし、別のサービスへのリクエストであることに変わりはないため、同意するまではオフのままです。いつでもオフにできます。",
  "breach.consent.agree": "オンにして今すぐチェック",
  "breach.consent.decline": "今はしない",
} satisfies Record<keyof typeof en, string>;
