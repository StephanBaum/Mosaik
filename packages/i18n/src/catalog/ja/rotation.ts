/**
 * Japanese UI strings: rotation. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { rotation as en } from "../en/rotation.ts";

export const rotation = {
  "rotation.section.title": "パスワードと鍵",
  "rotation.section.body":
    "マスターパスワードの変更、新しいシークレットキーの取得、アカウント全体の新しい鍵への移行ができます。何かに手を加える前に、必ず何が起こるかを表示します。",
  "rotation.action.password": "マスターパスワードを変更",
  "rotation.action.password.body":
    "新しいマスターパスワードを選びます。シークレットキーと保管庫は、そのままです。",
  "rotation.action.password.button": "変更…",
  "rotation.action.secretKey": "シークレットキーを更新",
  "rotation.action.secretKey.body":
    "新しいシークレットキーと、新しい緊急キットを取得します。キットを紛失した場合や、誰かに見られた可能性がある場合にご利用ください。",
  "rotation.action.secretKey.button": "更新…",
  "rotation.action.profile": "保護を強化",
  "rotation.action.profile.body":
    "試行のたびにより多くのメモリを使うことで、マスターパスワードの推測を難しくします。ロック解除に少し時間がかかるようになります。",
  "rotation.action.profile.button": "強化…",
  "rotation.action.rekey": "アカウントの鍵を再作成",
  "rotation.action.rekey.body":
    "アカウントのすべての鍵（新しいシークレットキー、新しい保管庫の鍵、新しい安全コード）を更新します。誰かがパスワードとシークレットキーに加えて保管庫のデータもコピーした可能性がある場合にご利用ください。",
  "rotation.action.rekey.button": "再作成…",
  "rotation.action.rekey.needsSync":
    "鍵の再作成では、同期サーバーまたはフォルダ上のコピーも更新するため、同期が必要です。先に同期を設定してください。",
  "rotation.title.password": "マスターパスワードを変更",
  "rotation.title.secretKey": "シークレットキーを更新",
  "rotation.title.profile": "保護を強化",
  "rotation.title.rekey": "アカウントの鍵を再作成",
  "rotation.step": "ステップ {current} / {total}",
  "rotation.back": "戻る",
  "rotation.stop.title": "この変更を中止しますか？",
  "rotation.stop.body": "先ほど保存した緊急キットは使えなくなります。古いキットは保管しておいてください。",
  "rotation.stop.confirm": "変更を中止",
  "rotation.stop.keep": "続ける",
  "rotation.next": "続ける",
  "rotation.close": "閉じる",
  "rotation.review.title.password": "パスワードを変更する前に",
  "rotation.review.title.secretKey": "シークレットキーを更新する前に",
  "rotation.review.title.profile": "保護を強化する前に",
  "rotation.review.title.rekey": "何かが漏えいしましたか？",
  "rotation.review.changes": "変わるもの",
  "rotation.review.stays": "変わらないもの",
  "rotation.review.password.changes":
    "マスターパスワードが変わります。このデバイスのクイックロック解除はオフになりますが、後で設定し直せます。",
  "rotation.review.password.stays":
    "シークレットキー、保管庫と項目、安全コード、緊急連絡先は変わりません。",
  "rotation.review.password.devices":
    "他のデバイスは同期からサインアウトされます。それぞれで新しいマスターパスワードを1回入力すると、同期が再開します。",
  "rotation.review.secretKey.changes":
    "シークレットキーと緊急キットが変わります。このデバイスのクイックロック解除はオフになりますが、後で設定し直せます。",
  "rotation.review.secretKey.stays":
    "マスターパスワード、保管庫と項目、安全コード、緊急連絡先は変わりません。登録済みのセキュリティキーは引き続き使えます。",
  "rotation.review.secretKey.devices":
    "他のデバイスは同期からサインアウトされます。それぞれで、パスワードと新しいキットの新しいシークレットキーを1回入力してください。",
  "rotation.review.secretKey.limit":
    "これにより、古いシークレットキーでのサインインはできなくなります。ただし、古いパスワードとシークレットキーとともに保管庫のデータをすでにコピーした人は、引き続きそれを読み取れます。それを止めるには、アカウントの鍵を再作成してください。",
  "rotation.review.secretKey.compromised": "何かが漏えいした場合: 代わりに鍵を再作成",
  "rotation.review.profile.changes":
    "ロック解除を1回試すたびに必要なメモリ量が変わります。これは、あなたにも、推測する人にも当てはまります。このデバイスのクイックロック解除はオフになりますが、後で設定し直せます。",
  "rotation.review.profile.stays":
    "マスターパスワード、シークレットキー、保管庫、安全コードは変わりません。登録済みのセキュリティキーは引き続き使えます。",
  "rotation.review.profile.devices":
    "他のデバイスは同期からサインアウトされます。それぞれでマスターパスワードを1回入力すると、同期が再開します。処理の遅いデバイスは、ロック解除に少し時間がかかります。",
  "rotation.review.rekey.intro":
    "鍵の再作成は、誰かが保管庫のデータに加えてパスワードとシークレットキーも入手した可能性がある場合のためのものです。デバイスを紛失しただけであれば、シークレットキーの更新かパスワードの変更で十分です。",
  "rotation.review.rekey.yes": "はい、アカウントの鍵を再作成する",
  "rotation.review.rekey.no": "いいえ、代わりにシークレットキーだけ更新する",
  "rotation.review.rekey.limit":
    "鍵を再作成しても、すでにコピーされたものを取り戻すことはできません。サーバーやバックアップ上の古いコピーは古い鍵で暗号化されたままで、古い鍵があれば開けてしまいます。",
  "rotation.review.rekey.title": "鍵の再作成で行われること",
  "rotation.review.rekey.vaults": "{count, plural, other {あなただけが使う#個の保管庫に新しい鍵が設定されます。}}",
  "rotation.review.rekey.keys":
    "新しいシークレットキー、新しい緊急キット、新しい安全コード、新しいアカウントキーが作成されます。",
  "rotation.review.rekey.devices":
    "他のデバイスはサインアウトされ、それぞれで新しいシークレットキーと新しいマスターパスワードを入力するまで、同期できません。",
  "rotation.review.rekey.shared":
    "{count, plural, other {#個の保管庫を共有しています。}}それぞれに新しい鍵が設定されます。",
  "rotation.review.rekey.sharedRow": "{name}、{count, plural, other {#人のメンバー}}",
  "rotation.review.rekey.sharedMembers":
    "共有している各相手に、あなたの新しい安全コードを照合するよう依頼してください。照合されるまで、その人はあなたから新しい鍵を受け取れず、更新された保管庫を開けません。",
  "rotation.review.rekey.memberVaults":
    "{count, plural, other {#個の保管庫は他の人のものです。}}それらのオーナーに、更新を依頼します。更新されるまでは、あなたの古い鍵でも読み取れます。",
  "rotation.review.rekey.grants":
    "{count, plural, other {#人が、あなたの保管庫への緊急アクセス権を持っています。}}そのアクセス権は新しい鍵用に更新されます。それぞれの人も、あなたの新しい安全コードを照合する必要があります。",
  "rotation.review.rekey.quick":
    "このデバイスのクイックロック解除はオフになります。後で設定し直せます。",
  "rotation.review.rekey.hardwareKeys":
    "{count, plural, other {ロック解除用に登録した#個のセキュリティキーはオフになります。}}後で登録し直せます。",
  "rotation.review.rekey.hardwareVaults":
    "{count, plural, other {ハードウェアで保護された#個の保管庫は影響を受けません。}}",
  "rotation.review.rekey.attachments":
    "{count, plural, other {#件の添付ファイルがこのデバイスにありません。}}再び保護できるよう、先にダウンロードされます。取得できなかったものは古い鍵のままです。",
  "rotation.review.rekey.folder":
    "同期フォルダに新しいアカウントファイルが作成されます。他のデバイスは、鍵が変更されたことを示すメモをそこで見つけます。",
  "rotation.review.rekey.unavailable": "鍵の再作成は利用できません。同期が設定されていません。",
  "rotation.review.loading": "鍵の再作成が影響する範囲を確認しています…",
  "rotation.review.rekey.needsClean":
    "鍵の再作成には、同期が最新で、警告がない状態であることが必要です。今すぐ同期し、警告があれば確認してから、もう一度お試しください。",
  "rotation.credentials.title": "ご本人であることを確認",
  "rotation.credentials.password.title": "マスターパスワード",
  "rotation.credentials.current": "現在のマスターパスワード",
  "rotation.credentials.new": "新しいマスターパスワード",
  "rotation.credentials.confirm": "新しいマスターパスワードをもう一度入力",
  "rotation.credentials.mismatch": "2つのパスワードが異なります。",
  "rotation.credentials.same": "新しいマスターパスワードは、現在のものと異なる必要があります。",
  "rotation.credentials.hint":
    "短くて複雑なものより、長いパスフレーズのほうが優れています。復元はできません。Mosaic はこのパスワードを知りません。",
  "rotation.credentials.code": "認証アプリの6桁のコード",
  "rotation.credentials.profile": "保護レベル",
  "rotation.credentials.profile.standard": "標準: 1回の試行につきメモリ 256 MB",
  "rotation.credentials.profile.strong": "強力: 1回の試行につきメモリ 512 MB",
  "rotation.credentials.profile.max": "最大: 1回の試行につきメモリ 1 GB",
  "rotation.credentials.profile.slow":
    "このレベルでは、このデバイスでのロック解除に2秒以上かかります。他の古いデバイスでは、さらに遅くなります。",
  "rotation.credentials.profile.force": "それでも強化する",
  "rotation.kit.title": "新しい緊急キットを保存",
  "rotation.kit.body":
    "新しいシークレットキーは、このキットに記載されています。先に保存してください。キットがないと新しいデバイスでサインインできないため、Mosaic は、入手したことを確認してからアカウントを変更します。",
  "rotation.kit.save": "新しい緊急キットを保存",
  "rotation.kit.saved": "保存しました。デバイスとは別の、安全な場所に保管してください。",
  "rotation.kit.cancelled": "まだ何も保存されていません。",
  "rotation.kit.verify": "保存したキットにある、新しいシークレットキーの最後のグループを入力してください。",
  "rotation.kit.confirm": "キットを確認",
  "rotation.kit.mistyped": "新しいシークレットキーの末尾と一致しません。保存したキットを確認してください。",
  "rotation.kit.verified": "キットを確認しました。",
  "rotation.confirm.title.password": "マスターパスワードを変更",
  "rotation.confirm.title.secretKey": "シークレットキーを更新",
  "rotation.confirm.title.profile": "保護を強化",
  "rotation.confirm.title.rekey": "アカウントの鍵を再作成",
  "rotation.confirm.body.rekey":
    "他のデバイスがサインアウトされ、すべての鍵が更新されます。完了するまで Mosaic を開いたままにしてください。",
  "rotation.confirm.body.secretKey":
    "他のデバイスが同期からサインアウトされます。完了するまで Mosaic を開いたままにしてください。",
  "rotation.confirm.verb.password": "パスワードを変更",
  "rotation.confirm.verb.secretKey": "シークレットキーを更新",
  "rotation.confirm.verb.profile": "保護を強化",
  "rotation.confirm.verb.rekey": "鍵を再作成",
  "rotation.run.title": "鍵を更新しています…",
  "rotation.run.keepOpen": "Mosaic を開いたままにしてください。添付ファイルが多い場合は、数分かかることがあります。",
  "rotation.progress.label": "進行状況",
  "rotation.progress.count": "{done} / {total}",
  "rotation.progress.done": "完了",
  "rotation.phase.prepared": "準備中",
  "rotation.phase.backendStarted": "アカウントに新しい鍵への切り替えを依頼しています",
  "rotation.phase.backendAccepted": "アカウントが新しい鍵を受け付けました",
  "rotation.phase.folderWritten": "同期フォルダに新しいアカウントファイルが作成されました",
  "rotation.phase.localCommitted": "このデバイスは新しい鍵を使用しています",
  "rotation.phase.attachments": "添付ファイルを再び保護しています",
  "rotation.phase.uploading": "保護したデータを送信しています",
  "rotation.phase.done": "完了しました",
  "rotation.done.title.password": "マスターパスワードを変更しました",
  "rotation.done.title.secretKey": "シークレットキーを更新しました",
  "rotation.done.title.profile": "保護を強化しました",
  "rotation.done.title.rekey": "アカウントの鍵を再作成しました",
  "rotation.done.body.password":
    "今後は新しいマスターパスワードを使用してください。他のデバイスでは、1回入力を求められます。",
  "rotation.done.body.secretKey":
    "新しい緊急キットは安全に保管し、古いものは削除してください。他のデバイスでは、新しいシークレットキーの入力を1回求められます。",
  "rotation.done.body.profile":
    "ロック解除に、より多くのメモリが必要になりました。他のデバイスでは、パスワードの入力を1回求められます。",
  "rotation.done.body.rekey":
    "新しい緊急キットは安全に保管し、古いものは削除してください。他のデバイスでは、新しいシークレットキーと新しいマスターパスワードの入力を1回求められます。",
  "rotation.done.members":
    "保管庫を共有しているすべての人に、あなたの新しい安全コードを照合するよう依頼してください。照合されるまで、その人はあなたから新しい鍵を受け取れません。",
  "rotation.done.quick": "クイックロック解除をもう一度設定",
  "rotation.done.quick.body": "クイックロック解除がオフになりました。",
  "rotation.done.hardware": "セキュリティキーをもう一度登録",
  "rotation.done.hardware.body":
    "{count, plural, other {ロック解除用の#個のセキュリティキーがオフになりました。}}",
  "rotation.done.unreachable.note":
    "これらのファイルは読み取れず、再暗号化されませんでした。削除するか追加し直すまで古いファイルキーのままのため、古い鍵とファイルのコピーを持つ人は、開けてしまう可能性があります。",
  "rotation.done.unreachable.file": "{item}: {file}",
  "rotation.done.unreachable.unknownItem": "不明な項目",
  "rotation.done.unreachable.unknownFile": "不明なファイル",
  "rotation.done.unreachable.more": "{count, plural, other {ほか#個のファイル}}",
  "rotation.done.unfinished":
    "最後の部分は後で続行されます。完了する前に Mosaic が閉じた場合は、次回「鍵の更新を続行」を選んでください。",
  "rotation.resume.title": "鍵の変更が未完了です",
  "rotation.resume.body.password":
    "鍵の変更が中断されました。失われたものはありません。「続ける」を選んでパスワードをもう一度入力するか、破棄してください。",
  "rotation.resume.body.secretKey":
    "シークレットキーの更新が中断されました。失われたものはありません。続けて完了してください。",
  "rotation.resume.body.rekey":
    "アカウントの鍵の再作成が中断されました。失われたものはありません。続けて完了してください。",
  "rotation.resume.body.profile":
    "保護の強化が中断されました。失われたものはありません。続けて完了してください。",
  "rotation.resume.continue": "鍵の更新を続行",
  "rotation.resume.discard": "破棄…",
  "rotation.resume.running": "続行しています…",
  "rotation.resume.progress": "{done} / {total} 完了。終了するまで Mosaic を開いたままにしてください。",
  "rotation.resume.dialog.title": "鍵の更新を続行",
  "rotation.resume.dialog.password": "現在のマスターパスワード",
  "rotation.resume.dialog.group": "新しいシークレットキーの最後のグループ",
  "rotation.resume.dialog.body":
    "アカウントの鍵は、すでに変更されている可能性があります。ご本人であることを確かめるため、マスターパスワードと、新しい緊急キットにある新しいシークレットキーの最後のグループを入力してください。",
  "rotation.resume.dialog.submit": "続ける",
  "rotation.resume.discard.title": "この鍵の変更を破棄しますか？",
  "rotation.resume.discard.body":
    "今回の試行で作成された新しいシークレットキーと新しい緊急キットは削除されます。アカウントは、そのままです。",
  "rotation.resume.discard.confirm": "鍵の変更を破棄",
  "rotation.resume.abandon": "残りを断念…",
  "rotation.resume.stalled": "続行が完了しませんでした。失敗が続く場合は、ここで中止できます。",
  "rotation.abandon.title": "ファイルの再暗号化を中止しますか？",
  "rotation.abandon.body":
    "新しい鍵はすでに使われています。Mosaic は、まだ待機中の添付ファイルの再暗号化を中止します。それらは古いファイルキーのままのため、古い鍵とファイルのコピーを持つ人は、開けてしまう可能性があります。",
  "rotation.abandon.stays":
    "すでに再暗号化されたものは、そのままです。待機中のファイルを新しい鍵で保護するには、後で削除するか追加し直してください。",
  "rotation.abandon.confirm": "再暗号化を中止",
  "rotation.abandon.keep": "試行を続ける",
  "rotation.marker.title": "未確認の鍵再作成の通知",
  "rotation.marker.hint":
    "別のデバイスで鍵を更新していない場合、この通知は本物ではない可能性があります。無視して構いません。",
  "rotation.marker.open": "この通知を無視…",
  "rotation.marker.open.label": "この通知を無視: {name}",
  "rotation.marker.confirm":
    "Mosaic が、確認できない鍵再作成の通知を見つけました。別のデバイスでアカウントの鍵を再作成していない場合は、無視して構いません。",
  "rotation.marker.ignore": "通知を無視",
  "rotation.marker.keep": "通知を残す",
  "rotation.ignored.title": "無視した鍵再作成の通知",
  "rotation.ignored.body":
    "無視することを選んだ通知です。別のデバイスで実際にアカウントの鍵を再作成した場合は、復元してください。",
  "rotation.ignored.row": "同期フォルダからの鍵再作成の通知",
  "rotation.ignored.restore": "復元",
  "rotation.ignored.restore.label": "通知 {name} を復元",
  "rotation.resume.done": "鍵の変更が完了しました。",
  "rotation.recovery.title": "アカウントの鍵が別のデバイスで更新されました",
  "rotation.recovery.body":
    "お使いのデバイスの1つが、このアカウントの鍵を更新しました。同期を続けるには、マスターパスワードと、新しい緊急キットのシークレットキーを入力してください。",
  "rotation.recovery.password": "マスターパスワード（そちらで変更した場合は新しいもの）",
  "rotation.recovery.secretKey": "新しい緊急キットのシークレットキー",
  "rotation.recovery.submit": "新しい鍵を使う",
  "rotation.recovery.warning":
    "ご自身で鍵を更新していませんか？その場合、古いパスワードとシークレットキーを知る誰かが更新した可能性があります。ここには何も入力せず、先に他のデバイスを確認してください。",
  "rotation.recovery.notice": "このデバイスでこの操作を行うまで、同期は一時停止されます。",
  "rotation.recovery.open": "新しい鍵を使う…",
  "rotation.recovery.keyHint":
    "鍵が別のデバイスで更新された場合は、その緊急キットにある新しいシークレットキーも入力してください。",
  "rotation.recovery.done": "このデバイスは、新しい鍵を使用しています。",
  "rotation.recovery.optional": "最新の緊急キットのシークレットキー（更新した場合のみ）",
  "rotation.file.unknown":
    "不明なアカウントファイルが同期フォルダに現れました。お使いの鍵では確認できません。残っていた古いファイルか、誰かがフォルダを変更した可能性があります。データへの影響はありません。",
  "rotation.file.dismiss": "閉じる",
  "rotation.file.conflict.title": "2台のデバイスが同時に鍵を更新しました",
  "rotation.file.conflict":
    "別のデバイスが、このデバイスと同時にアカウントの鍵を更新したため、両者が一致しなくなっています。このデバイスからもう一度鍵を再作成して、揃えてください。このデバイスの鍵が優先され、もう一方のデバイスは新しいシークレットキーを求めます。",
  "rotation.file.conflict.rekey": "このデバイスからもう一度鍵を再作成",
  "rotation.file.conflict.keep": "通知を非表示",
  "rotation.resume.body.vault":
    "保管庫の添付ファイルの再暗号化が中断されました。失われたものはありません。続けて完了してください。",
} satisfies Record<keyof typeof en, string>;
