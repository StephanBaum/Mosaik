/**
 * Japanese UI strings: sharing. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { sharing as en } from "../en/sharing.ts";

export const sharing = {
  "sharing.title": "共有",
  "sharing.body":
    "知っている人と保管庫を共有します。すべての内容は暗号化されたままです。共有保管庫を読み取れるのは招待した人だけで、Mosaic は、安全コードを照合した人とのみ共有します。",
  "sharing.notHosted.none":
    "共有には Mosaic サーバーが必要です。先に「同期」設定で、Mosaic サーバーとの同期をオンにしてください。",
  "sharing.notHosted.folder":
    "同期フォルダでは共有できません。フォルダでは、誰が保管庫を読み取れるかを判断できないため、Mosaic はフォルダ内の保管庫を自分専用に保ちます。共有するには、フォルダの接続を解除し、「同期」設定で Mosaic サーバーを接続してください。",
  "sharing.notHosted": "共有には Mosaic サーバーが必要です。先に「同期」設定で接続してください。",
  "sharing.syncSettings": "同期設定を開く",
  "sharing.ownCode.title": "あなたの安全コード",
  "sharing.ownCode.body":
    "共有する相手に、直接会うか電話で、このコードを読み上げてください。相手に表示されるあなたのコードと一致すれば、間に誰もいません。",
  "sharing.ownCode.pending": "あなたの安全コードは、サーバーとの最初の同期後に表示されます。",
  "sharing.contacts.title": "検索した人",
  "sharing.contacts.empty": "まだ誰も検索していません。",
  "sharing.contacts.verified": "確認済み",
  "sharing.contacts.notVerified": "未確認",
  "sharing.contacts.changed": "再確認が必要です",
  "sharing.contacts.verify": "コードを照合…",
  "sharing.contacts.verifyOf": "{email} とコードを照合",
  "sharing.contacts.review": "変更を確認…",
  "sharing.contacts.reviewOf": "{email} の変更を確認",
  "sharing.share.open": "共有…",
  "sharing.share.title": "{vault} を共有",
  "sharing.share.dialog": "共有: {vault}",
  "sharing.share.roleStep": "{email} の役割を選択",
  "sharing.lookup.email": "相手のメールアドレス",
  "sharing.lookup.find": "検索",
  "sharing.lookup.searching": "検索しています…",
  "sharing.lookup.notFoundHint":
    "Mosaic は、そのアドレスにアカウントがあるかどうかをお伝えできません。上のコードが相手に表示されるものと一致しない場合は、共有しないでください。",
  "sharing.code.title": "安全コードを照合",
  "sharing.code.compare":
    "このコードを、{email} と直接会うか電話で照合してください。メールやチャットは使わないでください。",
  "sharing.code.compareNew":
    "新しいコードを、{email} と直接会うか電話で照合してください。メールやチャットは使わないでください。",
  "sharing.code.shown": "{email} について Mosaic が表示するコード",
  "sharing.code.typed": "相手が読み上げたコード（任意）",
  "sharing.code.typedMatch": "これは上のコードです。",
  "sharing.code.attest":
    "このコードを {email} と電話または直接会って照合し、同じであることを確認しました。",
  "sharing.code.matches": "コードが一致",
  "sharing.code.mismatch":
    "コードが異なります。共有しないでください。あなたと {email} の間に誰かがいる可能性があります。",
  "sharing.code.mismatchHelp":
    "アドレスを確認し、{email} にもう一度コードを読み上げてもらってください。それでも異なる場合は、キャンセルして、別の方法で伝えてください。",
  "sharing.code.verified": "{email} を確認しました。",
  "sharing.code.status.new": "新しい連絡先",
  "sharing.code.status.pinned": "まだ確認されていません",
  "sharing.code.status.verified": "確認済み",
  "sharing.code.status.changed": "鍵が変更されました",
  "sharing.code.status.conflicting": "デバイス間で不一致",
  "sharing.code.conflicting":
    "お使いのデバイスが、{email} について異なる鍵を確認しました。共有する前に、もう一度相手とコードを照合してください。",
  "sharing.verify.title": "{email} を確認",
  "sharing.role.label": "役割",
  "sharing.role.owner": "オーナー",
  "sharing.role.manage": "管理者",
  "sharing.role.write": "編集者",
  "sharing.role.read": "閲覧者",
  "sharing.role.desc.manage": "人の招待と削除、役割の変更ができます。",
  "sharing.role.desc.write": "項目の追加と変更ができます。",
  "sharing.role.desc.read": "読み取りのみできます。",
  "sharing.role.honesty":
    "項目を読み取れる人は、それをコピーできます。閲覧者は保管庫を変更できませんが、見たものを手元に残すことは防げません。また、後から誰かを削除しても、すでにダウンロードされたものを取り戻すことはできません。",
  "sharing.invite.send": "招待を送信",
  "sharing.invite.sent": "{email} に招待を送信しました。相手が Mosaic を開いて承諾する必要があります。",
  "sharing.invite.sentTitle": "招待を送信しました",
  "sharing.invite.pending": "{email} の返事を待っています",
  "sharing.invite.expires": "有効期限: {when}",
  "sharing.invite.cancel": "招待を取り消す",
  "sharing.invite.cancelOf": "{email} への招待を取り消す",
  "sharing.invite.cancelled": "招待を取り消しました。",
  "sharing.inbox.title": "招待",
  "sharing.inbox.nav": "招待",
  "sharing.inbox.navCount": "招待、{count}件待機中",
  "sharing.inbox.intro":
    "共有保管庫に招待してくれた人の一覧です。Mosaic は、すべての招待を確認してから表示します。",
  "sharing.inbox.empty":
    "現在、招待はありません。誰かが共有保管庫に招待すると、ここに表示されます。",
  "sharing.inbox.from": "送信者: {email}",
  "sharing.inbox.review": "確認",
  "sharing.inbox.reviewOf": "{email} からの招待を確認",
  "sharing.inbox.refresh": "更新",
  "sharing.inbox.preview.title": "{vault} に参加しますか？",
  "sharing.inbox.preview.body": "{email} が、{role} として招待しています。",
  "sharing.inbox.preview.unnamed": "共有保管庫",
  "sharing.inbox.preview.owner": "この保管庫は {email} のものです。",
  "sharing.inbox.verifySender": "参加する前に、{email} の安全コードを照合してください。",
  "sharing.inbox.verifyOwner":
    "{sender} はオーナーではありません。参加する前に、オーナーの安全コードも照合してください。保管庫の鍵を変更できるのは、オーナーのコードだからです。",
  "sharing.inbox.ownerCode": "オーナー {email} のコード",
  "sharing.inbox.accept": "保管庫に参加",
  "sharing.inbox.readyTitle": "すべて確認しました。この保管庫に参加しますか？",
  "sharing.inbox.decline": "辞退",
  "sharing.inbox.block": "辞退して送信者をブロック",
  "sharing.inbox.block.title": "辞退して、この送信者をブロックしますか？",
  "sharing.inbox.block.body":
    "Mosaic は招待を辞退し、今後 {email} からのすべての招待を無視します。Mosaic では、この操作を元に戻せません。",
  "sharing.inbox.unreadable":
    "この招待を開けません。破損しているか、あなた宛てではない可能性があります。{email} に、もう一度送信するよう依頼してください。",
  "sharing.inbox.joined": "保管庫に参加しました。まもなく、保管庫の一覧に表示されます。",
  "sharing.inbox.declined": "招待を辞退しました。",
  "sharing.inbox.outgoing": "送信した招待",
  "sharing.inbox.outgoing.empty": "返事を待っている招待はありません。",
  "sharing.inbox.unknownVault": "保管庫",
  "sharing.members.title": "アクセスできる人",
  "sharing.members.you": "あなた",
  "sharing.members.unverified": "未確認",
  "sharing.members.unconfirmed": "メンバーリストにありません",
  "sharing.members.unconfirmed.help":
    "サーバーにはこの人が載っていますが、保管庫自身のメンバーリストにはありません。次の鍵の変更で、この人は締め出されます。知らない人であれば、削除してください。",
  "sharing.members.dropped": "サーバーにもうありません",
  "sharing.members.dropped.help":
    "この人は保管庫のメンバーリストにありますが、サーバーにはもう載っていません。退出した可能性があります。想定どおりか確認してください。",
  "sharing.members.invited": "招待済み",
  "sharing.members.removing": "削除中",
  "sharing.members.verified": "確認済み",
  "sharing.members.verify": "確認…",
  "sharing.members.verifyOf": "{email} を確認",
  "sharing.members.remove": "削除",
  "sharing.members.removeOf": "{email} を削除",
  "sharing.members.remove.title": "{email} を削除しますか？",
  "sharing.members.remove.confirm":
    "{email} は、すでにダウンロードしたものを保持します。Mosaic は今すぐ保管庫の鍵を変更し、相手は新しい内容を一切読み取れなくなります。",
  "sharing.members.remove.confirmManager":
    "{email} は、すでにダウンロードしたものを保持します。サーバーは今すぐ、相手への更新の送信を止めます。保管庫の鍵は、保管庫のオーナーのデバイスが次に同期したときに変更されます。それまでは、サーバーだけが {email} を締め出します。",
  "sharing.members.cancel": "キャンセル",
  "sharing.members.removed": "削除しました。",
  "sharing.members.leave": "保管庫から退出",
  "sharing.members.leave.title": "この保管庫から退出しますか？",
  "sharing.members.leave.confirm":
    "このデバイスにあるコピーは読み取れる状態のままですが、更新されなくなります。もう一度参加するには、新しい招待が必要です。",
  "sharing.members.left": "保管庫から退出しました。",
  "sharing.members.roleOf": "{email} の役割",
  "sharing.members.changeRole": "役割を変更",
  "sharing.members.changeRoleOf": "{email} の役割を変更",
  "sharing.members.invite": "誰かを招待…",
  "sharing.members.empty": "今のところ、この保管庫を開けるのはあなただけです。",
  "sharing.members.unknownEmail": "不明なアドレス",
  "sharing.members.managerNote":
    "人の招待と削除ができます。保管庫の鍵を変更するのはオーナーのデバイスだけのため、削除が完了するのは、オーナーのデバイスが同期した後です。",
  "sharing.members.readerNote": "この保管庫を読み取れますが、アクセスできる人を変更することはできません。",
  "sharing.rotation.running":
    "削除は、オーナーのデバイスによる保管庫の鍵の変更を待っています。同期されるまでは、サーバーだけが削除した人を締め出します。",
  "sharing.rotation.unprotected":
    "{count, plural, other {#個のファイル}}は、このデバイスにないため、古い保護のままです。削除した人が古い鍵を保持している場合は、まだ開けてしまう可能性があります。ファイルがあるデバイスから、もう一度追加してください。",
  "sharing.keyChanged.title": "{email} の安全コードが変更されました",
  "sharing.keyChanged.body":
    "サーバーが、{email} について別の鍵を表示しています。相手が Mosaic を設定し直したときに起こりますが、サーバーへの攻撃もこのように見えます。直接会うか電話でコードを照合するまで、新しい鍵では何も共有されません。",
  "sharing.keyChanged.old": "以前に照合したコード",
  "sharing.keyChanged.new": "新しいコード",
  "sharing.keyChanged.accept": "新しいコードが一致",
  "sharing.finding.region": "共有の警告",
  "sharing.finding.summary": "{count, plural, other {共有の警告が#件あります}}",
  "sharing.finding.review": "確認",
  "sharing.finding.collapse": "折りたたむ",
  "sharing.finding.dismiss": "閉じる",
  "sharing.finding.unknownEmail": "不明な人",
  "sharing.finding.verify": "確認…",
  "sharing.finding.remove": "削除",
  "sharing.finding.removeOf": "{vault} から {email} を削除",
  "sharing.finding.identityMismatch":
    "サーバーが、このデバイスが作成したものとは異なる、あなたのアカウントの共有キーを表示しているため、共有は一時停止されています。何も共有せず、サーバーと他のデバイスを確認してください。その後、「確認しました…」を選んでください。",
  "sharing.finding.rosterInvalid":
    "{vault} のメンバーリストが破損しているか、誰かが許可なく変更しました。誰がアクセスできるか確認するまで、人を追加しないでください。その後、「確認しました…」を選んでください。",
  "sharing.finding.shareUntrusted":
    "{vault} の新しい鍵は拒否されました。{sender} が送信しましたが、この保管庫の鍵を変更する権限がありません。保管庫は古い鍵のままです。オーナーに、変更したか確認してください。その後、「確認しました…」を選んでください。",
  "sharing.finding.memberUnverified":
    "{vault} の {email} を確認していません。Mosaic は新しい鍵を送らず、次の鍵の変更で、この人は締め出されます。安全コードを照合するか、削除してください。",
  "sharing.finding.memberDropped":
    "{email} は {vault} のメンバーリストにありますが、サーバーにはもう載っていません。退出したか、サーバーが削除した可能性があります。想定どおりか確認し、そうでなければ削除してください。",
  "sharing.finding.memberKeyMismatch":
    "Mosaic が受け取った {email} の鍵が、確認済みのものと一致しなかったため、{vault} から削除されました。もう一度招待する前に、安全コードを照合し直してください。",
  "sharing.finding.vaultHeld":
    "{vault} は、他のデバイスが所有者を確認するまで保留されています。この保管庫をすでに使用しているデバイスで Mosaic を開いてください。",
  "sharing.finding.accessLost":
    "{vault} へのアクセス権がなくなりました。このデバイスのコピーは読み取れる状態のままですが、更新されません。オーナーに、もう一度招待するよう依頼してください。",
  "sharing.finding.rotationBlocked":
    "誰かを削除した後に、{vault} の鍵を変更できなかったため、その人はまだ古い鍵を保持しています。もう一度同期してください。それでも解決しない場合は、メンバーリストを開いて、誰がアクセスできるか確認してください。",
  "emergency.title": "緊急アクセス",
  "emergency.body":
    "あなたに万一のことがあったときに、保管庫へのアクセスを申請できる人を指定します。これはアカウントの復旧ではありません。相手が得られるのは、選んだ保管庫への読み取り専用のアクセスのみで、待機期間が過ぎた後に限られ、その間はあなたが拒否できます。緊急キットの代わりにはなりません。",
  "emergency.folderUnsupported":
    "緊急アクセスには Mosaic サーバーが必要です。同期フォルダでは、待機期間を強制できません。",
  "emergency.granted.title": "アクセスを申請できる人",
  "emergency.granted.empty": "保管庫へのアクセスを申請できる人はいません。",
  "emergency.granted.add": "人を追加…",
  "emergency.granted.scopeAll": "すべての保管庫",
  "emergency.granted.scopeSome": "{count, plural, other {#個の保管庫}}",
  "emergency.granted.wait": "待機期間: {days, plural, other {#日}}",
  "emergency.granted.waitingUntil": "アクセスが申請されました。拒否しない限り、{when} に開きます。",
  "emergency.granted.approved": "リクエストを承認しました。",
  "emergency.granted.released": "アクセスが解放されました。相手は保管庫を読み取れます。",
  "emergency.granted.idle": "リクエストなし",
  "emergency.granted.remove": "アクセス権を削除",
  "emergency.granted.removeOf": "{email} のアクセス権を削除",
  "emergency.granted.remove.title": "{email} の緊急アクセスを削除しますか？",
  "emergency.granted.remove.body":
    "相手は、もう申請できません。すでにアクセスが解放されていた場合、Mosaic は、相手が開いた保管庫の鍵を変更します。相手は、すでにダウンロードしたものを保持します。",
  "emergency.granted.removed": "緊急アクセスを削除しました。",
  "emergency.grant.title": "緊急アクセスを許可",
  "emergency.grant.lookup": "誰に申請を許可しますか？",
  "emergency.grant.optionsTitle": "{email} が開けるもの",
  "emergency.grant.scope": "開けるもの",
  "emergency.grant.scope.all": "新しいものを含む、すべての保管庫",
  "emergency.grant.scope.selected": "選んだ保管庫のみ",
  "emergency.grant.scope.pick": "保管庫を1つ以上選んでください。",
  "emergency.grant.wait": "待機期間",
  "emergency.grant.wait.option": "{days, plural, other {#日}}",
  "emergency.grant.waitHint":
    "リクエストが届いたら、この期間内に少なくとも1回 Mosaic を開いて、確認して拒否できるようにしてください。期間が過ぎると、あなたに確認することなく、相手は読み取り専用のアクセスを得ます。",
  "emergency.grant.serverTrust":
    "待機期間はサーバーが保持します。指定した相手とサーバーの運用者が結託すれば、待機期間を飛ばせる可能性があります。サーバーだけで保管庫を読み取ることはできません。",
  "emergency.grant.vaultKeysOnly":
    "対象は、保管庫の鍵のみです。アカウント、マスターパスワード、シークレットキーへのアクセスは含まれません。",
  "emergency.grant.submit": "緊急アクセスを許可",
  "emergency.grant.done": "{email} が緊急アクセスを申請できるようになりました。",
  "emergency.grant.needVerify": "先に {email} の安全コードを照合してください。",
  "emergency.received.title": "あなたを指定した人",
  "emergency.received.empty": "あなたを緊急アクセスに指定した人はいません。",
  "emergency.received.request": "アクセスを申請",
  "emergency.received.requestOf": "{email} にアクセスを申請",
  "emergency.received.request.title": "{email} にアクセスを申請しますか？",
  "emergency.received.request.body":
    "{email} は、次に Mosaic を開いたときに、あなたのリクエストを確認します。相手が拒否しない限り、{days, plural, other {#日}}後に、相手の保管庫を読み取り専用で開けます。",
  "emergency.received.requested": "リクエストを送信しました。{email} は、次に Mosaic を開いたときに確認します。",
  "emergency.received.requestHint":
    "{email} は、Mosaic でリクエストを確認します。相手が拒否しない限り、{days, plural, other {#日}}後に、相手の保管庫を読み取り専用で開けます。",
  "emergency.received.countdown": "{when} に、相手の保管庫を開けます。",
  "emergency.received.waitingLive": "リクエストは待機中です。",
  "emergency.received.claim": "相手の保管庫を開く",
  "emergency.received.claimOf": "相手の保管庫を開く: {email}",
  "emergency.received.claimed": "{count, plural, other {#個の保管庫}}を、読み取り専用で追加しました。",
  "emergency.received.cooldown": "{email} が拒否しました。{when} に、もう一度申請できます。",
  "emergency.received.verifyFirst":
    "相手の保管庫を開く前に、{email} の安全コードを照合してください。Mosaic は、確認済みの鍵のみを使用します。",
  "emergency.received.verify": "コードを照合…",
  "emergency.received.verifyOf": "{email} とコードを照合",
  "emergency.received.approved": "{email} があなたのリクエストを承認しました。",
  "emergency.received.waitDays": "待機期間: {days, plural, other {#日}}",
  "emergency.banner.region": "緊急アクセスのリクエスト",
  "emergency.banner.body":
    "{email} が、あなたの保管庫への緊急アクセスを申請しました。拒否しない限り、{when} に開きます。",
  "emergency.banner.refuse": "拒否",
  "emergency.banner.approve": "今すぐ承認…",
  "emergency.banner.approve.title": "緊急アクセスを今すぐ承認しますか？",
  "emergency.banner.approve.body":
    "{email} は、すぐにあなたの保管庫への読み取り専用のアクセスを得ます。確認のため、マスターパスワードを入力してください。",
  "emergency.banner.approve.confirm": "承認",
  "emergency.banner.refused": "リクエストを拒否しました。",
  "emergency.banner.approved": "アクセスを承認しました。",
} satisfies Record<keyof typeof en, string>;
