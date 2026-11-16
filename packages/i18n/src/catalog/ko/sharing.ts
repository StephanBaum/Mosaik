/**
 * Korean UI strings: sharing. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { sharing as en } from "../en/sharing.ts";

export const sharing = {
  "sharing.title": "공유",
  "sharing.body":
    "아는 사람들과 보관함을 공유해요. 모든 내용은 암호화된 상태로 유지돼요. 초대한 사람만 공유 보관함을 읽을 수 있고, Mosaic은 보안 코드를 비교한 사람과만 공유해요.",
  "sharing.notHosted.none":
    "공유에는 Mosaic 서버가 필요해요. 먼저 동기화 설정에서 Mosaic 서버로 동기화를 켜세요.",
  "sharing.notHosted.folder":
    "동기화 폴더에서는 공유를 사용할 수 없어요. 폴더는 누가 보관함을 읽을 수 있는지 구분할 수 없어서 Mosaic은 폴더의 보관함을 나만 쓰도록 유지해요. 공유하려면 폴더 연결을 해제하고 동기화 설정에서 Mosaic 서버를 연결하세요.",
  "sharing.notHosted": "공유에는 Mosaic 서버가 필요해요. 먼저 동기화 설정에서 서버를 연결하세요.",
  "sharing.syncSettings": "동기화 설정 열기",
  "sharing.ownCode.title": "내 보안 코드",
  "sharing.ownCode.body":
    "공유할 사람에게 직접 만나거나 전화로 이 코드를 읽어 주세요. 상대방에게 보이는 코드와 같다면 중간에 아무도 끼어 있지 않은 거예요.",
  "sharing.ownCode.pending": "내 보안 코드는 서버와 처음 동기화한 후에 표시돼요.",
  "sharing.contacts.title": "조회한 사람",
  "sharing.contacts.empty": "아직 아무도 조회하지 않았어요.",
  "sharing.contacts.verified": "확인됨",
  "sharing.contacts.notVerified": "확인되지 않음",
  "sharing.contacts.changed": "다시 확인 필요",
  "sharing.contacts.verify": "코드 비교…",
  "sharing.contacts.verifyOf": "{email}과(와) 코드 비교",
  "sharing.contacts.review": "변경 사항 검토…",
  "sharing.contacts.reviewOf": "{email}의 변경 사항 검토",
  "sharing.share.open": "공유…",
  "sharing.share.title": "{vault} 공유",
  "sharing.share.dialog": "공유: {vault}",
  "sharing.share.roleStep": "{email}의 역할 선택",
  "sharing.lookup.email": "상대방의 이메일 주소",
  "sharing.lookup.find": "조회",
  "sharing.lookup.searching": "조회하는 중…",
  "sharing.lookup.notFoundHint":
    "Mosaic은 그 주소에 계정이 있는지 알려 줄 수 없어요. 위의 코드가 상대방에게 보이는 코드와 다르면 공유하지 마세요.",
  "sharing.code.title": "보안 코드 비교",
  "sharing.code.compare":
    "이 코드를 {email}과(와) 직접 만나거나 전화로 비교하세요. 이메일이나 채팅은 사용하지 마세요.",
  "sharing.code.compareNew":
    "새 코드를 {email}과(와) 직접 만나거나 전화로 비교하세요. 이메일이나 채팅은 사용하지 마세요.",
  "sharing.code.shown": "Mosaic이 {email}에 대해 표시하는 코드",
  "sharing.code.typed": "상대방이 읽어 준 코드 (선택)",
  "sharing.code.typedMatch": "위의 코드와 같아요.",
  "sharing.code.attest":
    "이 코드를 {email}과(와) 전화나 직접 만나서 비교했고 같아요.",
  "sharing.code.matches": "코드가 일치해요",
  "sharing.code.mismatch":
    "코드가 달라요. 공유하지 마세요. 나와 {email} 사이에 누군가 끼어 있을 수 있어요.",
  "sharing.code.mismatchHelp":
    "주소를 확인하고 {email}에게 코드를 다시 읽어 달라고 요청하세요. 그래도 다르면 취소하고 다른 방법으로 알리세요.",
  "sharing.code.verified": "{email}을(를) 확인했어요.",
  "sharing.code.status.new": "새 연락처",
  "sharing.code.status.pinned": "아직 확인되지 않음",
  "sharing.code.status.verified": "확인됨",
  "sharing.code.status.changed": "키가 변경됨",
  "sharing.code.status.conflicting": "기기 간 불일치",
  "sharing.code.conflicting":
    "내 기기들이 {email}에 대해 서로 다른 키를 확인했어요. 공유하기 전에 상대방과 코드를 다시 비교하세요.",
  "sharing.verify.title": "{email} 확인",
  "sharing.role.label": "역할",
  "sharing.role.owner": "소유자",
  "sharing.role.manage": "관리자",
  "sharing.role.write": "편집자",
  "sharing.role.read": "보기 전용",
  "sharing.role.desc.manage": "사람을 초대하고 제거하며 역할을 변경할 수 있어요.",
  "sharing.role.desc.write": "항목을 추가하고 변경할 수 있어요.",
  "sharing.role.desc.read": "읽기만 할 수 있어요.",
  "sharing.role.honesty":
    "항목을 읽을 수 있는 사람은 누구나 복사할 수 있어요. 보기 전용 구성원은 보관함을 변경할 수 없지만 보이는 내용을 보관하는 것을 막을 방법은 없어요. 나중에 사람을 제거해도 이미 다운로드한 내용은 되돌릴 수 없어요.",
  "sharing.invite.send": "초대 보내기",
  "sharing.invite.sent": "{email}에게 초대를 보냈어요. Mosaic을 열어 수락해야 해요.",
  "sharing.invite.sentTitle": "초대를 보냈어요",
  "sharing.invite.pending": "{email}의 응답을 기다리는 중",
  "sharing.invite.expires": "{when}에 만료",
  "sharing.invite.cancel": "초대 철회",
  "sharing.invite.cancelOf": "{email}에 대한 초대 철회",
  "sharing.invite.cancelled": "초대를 철회했어요.",
  "sharing.inbox.title": "초대",
  "sharing.inbox.nav": "초대",
  "sharing.inbox.navCount": "초대, {count}개 대기 중",
  "sharing.inbox.intro":
    "공유 보관함에 나를 초대한 사람들이에요. Mosaic은 모든 초대를 확인한 후에 보여 줘요.",
  "sharing.inbox.empty":
    "지금은 초대가 없어요. 누군가 공유 보관함에 초대하면 여기에 표시돼요.",
  "sharing.inbox.from": "보낸 사람: {email}",
  "sharing.inbox.review": "검토",
  "sharing.inbox.reviewOf": "{email}의 초대 검토",
  "sharing.inbox.refresh": "새로 고침",
  "sharing.inbox.preview.title": "{vault}에 참여할까요?",
  "sharing.inbox.preview.body": "{email}이(가) {role}(으)로 초대했어요.",
  "sharing.inbox.preview.unnamed": "공유 보관함",
  "sharing.inbox.preview.owner": "이 보관함은 {email}의 것이에요.",
  "sharing.inbox.verifySender": "참여하기 전에 {email}의 보안 코드를 비교하세요.",
  "sharing.inbox.verifyOwner":
    "{sender}은(는) 소유자가 아니에요. 참여하기 전에 소유자의 보안 코드도 비교하세요. 보관함의 키를 바꿀 수 있는 사람은 소유자예요.",
  "sharing.inbox.ownerCode": "소유자 {email}의 코드",
  "sharing.inbox.accept": "보관함 참여",
  "sharing.inbox.readyTitle": "모두 확인했어요. 이 보관함에 참여할까요?",
  "sharing.inbox.decline": "거절",
  "sharing.inbox.block": "거절하고 보낸 사람 차단",
  "sharing.inbox.block.title": "거절하고 이 보낸 사람을 차단할까요?",
  "sharing.inbox.block.body":
    "Mosaic이 초대를 거절하고 지금부터 {email}의 모든 초대를 무시해요. Mosaic에서는 되돌릴 수 없어요.",
  "sharing.inbox.unreadable":
    "이 초대를 열 수 없어요. 손상되었거나 내게 보낸 것이 아닐 수 있어요. {email}에게 다시 보내 달라고 요청하세요.",
  "sharing.inbox.joined": "보관함에 참여했어요. 잠시 후 보관함 목록에 표시돼요.",
  "sharing.inbox.declined": "초대를 거절했어요.",
  "sharing.inbox.outgoing": "내가 보낸 초대",
  "sharing.inbox.outgoing.empty": "응답을 기다리는 초대가 없어요.",
  "sharing.inbox.unknownVault": "보관함",
  "sharing.members.title": "접근 권한이 있는 사람",
  "sharing.members.you": "나",
  "sharing.members.unverified": "확인되지 않음",
  "sharing.members.unconfirmed": "구성원 목록에 없음",
  "sharing.members.unconfirmed.help":
    "서버에는 이 사람이 있지만 보관함 자체의 구성원 목록에는 없어요. 다음 키 변경 때 이 사람은 접근할 수 없게 돼요. 모르는 사람이면 제거하세요.",
  "sharing.members.dropped": "서버에 더 이상 없음",
  "sharing.members.dropped.help":
    "이 사람은 보관함의 구성원 목록에 있지만 서버에는 더 이상 없어요. 나갔을 수 있어요. 예상한 일인지 확인하세요.",
  "sharing.members.invited": "초대됨",
  "sharing.members.removing": "제거 중",
  "sharing.members.verified": "확인됨",
  "sharing.members.verify": "확인…",
  "sharing.members.verifyOf": "{email} 확인",
  "sharing.members.remove": "제거",
  "sharing.members.removeOf": "{email} 제거",
  "sharing.members.remove.title": "{email}을(를) 제거할까요?",
  "sharing.members.remove.confirm":
    "{email}은(는) 이미 다운로드한 내용을 그대로 가지고 있어요. Mosaic이 지금 보관함 키를 바꾸므로 새 내용은 읽을 수 없어요.",
  "sharing.members.remove.confirmManager":
    "{email}은(는) 이미 다운로드한 내용을 그대로 가지고 있어요. 서버가 지금부터 업데이트를 보내지 않아요. 보관함 소유자의 기기가 다음에 동기화할 때 보관함 키를 바꾸며, 그때까지는 서버만 {email}의 접근을 막아요.",
  "sharing.members.cancel": "취소",
  "sharing.members.removed": "제거했어요.",
  "sharing.members.leave": "보관함 나가기",
  "sharing.members.leave.title": "이 보관함에서 나갈까요?",
  "sharing.members.leave.confirm":
    "이 기기의 사본은 계속 읽을 수 있지만 더 이상 업데이트되지 않아요. 다시 참여하려면 새 초대가 필요해요.",
  "sharing.members.left": "보관함에서 나갔어요.",
  "sharing.members.roleOf": "{email}의 역할",
  "sharing.members.changeRole": "역할 변경",
  "sharing.members.changeRoleOf": "{email}의 역할 변경",
  "sharing.members.invite": "사람 초대…",
  "sharing.members.empty": "지금까지는 나만 이 보관함을 열 수 있어요.",
  "sharing.members.unknownEmail": "알 수 없는 주소",
  "sharing.members.managerNote":
    "사람을 초대하고 제거할 수 있어요. 보관함 키는 소유자의 기기만 바꿀 수 있으므로, 제거는 소유자의 기기가 동기화한 후에 완료돼요.",
  "sharing.members.readerNote": "이 보관함을 읽을 수는 있지만 접근 권한이 있는 사람은 바꿀 수 없어요.",
  "sharing.rotation.running":
    "제거가 소유자의 기기가 보관함 키를 바꾸기를 기다리고 있어요. 동기화될 때까지는 서버만 제거된 사람의 접근을 막아요.",
  "sharing.rotation.unprotected":
    "{count, plural, other {파일 #개는}} 이 기기에 없어서 이전 보호가 유지돼요. 제거한 사람이 이전 키를 보관했다면 그 파일을 여전히 열 수 있어요. 해당 파일이 있는 기기에서 다시 추가하세요.",
  "sharing.keyChanged.title": "{email}의 보안 코드가 바뀌었어요",
  "sharing.keyChanged.body":
    "서버가 이제 {email}에 대해 다른 키를 보여 줘요. 상대방이 Mosaic을 다시 설정했을 때도 이렇게 되지만, 서버에 대한 공격도 이렇게 보여요. 직접 만나거나 전화로 코드를 비교하기 전에는 새 키로 아무것도 공유하지 않아요.",
  "sharing.keyChanged.old": "이전에 비교한 코드",
  "sharing.keyChanged.new": "새 코드",
  "sharing.keyChanged.accept": "새 코드가 일치해요",
  "sharing.finding.region": "공유 경고",
  "sharing.finding.summary": "{count, plural, other {공유 경고 #개}}",
  "sharing.finding.review": "검토",
  "sharing.finding.collapse": "접기",
  "sharing.finding.dismiss": "닫기",
  "sharing.finding.unknownEmail": "알 수 없는 사람",
  "sharing.finding.verify": "확인…",
  "sharing.finding.remove": "제거",
  "sharing.finding.removeOf": "{vault}에서 {email} 제거",
  "sharing.finding.identityMismatch":
    "서버가 내 계정에 대해 이 기기가 만든 것과 다른 공유 키를 보여 줘서 공유가 일시 중지되었어요. 아무것도 공유하지 말고 서버와 다른 기기를 확인하세요. 그런 다음 “확인했어요…”를 선택하세요.",
  "sharing.finding.rosterInvalid":
    "{vault}의 구성원 목록이 손상되었거나 누군가 권한 없이 변경했어요. 누가 접근할 수 있는지 확인하기 전에는 사람을 추가하지 마세요. 그런 다음 “확인했어요…”를 선택하세요.",
  "sharing.finding.shareUntrusted":
    "{vault}의 새 키가 거부되었어요. {sender}이(가) 보냈지만 이 보관함의 키를 바꿀 권한이 없어요. 보관함은 이전 키를 유지해요. 소유자에게 변경했는지 물어보세요. 그런 다음 “확인했어요…”를 선택하세요.",
  "sharing.finding.memberUnverified":
    "{vault}의 {email}을(를) 확인하지 않았어요. Mosaic은 이 사람에게 새 키를 보내지 않으며 다음 키 변경 때 접근할 수 없게 돼요. 보안 코드를 비교하거나 제거하세요.",
  "sharing.finding.memberDropped":
    "{email}은(는) {vault}의 구성원 목록에 있지만 서버에는 더 이상 없어요. 나갔거나 서버가 제거했을 수 있어요. 예상한 일인지 확인하고, 아니라면 제거하세요.",
  "sharing.finding.memberKeyMismatch":
    "Mosaic이 받은 {email}의 키가 확인했던 키와 달라서 {vault}에서 제거되었어요. 다시 초대하기 전에 상대방과 보안 코드를 다시 비교하세요.",
  "sharing.finding.vaultHeld":
    "{vault}은(는) 다른 기기들이 소유자를 확인할 때까지 대기 중이에요. 이 보관함을 이미 사용하는 기기에서 Mosaic을 여세요.",
  "sharing.finding.accessLost":
    "{vault}에 대한 접근 권한이 없어졌어요. 이 기기의 사본은 계속 읽을 수 있지만 업데이트되지 않아요. 소유자에게 다시 초대해 달라고 요청하세요.",
  "sharing.finding.rotationBlocked":
    "사람을 제거한 후 {vault}의 키를 바꾸지 못해서 그 사람이 아직 이전 키를 가지고 있어요. 다시 동기화하세요. 계속되면 구성원 목록을 열어 누가 접근할 수 있는지 확인하세요.",
  "emergency.title": "비상 접근",
  "emergency.body":
    "내게 무슨 일이 생겼을 때 보관함 접근을 요청할 수 있는 사람을 지정해요. 이것은 계정 복구가 아니에요. 지정한 사람은 선택한 보관함에 대한 읽기 전용 접근만 얻으며, 대기 시간이 지난 후에만 가능하고, 그 시간 동안 내가 거부할 수 있어요. 비상 키트를 대체하지 않아요.",
  "emergency.folderUnsupported":
    "비상 접근에는 Mosaic 서버가 필요해요. 동기화 폴더는 대기 시간을 강제할 수 없어요.",
  "emergency.granted.title": "접근을 요청할 수 있는 사람",
  "emergency.granted.empty": "내 보관함에 접근을 요청할 수 있는 사람이 없어요.",
  "emergency.granted.add": "사람 추가…",
  "emergency.granted.scopeAll": "내 모든 보관함",
  "emergency.granted.scopeSome": "{count, plural, other {보관함 #개}}",
  "emergency.granted.wait": "{days, plural, other {#일}} 대기",
  "emergency.granted.waitingUntil": "접근을 요청했어요. 거부하지 않으면 {when}에 열려요.",
  "emergency.granted.approved": "요청을 승인했어요.",
  "emergency.granted.released": "접근이 허용되었어요. 보관함을 읽을 수 있어요.",
  "emergency.granted.idle": "요청 없음",
  "emergency.granted.remove": "접근 권한 제거",
  "emergency.granted.removeOf": "{email}의 접근 권한 제거",
  "emergency.granted.remove.title": "{email}의 비상 접근 권한을 제거할까요?",
  "emergency.granted.remove.body":
    "더 이상 요청할 수 없어요. 이미 접근이 허용되었다면 Mosaic이 그 사람이 연 보관함의 키를 바꿔요. 이미 다운로드한 내용은 그대로 가지고 있어요.",
  "emergency.granted.removed": "비상 접근 권한을 제거했어요.",
  "emergency.grant.title": "비상 접근 허용",
  "emergency.grant.lookup": "누가 요청할 수 있게 할까요?",
  "emergency.grant.optionsTitle": "{email}이(가) 열 수 있는 것",
  "emergency.grant.scope": "열 수 있는 것",
  "emergency.grant.scope.all": "새 보관함을 포함한 내 모든 보관함",
  "emergency.grant.scope.selected": "내가 선택한 보관함만",
  "emergency.grant.scope.pick": "보관함을 하나 이상 선택하세요.",
  "emergency.grant.wait": "대기 시간",
  "emergency.grant.wait.option": "{days, plural, other {#일}}",
  "emergency.grant.waitHint":
    "요청이 있은 후 이 시간 안에 Mosaic을 한 번 이상 열어서 요청을 확인하고 거부할 수 있게 하세요. 시간이 지나면 그 사람은 내게 더 묻지 않고 읽기 전용 접근을 얻어요.",
  "emergency.grant.serverTrust":
    "대기 시간은 서버가 지켜요. 지정한 사람과 서버 운영자가 공모하면 대기 시간을 건너뛸 수 있어요. 서버만으로는 내 보관함을 읽을 수 없어요.",
  "emergency.grant.vaultKeysOnly":
    "내 보관함의 키만 해당돼요. 계정, 마스터 비밀번호, 비밀 키에 대한 접근 권한은 주지 않아요.",
  "emergency.grant.submit": "비상 접근 허용",
  "emergency.grant.done": "{email}이(가) 이제 비상 접근을 요청할 수 있어요.",
  "emergency.grant.needVerify": "먼저 {email}의 보안 코드를 비교하세요.",
  "emergency.received.title": "나를 지정한 사람",
  "emergency.received.empty": "나를 비상 접근 대상으로 지정한 사람이 없어요.",
  "emergency.received.request": "접근 요청",
  "emergency.received.requestOf": "{email}에게 접근 요청",
  "emergency.received.request.title": "{email}에게 접근을 요청할까요?",
  "emergency.received.request.body":
    "{email}은(는) 다음에 Mosaic을 열 때 내 요청을 봐요. {days, plural, other {#일}} 후에는 상대방이 거부하지 않는 한 상대방의 보관함을 읽기 전용으로 열 수 있어요.",
  "emergency.received.requested": "요청을 보냈어요. {email}이(가) 다음에 Mosaic을 열 때 확인해요.",
  "emergency.received.requestHint":
    "{email}은(는) Mosaic에서 내 요청을 봐요. {days, plural, other {#일}} 후에는 상대방이 거부하지 않는 한 상대방의 보관함을 읽기 전용으로 열 수 있어요.",
  "emergency.received.countdown": "{when}에 상대방의 보관함을 열 수 있어요.",
  "emergency.received.waitingLive": "요청이 대기 중이에요.",
  "emergency.received.claim": "상대방의 보관함 열기",
  "emergency.received.claimOf": "상대방의 보관함 열기: {email}",
  "emergency.received.claimed": "{count, plural, other {보관함 #개를}} 읽기 전용으로 추가했어요.",
  "emergency.received.cooldown": "{email}이(가) 거부했어요. {when}에 다시 요청할 수 있어요.",
  "emergency.received.verifyFirst":
    "상대방의 보관함을 열기 전에 {email}의 보안 코드를 비교하세요. Mosaic은 확인한 키만 사용해요.",
  "emergency.received.verify": "코드 비교…",
  "emergency.received.verifyOf": "{email}과(와) 코드 비교",
  "emergency.received.approved": "{email}이(가) 내 요청을 승인했어요.",
  "emergency.received.waitDays": "{days, plural, other {#일}} 대기",
  "emergency.banner.region": "비상 접근 요청",
  "emergency.banner.body":
    "{email}이(가) 내 보관함에 대한 비상 접근을 요청했어요. 거부하지 않으면 {when}에 열려요.",
  "emergency.banner.refuse": "거부",
  "emergency.banner.approve": "지금 승인…",
  "emergency.banner.approve.title": "지금 비상 접근을 승인할까요?",
  "emergency.banner.approve.body":
    "{email}이(가) 즉시 내 보관함에 대한 읽기 전용 접근을 얻어요. 확인하려면 마스터 비밀번호를 입력하세요.",
  "emergency.banner.approve.confirm": "승인",
  "emergency.banner.refused": "요청을 거부했어요.",
  "emergency.banner.approved": "접근을 승인했어요.",
} satisfies Record<keyof typeof en, string>;
