/**
 * Japanese UI strings: credentialRotation. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { credentialRotation as en } from "../en/credentialRotation.ts";

export const credentialRotation = {
  "credentialRotation.tablist": "セキュリティの表示",
  "credentialRotation.tab.overview": "概要",
  "credentialRotation.tab.rotate": "今すぐ変更",
  "credentialRotation.tab.dueHidden": "、変更が必要: {count}件",
  "credentialRotation.queue.title": "今すぐ変更",
  "credentialRotation.queue.intro":
    "優先して変更したほうがよいパスワードです。Mosaic が案内しますが、各パスワードはご自身でサイト上で変更します。",
  "credentialRotation.queue.empty": "今すぐ変更するものはありません。",
  "credentialRotation.queue.count": "{count, plural, other {変更するパスワードが#件あります}}",
  "credentialRotation.queue.list": "変更するパスワード",
  "credentialRotation.queue.snoozedHeading": "スヌーズ中",
  "credentialRotation.queue.snoozedUntil": "{date, date, medium} までスヌーズ中",
  "credentialRotation.queue.reason.weak": "脆弱",
  "credentialRotation.queue.reason.reused": "使い回し",
  "credentialRotation.queue.reason.old": "古い",
  "credentialRotation.queue.reason.breached": "漏えいあり",
  "credentialRotation.queue.reason.unfinished": "変更が未完了",
  "credentialRotation.queue.change": "パスワードを変更",
  "credentialRotation.queue.continue": "変更を続ける",
  "credentialRotation.queue.editItem": "項目を編集",
  "credentialRotation.queue.manualHint":
    "この項目は Mosaic では案内できません。項目を開いて、ご自身で変更してください。",
  "credentialRotation.queue.actionFor": "{action}: {title}",
  "credentialRotation.queue.snooze": "スヌーズ",
  "credentialRotation.queue.snoozeFor": "{title} をスヌーズ",
  "credentialRotation.queue.snoozeDays": "{days, plural, other {#日}}",
  "credentialRotation.queue.snoozed": "{days, plural, other {#日}}間スヌーズしました。",
  "credentialRotation.queue.dismiss": "非表示にする",
  "credentialRotation.queue.dismissFor": "{title} を非表示にする",
  "credentialRotation.queue.dismissed": "非表示にしました。パスワードが変わると、再び表示されます。",
  "credentialRotation.queue.undo": "元に戻す",
  "credentialRotation.panel.title": "{title} のパスワードを変更",
  "credentialRotation.panel.step": "ステップ {current} / {total}",
  "credentialRotation.panel.stays":
    "{title} の変更は開いたままです。「うまくいった」または「うまくいかなかった」を選ぶまで、新しいパスワードは項目に保持されます。",
  "credentialRotation.panel.cancel": "キャンセル",
  "credentialRotation.steps.new.title": "新しいパスワードを選ぶ",
  "credentialRotation.steps.new.intro":
    "Mosaic は、新しいパスワードをどこかで使う前にこの項目へ保存するため、失われることはありません。確認するまで、現在のパスワードはそのまま残ります。",
  "credentialRotation.steps.new.label": "新しいパスワード",
  "credentialRotation.steps.new.use": "このパスワードを使う",
  "credentialRotation.steps.new.saving": "保存しています…",
  "credentialRotation.steps.site.title": "サイトで変更する",
  "credentialRotation.steps.site.pageLabel": "変更ページ",
  "credentialRotation.steps.site.body":
    "ブラウザでサイトのパスワードのページを開き、ツールバーの Mosaic をクリックして「変更フォームに入力」を選びます。Mosaic がフォームを送信することはありません。",
  "credentialRotation.steps.site.copyLink": "変更ページのリンクをコピー",
  "credentialRotation.steps.site.linkCopied": "リンクをコピーしました。",
  "credentialRotation.steps.site.linkHint":
    "リンクは秘密の情報ではありません。そのアドレスにページがない場合は、サイトのパスワード設定を手動で探してください。",
  "credentialRotation.steps.site.noLink":
    "このログインには https のアドレスがないため、Mosaic が提案できる変更ページがありません。サイトで手動でパスワードを変更してください。",
  "credentialRotation.steps.site.copyPassword": "新しいパスワードをコピー",
  "credentialRotation.steps.site.next": "試しました",
  "credentialRotation.steps.result.title": "サイトは新しいパスワードを受け付けましたか？",
  "credentialRotation.steps.done.failed": "現在のパスワードを維持しました",
  "credentialRotation.steps.done.notSure": "変更は開いたままです",
  "credentialRotation.steps.result.body":
    "Mosaic にはサイトの状態が見えません。この項目に正しいパスワードが残るよう、結果を教えてください。どちらの場合も、何も削除されません。",
  "credentialRotation.steps.result.working": "保存しています…",
  "credentialRotation.result.worked": "うまくいった",
  "credentialRotation.result.failed": "うまくいかなかった",
  "credentialRotation.result.notSure": "まだ分からない",
  "credentialRotation.result.workedDone":
    "完了しました。新しいパスワードが、この項目のパスワードになりました。古いパスワードは履歴にあります。",
  "credentialRotation.result.failedDone":
    "現在のパスワードを維持します。新しいパスワードは、サイトが実際には適用していた場合に備え、未確認として履歴に残ります。",
  "credentialRotation.result.notSureDone":
    "この項目の変更は開いたままです。結果が分かったら、戻ってきてください。",
  "credentialRotation.banner.title": "パスワードの変更を開始しました: {date, date, medium}",
  "credentialRotation.banner.prepared":
    "確認するまで、新しいパスワードはここに保持されます。まだどのページにも渡されていません。",
  "credentialRotation.banner.delivered":
    "確認するまで、新しいパスワードはここに保持されます。ブラウザ拡張機能がサイトのフォームに入力しました。",
  "credentialRotation.banner.reveal": "新しいパスワードを表示",
  "credentialRotation.banner.hide": "新しいパスワードを非表示",
  "credentialRotation.banner.copy": "新しいパスワードをコピー",
  "credentialRotation.banner.newPassword": "新しいパスワード",
  "credentialRotation.banner.copyLink": "変更ページのリンクをコピー",
  "credentialRotation.history.title": "パスワードの履歴",
  "credentialRotation.history.list": "以前のパスワード",
  "credentialRotation.history.entry": "置き換え日: {date, date, medium}",
  "credentialRotation.history.unconfirmed": "未確認",
  "credentialRotation.history.unconfirmedEntry": "試行日: {date, date, medium}",
  "credentialRotation.history.unconfirmedHint":
    "サイトがこのパスワードを適用した可能性があります。現在のパスワードでサインインできない場合は、復元してください。",
  "credentialRotation.history.reveal": "{date, date, medium} のパスワードを表示",
  "credentialRotation.history.hide": "{date, date, medium} のパスワードを非表示",
  "credentialRotation.history.restore": "復元",
  "credentialRotation.history.restoreFor": "{date, date, medium} のパスワードを復元",
  "credentialRotation.history.restored": "復元しました。直前のパスワードは履歴にあります。",
  "credentialRotation.policy.label": "このパスワードの変更を通知",
  "credentialRotation.policy.none": "しない",
  "credentialRotation.policy.every": "{days} 日ごと",
  "credentialRotation.policy.help":
    "リマインダーを設定すると、この項目が「今すぐ変更」リストに入ります。Mosaic は通知を送らず、そのための通信も行いません。",
  "credentialRotation.url.help":
    "任意です。このログインと同じサイトにある、サイトのパスワードページの https アドレスです。Mosaic は、ブラウザ拡張機能で「変更ページを開く」を選んだときにのみ開きます。",
  "credentialRotation.url.invalid":
    "Mosaic はこのアドレスを使用しません。https:// で始まり、このログインと同じサイトにある必要があります。",
  "credentialRotation.settings.title": "パスワードのリマインダー",
  "credentialRotation.settings.body":
    "ロック解除後に、変更したほうがよいパスワードの数を Mosaic がお知らせします。",
  "credentialRotation.settings.reminders": "ロック解除後に、変更するパスワードの数を表示",
  "credentialRotation.settings.remindersHint":
    "アプリ内のメッセージのみです。通知も、ネットワーク通信もありません。",
  "credentialRotation.reminder.toast": "{count, plural, other {変更するパスワードが#件あります}}",
  "credentialRotation.reminder.show": "表示",
  "credentialRotation.dialog.beginTitle": "パスワードの変更を開始しますか？",
  "credentialRotation.dialog.beginBody":
    "{browser} が、{host} の {title} のパスワードを変更しようとしています。Mosaic は新しいパスワードをこのログインに保持し、サイトが受け付けたことを確認するまで、現在のパスワードも保持します。",
  "credentialRotation.dialog.site": "ウェブサイト",
  "credentialRotation.dialog.login": "ログイン",
  "credentialRotation.dialog.password": "新しいパスワード",
  "credentialRotation.dialog.hidden": "非表示",
  "credentialRotation.dialog.show": "表示",
  "credentialRotation.dialog.hide": "非表示",
  "credentialRotation.dialog.regenerate": "別のものを生成",
  "credentialRotation.dialog.regenerated": "新しいパスワードを生成しました。",
  "credentialRotation.dialog.allow": "このパスワードを使う",
  "credentialRotation.dialog.cancel": "キャンセル",
  "credentialRotation.dialog.decideLater": "後で決める",
  "credentialRotation.dialog.untitled": "このログイン",
  "credentialRotation.dialog.attempt":
    "開始日時: {at, date, medium} {at, time, short} · {stage}{n, plural, =0 {} other { · 他に#件の変更が未完了}}",
  "credentialRotation.dialog.stagePrepared": "新しいパスワードはまだどのページにもありません",
  "credentialRotation.dialog.stageDelivered": "新しいパスワードをページに入力しました",
  "credentialRotation.dialog.resultTitle": "{host} は新しいパスワードを受け付けましたか？",
  "credentialRotation.dialog.resultBody":
    "{browser} は、変更がうまくいったと伝えています。確認すると、新しいパスワードが {title} のパスワードになり、現在のパスワードは履歴に移ります。",
  "credentialRotation.dialog.confirm": "うまくいった",
  "credentialRotation.dialog.keepOld": "古いパスワードを維持",
  "credentialRotation.dialog.failedTitle": "古いパスワードを維持しますか？",
  "credentialRotation.dialog.failedBody":
    "{browser} は、変更がうまくいかなかったと伝えています。Mosaic は {title} の現在のパスワードを維持します。サイトが実際には変更していた場合に備え、新しい値は履歴に残ります。",
  "credentialRotation.addOrigin.title": "{host} を {title} に追加しますか？",
  "credentialRotation.addOrigin.body":
    "Mosaic は、{host} でもこのログインを入力するようになります。{host} が本当に同じサイトの一部である場合にのみ追加してください。",
  "credentialRotation.addOrigin.idn":
    "このアドレスには特殊な文字が使われています。ブラウザでは {unicode} と表示される場合があります。慎重に確認してください。",
  "credentialRotation.addOrigin.add": "アドレスを追加",
  "credentialRotation.addOrigin.cancel": "今はしない",
} satisfies Record<keyof typeof en, string>;
