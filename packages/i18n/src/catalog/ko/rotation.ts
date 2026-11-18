/**
 * Korean UI strings: rotation. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { rotation as en } from "../en/rotation.ts";

export const rotation = {
  "rotation.section.title": "비밀번호와 키",
  "rotation.section.body":
    "마스터 비밀번호를 변경하거나, 새 비밀 키를 받거나, 계정 전체를 새 키로 옮겨요. 어떤 것을 건드리기 전에 각 변경에서 어떤 일이 일어나는지 먼저 보여 줘요.",
  "rotation.action.password": "마스터 비밀번호 변경",
  "rotation.action.password.body":
    "새 마스터 비밀번호를 선택하세요. 비밀 키와 보관함은 그대로 유지돼요.",
  "rotation.action.password.button": "변경…",
  "rotation.action.secretKey": "비밀 키 교체",
  "rotation.action.secretKey.body":
    "새 비밀 키와 새 비상 키트를 받아요. 키트를 잃어버렸거나 다른 사람이 봤을 수 있을 때 사용하세요.",
  "rotation.action.secretKey.button": "교체…",
  "rotation.action.profile": "보호 수준 업그레이드",
  "rotation.action.profile.body":
    "시도할 때마다 더 많은 메모리를 사용해서 마스터 비밀번호를 추측하기 더 어렵게 해요. 잠금을 해제하는 데 시간이 조금 더 걸려요.",
  "rotation.action.profile.button": "업그레이드…",
  "rotation.action.rekey": "계정 키 재설정",
  "rotation.action.rekey.body":
    "계정의 모든 키를 갱신해요. 새 비밀 키, 새 보관함 키, 새 보안 코드가 만들어져요. 누군가 비밀번호와 비밀 키와 함께 보관함 데이터를 복사했을 수 있을 때 사용하세요.",
  "rotation.action.rekey.button": "키 재설정…",
  "rotation.action.rekey.needsSync":
    "키 재설정은 동기화 서버나 폴더의 사본도 갱신하므로 동기화가 필요해요. 먼저 동기화를 설정하세요.",
  "rotation.title.password": "마스터 비밀번호 변경",
  "rotation.title.secretKey": "비밀 키 교체",
  "rotation.title.profile": "보호 수준 업그레이드",
  "rotation.title.rekey": "계정 키 재설정",
  "rotation.step": "{total}단계 중 {current}단계",
  "rotation.back": "뒤로",
  "rotation.stop.title": "이 변경을 중단할까요?",
  "rotation.stop.body": "방금 저장한 비상 키트는 사용할 수 없게 돼요. 이전 키트를 보관하세요.",
  "rotation.stop.confirm": "변경 중단",
  "rotation.stop.keep": "계속 진행",
  "rotation.next": "계속",
  "rotation.close": "닫기",
  "rotation.review.title.password": "비밀번호를 변경하기 전에",
  "rotation.review.title.secretKey": "비밀 키를 교체하기 전에",
  "rotation.review.title.profile": "보호 수준을 업그레이드하기 전에",
  "rotation.review.title.rekey": "무언가 유출되었나요?",
  "rotation.review.changes": "바뀌는 것",
  "rotation.review.stays": "그대로인 것",
  "rotation.review.password.changes":
    "마스터 비밀번호가 바뀌어요. 이 기기의 빠른 잠금 해제가 꺼지며 나중에 다시 설정할 수 있어요.",
  "rotation.review.password.stays":
    "비밀 키, 보관함과 항목, 보안 코드, 비상 연락처는 그대로예요.",
  "rotation.review.password.devices":
    "다른 기기는 동기화에서 로그아웃돼요. 각 기기에서 새 마스터 비밀번호를 한 번 입력하면 동기화가 계속돼요.",
  "rotation.review.secretKey.changes":
    "비밀 키와 비상 키트가 바뀌어요. 이 기기의 빠른 잠금 해제가 꺼지며 나중에 다시 설정할 수 있어요.",
  "rotation.review.secretKey.stays":
    "마스터 비밀번호, 보관함과 항목, 보안 코드, 비상 연락처는 그대로예요. 등록한 보안 키는 계속 작동해요.",
  "rotation.review.secretKey.devices":
    "다른 기기는 동기화에서 로그아웃돼요. 각 기기에서 비밀번호와 새 키트의 새 비밀 키를 한 번 입력하세요.",
  "rotation.review.secretKey.limit":
    "이전 비밀 키로는 더 이상 로그인할 수 없게 돼요. 이전 비밀번호와 비밀 키로 이미 보관함 데이터를 복사한 사람은 여전히 읽을 수 있어요. 이를 막으려면 계정 키를 재설정하세요.",
  "rotation.review.secretKey.compromised": "유출되었어요: 대신 키 재설정",
  "rotation.review.profile.changes":
    "잠금 해제를 시도할 때마다 필요한 메모리 양이 바뀌어요. 나에게도, 추측하는 사람에게도 마찬가지예요. 이 기기의 빠른 잠금 해제가 꺼지며 나중에 다시 설정할 수 있어요.",
  "rotation.review.profile.stays":
    "마스터 비밀번호, 비밀 키, 보관함, 보안 코드는 그대로예요. 등록한 보안 키는 계속 작동해요.",
  "rotation.review.profile.devices":
    "다른 기기는 동기화에서 로그아웃돼요. 각 기기에서 마스터 비밀번호를 한 번 입력하면 동기화가 계속돼요. 느린 기기는 잠금을 해제하는 데 시간이 조금 더 걸려요.",
  "rotation.review.rekey.intro":
    "키 재설정은 누군가 보관함 데이터와 비밀번호, 비밀 키를 가지고 있을 수 있는 경우를 위한 거예요. 기기만 잃어버렸다면 비밀 키를 교체하거나 비밀번호를 변경하는 것으로 충분해요.",
  "rotation.review.rekey.yes": "네, 계정 키를 재설정할게요",
  "rotation.review.rekey.no": "아니요, 비밀 키만 교체할게요",
  "rotation.review.rekey.limit":
    "키 재설정으로 이미 복사된 것을 되돌릴 수는 없어요. 서버와 백업에 있는 이전 사본은 이전 키로 암호화된 채 남아 있고, 이전 키로 여전히 열 수 있어요.",
  "rotation.review.rekey.title": "키 재설정이 하는 일",
  "rotation.review.rekey.vaults": "{count, plural, other {나만 사용하는 보관함 #개가 새 키를 받아요.}}",
  "rotation.review.rekey.keys":
    "새 비밀 키, 새 비상 키트, 새 보안 코드, 새 계정 키가 만들어져요.",
  "rotation.review.rekey.devices":
    "다른 기기는 로그아웃되며, 각 기기에서 새 비밀 키와 새 마스터 비밀번호를 입력하기 전에는 동기화할 수 없어요.",
  "rotation.review.rekey.shared":
    "{count, plural, other {보관함 #개를 공유하고 있어요.}} 각각 새 키를 받아요.",
  "rotation.review.rekey.sharedRow": "{name}, {count, plural, other {구성원 #명}}",
  "rotation.review.rekey.sharedMembers":
    "공유하는 사람들에게 내 새 보안 코드를 비교해 달라고 요청하세요. 비교하기 전에는 내게서 새 키를 받지 못하고 갱신된 보관함을 열 수 없어요.",
  "rotation.review.rekey.memberVaults":
    "{count, plural, other {보관함 #개는 다른 사람의 것이에요.}} 소유자에게 갱신을 요청하게 돼요. 갱신하기 전에는 내 이전 키로 여전히 읽을 수 있어요.",
  "rotation.review.rekey.grants":
    "{count, plural, other {#명이 내 보관함에 비상 접근 권한을 가지고 있어요.}} 이들의 접근 권한은 새 키로 갱신돼요. 이들도 각자 내 새 보안 코드를 비교해야 해요.",
  "rotation.review.rekey.quick":
    "이 기기의 빠른 잠금 해제가 꺼져요. 나중에 다시 설정할 수 있어요.",
  "rotation.review.rekey.hardwareKeys":
    "{count, plural, other {잠금 해제용으로 등록한 보안 키 #개가 꺼져요.}} 나중에 다시 등록할 수 있어요.",
  "rotation.review.rekey.hardwareVaults":
    "{count, plural, other {하드웨어로 보호되는 보관함 #개는 영향을 받지 않아요.}}",
  "rotation.review.rekey.attachments":
    "{count, plural, other {첨부 파일 #개가 이 기기에 없어요.}} 다시 보호할 수 있도록 먼저 다운로드해요. 가져올 수 없는 파일은 이전 키를 유지해요.",
  "rotation.review.rekey.folder":
    "동기화 폴더에 새 계정 파일이 생겨요. 다른 기기는 그곳에서 키가 바뀌었다는 안내를 발견해요.",
  "rotation.review.rekey.unavailable": "키 재설정을 사용할 수 없어요. 동기화가 설정되어 있지 않아요.",
  "rotation.review.loading": "키 재설정이 건드리는 항목을 확인하는 중…",
  "rotation.review.rekey.needsClean":
    "키 재설정을 하려면 동기화가 최신이고 경고가 없어야 해요. 지금 동기화하고 경고가 있으면 확인한 다음 다시 시도하세요.",
  "rotation.credentials.title": "본인 확인",
  "rotation.credentials.password.title": "마스터 비밀번호",
  "rotation.credentials.current": "현재 마스터 비밀번호",
  "rotation.credentials.new": "새 마스터 비밀번호",
  "rotation.credentials.confirm": "새 마스터 비밀번호를 다시 입력",
  "rotation.credentials.mismatch": "두 비밀번호가 서로 달라요.",
  "rotation.credentials.same": "새 마스터 비밀번호는 현재 비밀번호와 달라야 해요.",
  "rotation.credentials.hint":
    "짧고 복잡한 비밀번호보다 긴 암호 문구가 더 좋아요. 복구할 수 없어요. Mosaic은 이 비밀번호를 알지 못해요.",
  "rotation.credentials.code": "인증 앱의 6자리 코드",
  "rotation.credentials.profile": "보호 수준",
  "rotation.credentials.profile.standard": "표준: 시도당 메모리 256 MB",
  "rotation.credentials.profile.strong": "강력: 시도당 메모리 512 MB",
  "rotation.credentials.profile.max": "최대: 시도당 메모리 1 GB",
  "rotation.credentials.profile.slow":
    "이 수준으로 잠금을 해제하면 이 기기에서 2초 넘게 걸려요. 더 오래된 다른 기기에서는 훨씬 더 느려요.",
  "rotation.credentials.profile.force": "그래도 업그레이드",
  "rotation.kit.title": "새 비상 키트 저장",
  "rotation.kit.body":
    "이 키트에 새 비밀 키가 들어 있어요. 먼저 저장하세요. 키트가 있다고 확인하기 전에는 Mosaic이 계정을 변경하지 않아요. 키트가 없으면 새 기기에서 로그인할 수 없기 때문이에요.",
  "rotation.kit.save": "새 비상 키트 저장",
  "rotation.kit.saved": "저장했어요. 기기와 떨어진 안전한 곳에 보관하세요.",
  "rotation.kit.cancelled": "아직 아무것도 저장하지 않았어요.",
  "rotation.kit.verify": "저장한 키트에서 새 비밀 키의 마지막 그룹을 입력하세요.",
  "rotation.kit.confirm": "키트 확인",
  "rotation.kit.mistyped": "새 비밀 키의 끝부분이 아니에요. 저장한 키트를 확인하세요.",
  "rotation.kit.verified": "키트를 확인했어요.",
  "rotation.confirm.title.password": "마스터 비밀번호 변경",
  "rotation.confirm.title.secretKey": "비밀 키 교체",
  "rotation.confirm.title.profile": "보호 수준 업그레이드",
  "rotation.confirm.title.rekey": "계정 키 재설정",
  "rotation.confirm.body.rekey":
    "다른 기기가 로그아웃되고 모든 키가 갱신돼요. 끝날 때까지 Mosaic을 열어 두세요.",
  "rotation.confirm.body.secretKey":
    "다른 기기가 동기화에서 로그아웃돼요. 끝날 때까지 Mosaic을 열어 두세요.",
  "rotation.confirm.verb.password": "비밀번호 변경",
  "rotation.confirm.verb.secretKey": "비밀 키 교체",
  "rotation.confirm.verb.profile": "보호 수준 업그레이드",
  "rotation.confirm.verb.rekey": "계정 키 재설정",
  "rotation.run.title": "키를 갱신하는 중…",
  "rotation.run.keepOpen": "Mosaic을 열어 두세요. 첨부 파일이 많으면 몇 분 걸릴 수 있어요.",
  "rotation.progress.label": "진행 상황",
  "rotation.progress.count": "{total}개 중 {done}개",
  "rotation.progress.done": "완료",
  "rotation.phase.prepared": "준비 중",
  "rotation.phase.backendStarted": "계정에 새 키로 전환하도록 요청하는 중",
  "rotation.phase.backendAccepted": "계정이 새 키를 받아들였어요",
  "rotation.phase.folderWritten": "동기화 폴더에 새 계정 파일이 생겼어요",
  "rotation.phase.localCommitted": "이 기기는 이제 새 키를 사용해요",
  "rotation.phase.attachments": "첨부 파일을 다시 보호하는 중",
  "rotation.phase.uploading": "보호된 데이터를 보내는 중",
  "rotation.phase.done": "완료",
  "rotation.done.title.password": "마스터 비밀번호를 변경했어요",
  "rotation.done.title.secretKey": "비밀 키를 교체했어요",
  "rotation.done.title.profile": "보호 수준을 업그레이드했어요",
  "rotation.done.title.rekey": "계정 키를 재설정했어요",
  "rotation.done.body.password":
    "이제부터 새 마스터 비밀번호를 사용하세요. 다른 기기에서 한 번 물어봐요.",
  "rotation.done.body.secretKey":
    "새 비상 키트를 안전하게 보관하고 이전 키트는 삭제하세요. 다른 기기에서 새 비밀 키를 한 번 물어봐요.",
  "rotation.done.body.profile":
    "이제 잠금을 해제하는 데 메모리가 더 필요해요. 다른 기기에서 비밀번호를 한 번 물어봐요.",
  "rotation.done.body.rekey":
    "새 비상 키트를 안전하게 보관하고 이전 키트는 삭제하세요. 다른 기기에서 새 비밀 키와 새 마스터 비밀번호를 한 번 물어봐요.",
  "rotation.done.members":
    "보관함을 공유하는 모든 사람에게 내 새 보안 코드를 비교해 달라고 요청하세요. 비교하기 전에는 내게서 새 키를 받지 못해요.",
  "rotation.done.quick": "빠른 잠금 해제 다시 설정",
  "rotation.done.quick.body": "빠른 잠금 해제가 꺼졌어요.",
  "rotation.done.hardware": "보안 키 다시 등록",
  "rotation.done.hardware.body":
    "{count, plural, other {잠금 해제용 보안 키 #개가 꺼졌어요.}}",
  "rotation.done.unreachable.note":
    "이 파일들은 읽을 수 없어서 다시 암호화되지 않았어요. 제거하거나 다시 추가하기 전까지는 이전 파일 키를 유지하므로, 이전 키와 파일 사본을 가진 사람은 여전히 열 수 있어요.",
  "rotation.done.unreachable.file": "{item}: {file}",
  "rotation.done.unreachable.unknownItem": "알 수 없는 항목",
  "rotation.done.unreachable.unknownFile": "알 수 없는 파일",
  "rotation.done.unreachable.more": "{count, plural, other {외 파일 #개}}",
  "rotation.done.unfinished":
    "마지막 부분은 나중에 이어서 진행돼요. 끝나기 전에 Mosaic이 종료되면 다음에 키 교체 계속을 선택하세요.",
  "rotation.resume.title": "키 변경이 끝나지 않았어요",
  "rotation.resume.body.password":
    "키 변경이 중간에 끊겼어요. 잃은 것은 없어요. 계속을 선택해 비밀번호를 다시 입력하거나 취소하세요.",
  "rotation.resume.body.secretKey":
    "비밀 키 교체가 중간에 끊겼어요. 잃은 것은 없어요. 계속해서 끝내세요.",
  "rotation.resume.body.rekey":
    "계정 키 재설정이 중간에 끊겼어요. 잃은 것은 없어요. 계속해서 끝내세요.",
  "rotation.resume.body.profile":
    "보호 수준 업그레이드가 중간에 끊겼어요. 잃은 것은 없어요. 계속해서 끝내세요.",
  "rotation.resume.continue": "키 교체 계속",
  "rotation.resume.discard": "취소…",
  "rotation.resume.running": "계속하는 중…",
  "rotation.resume.progress": "{total}개 중 {done}개 완료. 끝날 때까지 Mosaic을 열어 두세요.",
  "rotation.resume.dialog.title": "키 교체 계속",
  "rotation.resume.dialog.password": "현재 사용 중인 마스터 비밀번호",
  "rotation.resume.dialog.group": "새 비밀 키의 마지막 그룹",
  "rotation.resume.dialog.body":
    "계정의 키가 이미 바뀌었을 수 있어요. 본인임을 확인하기 위해 마스터 비밀번호와 새 비상 키트에 있는 새 비밀 키의 마지막 그룹을 입력하세요.",
  "rotation.resume.dialog.submit": "계속",
  "rotation.resume.discard.title": "이 키 변경을 취소할까요?",
  "rotation.resume.discard.body":
    "이번 시도에서 만든 새 비밀 키와 새 비상 키트가 삭제돼요. 계정은 그대로 유지돼요.",
  "rotation.resume.discard.confirm": "키 변경 취소",
  "rotation.resume.abandon": "나머지를 포기…",
  "rotation.resume.stalled": "계속하는 작업이 끝나지 않았어요. 계속 실패하면 여기서 중단할 수 있어요.",
  "rotation.abandon.title": "파일 재암호화를 중단할까요?",
  "rotation.abandon.body":
    "새 키는 이미 사용 중이에요. Mosaic은 아직 대기 중인 첨부 파일의 재암호화를 중단해요. 이 파일들은 이전 파일 키를 유지하므로, 이전 키와 파일 사본을 가진 사람은 여전히 열 수 있어요.",
  "rotation.abandon.stays":
    "이미 다시 암호화된 것은 그대로 유지돼요. 대기 중인 파일은 나중에 제거했다가 다시 추가하면 새 키로 보호돼요.",
  "rotation.abandon.confirm": "재암호화 중단",
  "rotation.abandon.keep": "계속 시도",
  "rotation.marker.title": "확인되지 않은 키 재설정 알림",
  "rotation.marker.hint":
    "다른 기기에서 키를 갱신하지 않았다면 이 알림은 진짜가 아닐 수 있어요. 무시해도 돼요.",
  "rotation.marker.open": "이 알림 무시…",
  "rotation.marker.open.label": "이 알림 무시: {name}",
  "rotation.marker.confirm":
    "Mosaic이 확인할 수 없는 키 재설정 알림을 발견했어요. 다른 기기에서 계정 키를 재설정하지 않았다면 무시해도 돼요.",
  "rotation.marker.ignore": "알림 무시",
  "rotation.marker.keep": "알림 유지",
  "rotation.ignored.title": "무시한 키 재설정 알림",
  "rotation.ignored.body":
    "무시하기로 한 알림이에요. 실제로 다른 기기에서 계정 키를 재설정했다면 복원하세요.",
  "rotation.ignored.row": "동기화 폴더의 키 재설정 알림",
  "rotation.ignored.restore": "복원",
  "rotation.ignored.restore.label": "알림 {name} 복원",
  "rotation.resume.done": "키 변경을 마쳤어요.",
  "rotation.recovery.title": "다른 기기에서 계정의 키가 갱신되었어요",
  "rotation.recovery.body":
    "내 기기 중 하나가 이 계정의 키를 갱신했어요. 계속 동기화하려면 마스터 비밀번호와 새 비상 키트의 비밀 키를 입력하세요.",
  "rotation.recovery.password": "마스터 비밀번호 (그곳에서 변경했다면 새 비밀번호)",
  "rotation.recovery.secretKey": "새 비상 키트의 비밀 키",
  "rotation.recovery.submit": "새 키 사용",
  "rotation.recovery.warning":
    "직접 키를 갱신하지 않았나요? 그렇다면 이전 비밀번호와 비밀 키를 가진 누군가가 했을 수 있어요. 여기에 아무것도 입력하지 말고 먼저 다른 기기를 확인하세요.",
  "rotation.recovery.notice": "이 기기에서 이 작업을 하기 전까지 동기화가 일시 중지돼요.",
  "rotation.recovery.open": "새 키 사용…",
  "rotation.recovery.keyHint":
    "다른 기기에서 키를 갱신했다면 그 비상 키트의 새 비밀 키도 입력하세요.",
  "rotation.recovery.done": "이 기기는 이제 새 키를 사용해요.",
  "rotation.recovery.optional": "가장 최근 비상 키트의 비밀 키 (갱신한 경우에만)",
  "rotation.file.unknown":
    "동기화 폴더에 알 수 없는 계정 파일이 나타났어요. 내 키로는 확인할 수 없어요. 남은 파일일 수도 있고, 누군가 폴더를 변경했을 수도 있어요. 내 데이터는 영향을 받지 않아요.",
  "rotation.file.dismiss": "닫기",
  "rotation.file.conflict.title": "두 기기가 동시에 키를 갱신했어요",
  "rotation.file.conflict":
    "다른 기기가 이 기기와 같은 시점에 계정의 키를 갱신해서 둘이 더 이상 일치하지 않아요. 이 기기에서 키를 다시 재설정하면 하나로 맞춰져요. 그러면 이 기기의 키가 적용되고 다른 기기는 새 비밀 키를 물어봐요.",
  "rotation.file.conflict.rekey": "이 기기에서 키 다시 재설정",
  "rotation.file.conflict.keep": "알림 숨기기",
  "rotation.resume.body.vault":
    "보관함의 첨부 파일 재암호화가 중간에 끊겼어요. 잃은 것은 없어요. 계속해서 끝내세요.",
} satisfies Record<keyof typeof en, string>;
