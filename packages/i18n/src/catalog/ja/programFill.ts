/**
 * Japanese UI strings: programFill. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { programFill as en } from "../en/programFill.ts";

export const programFill = {
  "programFill.chip.label": "Mosaic で入力",
  "programFill.chip.locked": "入力するには Mosaic のロックを解除",
  "programFill.list.title": "{program} のログイン",
  "programFill.list.none": "{program} のログインはまだありません",
  "programFill.list.signerChanged": "このプログラムの署名者が変更されました。もう一度リンクしますか？",
  "programFill.list.fileChanged": "リンクしてからこのプログラムが変更されました。もう一度リンクしますか？",
  "programFill.list.suggestions": "候補",
  "programFill.list.search": "ログインを検索",
  "programFill.list.link": "ログインをリンク…",
  "programFill.list.block": "このプログラムでは表示しない",
  "programFill.link.packaged": "{name} · ストアアプリ · 発行元 {publisher}",
  "programFill.link.packagedNoPublisher": "{name} · ストアアプリ",
  "programFill.link.signed": "{exe} · 署名者: {signer}（Windows が確認済み）",
  "programFill.link.unsigned": "{exe} · 署名なし · {path}",
  "programFill.link.unsignedNote":
    "Mosaic はこのファイルにのみ入力します。ファイルが変更された場合は、もう一度確認します。",
  "programFill.link.lookalike": "ご注意ください: この名前は、以前リンクした {similar} に似ています。",
  "programFill.link.hashChange": "リンク済みのファイル {old}…、このファイル {new}…",
  "programFill.link.login": "ログイン: {title}",
  "programFill.link.confirm": "リンクしてサインイン",
  "programFill.link.confirmFill": "リンクして入力",
  "programFill.link.confirmUnsigned": "この署名なしのプログラムをリンク",
  "programFill.link.confirmAgain": "もう一度確認してリンク",
  "programFill.notice.elevated":
    "Mosaic は、管理者として実行されているプログラムには入力できません。代わりに Mosaic からパスワードをコピーしてください。",
  "programFill.notice.noField": "ここにはログイン欄が見当たりません。",
  "programFill.notice.aborted":
    "ウィンドウが変わったため、Mosaic は入力を中止しました。入力欄を確認してください。",
  "programFill.notice.abortedCleared":
    "ウィンドウが変わったため、Mosaic は入力を中止し、入力済みの内容を削除しました。",
  "programFill.notice.modifiersHeld":
    "Shift、Ctrl、Alt、Windows キーを離してから、もう一度ログインを選んでください。",
  "programFill.notice.notResponding":
    "このプログラムが時間内に応答しませんでした。それ以上は入力されていません。",
  "programFill.notice.close": "閉じる",
  "programFill.settings.title": "プログラムへの入力",
  "programFill.settings.description":
    "Mosaic は、このデバイス上の他のプログラムでどの入力欄にフォーカスがあるかを確認し、そこにログインを表示します。何も保存・送信されません。",
  "programFill.settings.enable": "プログラムでログインを表示",
  "programFill.settings.submit": "入力後にサインイン",
  "programFill.settings.hotkey": "リストを開くショートカット",
  "programFill.settings.hotkeyNone": "ショートカットなし",
  "programFill.settings.hotkeySave": "リストのショートカットを保存",
  "programFill.settings.blocked": "Mosaic がログインを表示しないプログラム",
  "programFill.settings.blockedNone": "なし",
  "programFill.settings.unblock": "再び表示する",
  "programFill.settings.notWindows": "現時点では Windows のみ対応しています。",
  "programFill.settings.mosaicElevated":
    "Mosaic は管理者として実行されているため、プログラムには入力しません。通常どおり起動してください。",
  "programFill.settings.hostFailed":
    "Mosaic はプログラムの監視を開始できませんでした。Mosaic を再起動して、もう一度お試しください。",
  "programFill.card.title": "プログラムでもログインを入力",
  "programFill.card.turnOn": "オンにする",
  "programFill.card.notNow": "今はしない",
  "programFill.item.program": "プログラム",
  "programFill.item.storeApp": "ストアアプリ",
  "programFill.item.signedBy": "署名者: {signer}",
  "programFill.item.unsigned": "署名なし: {path}",
  "programFill.item.submit": "入力後にサインイン",
  "programFill.item.unlink": "リンクを解除",
} satisfies Record<keyof typeof en, string>;
