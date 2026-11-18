/**
 * Japanese UI strings: vault. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { vault as en } from "../en/vault.ts";

export const vault = {
  "vault.itemCount": "{count, plural, =0 {項目なし} other {#件の項目}}",
  "vault.deleteConfirm": "「{title}」を削除しますか？30日間はゴミ箱から復元できます。",
  "vault.defaultName": "個人用",
  "vault.unnamed": "名前のない保管庫",
  "vault.vaults": "保管庫",
  "vault.allItems": "すべての項目",
  "vault.items": "項目",
  "vault.search": "保管庫を検索",
  "vault.untitled": "無題",
  "vault.noResults":
    "「{query}」に一致するものがありません。つづりを確認するか、ウェブサイトで検索してみてください。",
  "vault.new": "新しい項目",
  "vault.selectItem": "項目を選ぶと詳細が表示されます。",
  "vault.trashEmpty": "ゴミ箱は空です。削除した項目は30日間ここに残ります。",
  "vault.favorite": "お気に入り",
  "vault.unfavorite": "お気に入りから削除",
  "vault.moveTo": "保管庫に移動",
  "vault.allVaults": "すべての保管庫",
  "vault.category.favorites": "お気に入り",
  "vault.category.logins": "ログイン",
  "vault.category.passkeys": "パスキー",
  "vault.category.codes": "ワンタイムコード",
  "vault.category.cards": "カード",
  "vault.category.notes": "セキュアノート",
  "vault.category.ssh": "SSH キー",
  "vault.flag.otp": "2FA",
  "vault.moveToNamed": "{name} に移動",
  "vault.emptyTitle": "保管庫は空です",
  "vault.emptyBody":
    "ログイン、パスキー、カード、メモを追加してください。すべての内容は、同期される前にこのデバイスで暗号化されます。",
  "vault.addFirst": "最初の項目を追加",
  "vault.paletteTip.before": "いつでも",
  "vault.paletteTip.after": "を押すと、検索やコマンドの実行ができます。",
  "trash.retention": "削除した項目は30日間保持されます。",
  "trash.rowMeta": "削除日: {deletedAt, date, medium} · 残り{days, plural, other {#日}}",
  "trash.detailMeta": "{kind} · {vault} · 削除日: {deletedAt, date, medium}",
  "trash.purgeConfirm.title": "「{title}」を完全に削除しますか？",
  "trash.purgeConfirm.body": "この操作は元に戻せません。",
  "vault.hardware.placeholderName": "コード（セキュリティキー）",
  "vault.hardware.title": "この保管庫にはセキュリティキーが必要です",
  "vault.hardware.body":
    "マスターパスワードとシークレットキーを誰かに知られても、ここのコードは安全に保たれますが、セキュリティキーがないとコードを見ることはできません。",
  "vault.hardware.open": "セキュリティキーを使用",
  "vault.hardware.lockedNote": "ロック中です。他の保管庫はロック解除されたままです。",
  "vault.hardware.lockNow": "この保管庫をロック",
  "vault.hardware.closesIn": "{time}後にロックされます",
  "vault.hardware.backupTitle": "バックアップキーを登録",
  "vault.hardware.backupBody":
    "登録されているキーは1つです。2つをおすすめします。唯一のキーをなくすと、この保管庫を二度と開けなくなります。",
  "vault.hardware.createOption": "セキュリティキーで保護（コードのみ）",
  "vault.hardware.tradeoff":
    "この保管庫のコードには毎回セキュリティキーが必要なため、マスターパスワードが盗まれただけでは足りません。ただし、登録したキーをすべてなくすと、これらのコードも失われます。",
  "vault.hardware.removeNote":
    "キーを削除しても、この保管庫の鍵は変わりません。キーが盗まれた場合は、コードを新しい保管庫に移してください。",
  "vault.hardware.moveNote":
    "すでに使用しているコードをこの保管庫に移しても、以前のコピー（古い項目、その履歴、バックアップ、エクスポート）は保護されません。可能であれば、提供元で新しいシークレットを発行し、それを追加してください。",
  "vault.hardware.pinBody":
    "キーに PIN がある場合は入力し、キーが点滅したら触れてください。",
  "vault.hardware.keys": "キー",
  "vault.hardware.keysLabel": "この保管庫のキー",
  "vault.hardware.eyebrow": "ハードウェアで保護されたコード",
  "vault.hardware.selfLocked": "この保管庫は自動でロックされました。もう一度開くには、セキュリティキーを使用してください。",
  "vault.hardware.closingSoon":
    "この保管庫は30秒以内にロックされます。その後にもう一度開くには、セキュリティキーを使用してください。",
  "vault.hardware.keysTitle": "この保管庫のキー",
  "vault.hardware.codesOnly": "ワンタイムコードのみ",
  "vault.create": "新しい保管庫",
  "vault.create.name": "保管庫の名前",
  "vault.create.submit": "保管庫を作成",
  "vault.menu": "保管庫の操作",
  "vaultKey.menu": "保管庫の鍵を更新…",
  "vaultKey.title": "{vault} の鍵を更新",
  "vaultKey.body":
    "Mosaic はこの保管庫の新しい鍵を作成し、アクセス権のあるすべてのデバイスと人に渡します。すでに持っているコピーは引き続き読み取れますが、新しい鍵は今後の変更を保護します。誰かがアクセスできなくなった場合や、デバイスを紛失した場合にご利用ください。",
  "vaultKey.attachments": "添付ファイルの鍵も更新",
  "vaultKey.attachments.hint":
    "この保管庫内のすべてのファイルにも新しい鍵を設定します。ファイルが多い場合や大きい場合は、時間がかかります。",
  "vaultKey.reauth": "保管庫の鍵を更新するには、マスターパスワードが必要です。",
  "vaultKey.submit": "鍵を更新",
  "vaultKey.working": "鍵を更新しています…",
  "vaultKey.progress": "添付ファイルの鍵を更新中: {done} / {total}",
  "vaultKey.done": "保管庫の鍵を更新しました。",
  "vaultKey.doneAttachments":
    "保管庫の鍵を更新しました。添付ファイルの鍵も更新しました。",
  "vaultKey.unfinished":
    "保管庫の鍵を更新しました。添付ファイルの鍵の更新は後で続行されます。その前に Mosaic が閉じた場合は、次回「鍵の更新を続行」を選んでください。",
  "vaultKey.unreachable":
    "{count, plural, other {#件の添付ファイルがまだこのデバイスになく、古い鍵のままです。}}",
  "vaultKey.close": "閉じる",
  "vault.meta.title": "保管庫の詳細が短縮されました",
  "vault.meta.body":
    "2台のデバイスが「{vault}」の{fields}を同時に変更し、合わせると長すぎて保存できませんでした。Mosaic は一方の内容を残すか、欄を空にしました。内容を確認し、必要なら入力し直してください。",
  "vault.edit.title": "保管庫を編集",
  "vault.meta.edit": "保管庫を編集",
  "vault.meta.field.name": "名前",
  "vault.meta.field.icon": "アイコン",
  "vault.meta.field.description": "説明",
  "vault.meta.dismiss": "閉じる",
} satisfies Record<keyof typeof en, string>;
