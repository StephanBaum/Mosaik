/**
 * Korean UI strings: sync. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { sync as en } from "../en/sync.ts";

export const sync = {
  "sync.status.signedOut": "로그아웃됨",
  "sync.platform.windows": "Windows",
  "sync.platform.macos": "macOS",
  "sync.platform.linux": "Linux",
  "settings.sync.title": "동기화",
  "settings.sync.body":
    "모든 기기에서 보관함을 최신 상태로 유지해요. 모든 내용은 이 기기를 떠나기 전에 암호화돼요.",
  "settings.sync.notSetUp": "동기화가 꺼져 있어요. 보관함은 이 기기에만 저장돼요.",
  "settings.sync.deviceName": "이 기기의 이름",
  "settings.sync.deviceName.default": "{platform}의 Mosaic",
  "settings.sync.password": "마스터 비밀번호",
  "settings.sync.code": "6자리 코드",
  "settings.sync.server.title": "Mosaic 서버",
  "settings.sync.server.body": "내가 또는 신뢰하는 사람이 운영하는 Mosaic 서버로 동기화해요.",
  "settings.sync.server.url": "서버 주소",
  "settings.sync.server.urlHint": "예: https://sync.example.com",
  "settings.sync.server.check": "서버 확인",
  "settings.sync.server.checking": "서버를 확인하는 중…",
  "settings.sync.server.registration.open": "이 서버는 누구나 동기화 계정을 만들 수 있어요.",
  "settings.sync.server.registration.invite":
    "이 서버에서 동기화 계정을 만들려면 초대 코드가 필요해요.",
  "settings.sync.server.registration.closed":
    "이 서버는 새 동기화 계정을 받지 않아요. 기존 계정으로는 계속 로그인할 수 있어요.",
  "settings.sync.server.choice.label": "동기화 계정",
  "settings.sync.server.choice.signIn": "동기화 계정이 있어요",
  "settings.sync.server.choice.create": "새로 만들기",
  "settings.sync.server.invite": "초대 코드",
  "settings.sync.server.create": "동기화 계정 만들기",
  "settings.sync.server.signIn": "로그인",
  "settings.sync.server.signInAgain": "다시 로그인",
  "settings.sync.server.working": "연결하는 중…",
  "settings.sync.server.sees":
    "서버는 암호화된 데이터, 계정 이메일, 동기화 시점만 볼 수 있어요. 마스터 비밀번호나 비밀 키는 절대 볼 수 없어요.",
  "settings.sync.twoFactor.title": "코드 입력",
  "settings.sync.twoFactor.body":
    "이 동기화 계정은 2단계 인증을 사용해요. 인증 앱의 6자리 코드를 입력하세요.",
  "settings.sync.twoFactor.submit": "확인",
  "settings.sync.folder.title": "내 클라우드 폴더",
  "settings.sync.folder.body":
    "클라우드 서비스(Dropbox, OneDrive, iCloud Drive 등)가 최신 상태로 유지하는 폴더로 동기화해요.",
  "settings.sync.folder.choose": "폴더 선택…",
  "settings.sync.folder.chooseOther": "다른 폴더 선택…",
  "settings.sync.folder.path": "폴더",
  "settings.sync.folder.contents.empty": "폴더가 비어 있어요. Mosaic이 사용할 수 있어요.",
  "settings.sync.folder.contents.mosaic": "이 폴더에는 이미 Mosaic 동기화 데이터가 있어요.",
  "settings.sync.folder.contents.other":
    "이 폴더에 다른 파일이 있어요. Mosaic은 그곳에 아무것도 쓰지 않아요. 빈 폴더를 선택하거나 새로 만드세요.",
  "settings.sync.folder.connect": "폴더 연결",
  "settings.sync.folder.working": "연결하는 중…",
  "settings.sync.folder.sees":
    "클라우드 제공업체와 폴더에 접근할 수 있는 사람은 암호화된 데이터, 계정 ID, 파일이 변경되는 시점만 볼 수 있어요. 마스터 비밀번호나 비밀 키는 절대 볼 수 없어요.",
  "settings.sync.status.label": "상태",
  "settings.sync.lastSynced": "마지막 동기화 {when}",
  "settings.sync.neverSynced": "아직 동기화하지 않음",
  "settings.sync.offline":
    "오프라인이에요. 변경 사항은 이 기기에 저장되었으며 다시 온라인이 되면 동기화돼요.",
  "settings.sync.now": "지금 동기화",
  "settings.sync.backend.server": "서버",
  "settings.sync.backend.account": "동기화 계정",
  "settings.sync.backend.folder": "폴더",
  "settings.sync.backend.device": "이 기기",
  "settings.sync.signedOut.title": "이 기기가 로그아웃되었어요",
  "settings.sync.signedOut.body":
    "서버가 이 기기의 세션을 종료했어요. 계속 동기화하려면 다시 로그인하세요. 변경 사항은 이 기기에 안전하게 있어요.",
  "settings.sync.recover.title": "마스터 비밀번호가 변경되었어요",
  "settings.sync.recover.body":
    "서버나 폴더에 이 기기보다 새로운 마스터 비밀번호가 있어요. 이 기기를 최신 상태로 만들려면 새 마스터 비밀번호를 입력하세요. 보관함과 비밀 키는 그대로예요.",
  "settings.sync.recover.password": "새 마스터 비밀번호",
  "settings.sync.recover.submit": "이 기기 업데이트",
  "settings.sync.disconnect": "연결 해제…",
  "settings.sync.disconnect.title": "동기화 연결을 해제할까요?",
  "settings.sync.disconnect.body":
    "이 기기는 동기화를 멈추고 서버에서 로그아웃해요. 보관함은 이 기기에 남고, 다른 기기는 각자의 사본을 유지해요.",
  "settings.sync.disconnect.confirm": "연결 해제",
  "settings.sync.devices.title": "기기",
  "settings.sync.devices.thisDevice": "이 기기",
  "settings.sync.devices.unnamed": "이름 없는 기기",
  "settings.sync.devices.lastSeen": "마지막 접속 {when}",
  "settings.sync.devices.neverSeen": "아직 접속 기록 없음",
  "settings.sync.devices.rename": "이름 변경",
  "settings.sync.devices.renameNamed": "{name} 이름 변경",
  "settings.sync.devices.renameLabel": "이 기기의 새 이름",
  "settings.sync.devices.remove": "제거",
  "settings.sync.devices.removeNamed": "{name} 제거",
  "settings.sync.devices.removeTitle": "{name}을(를) 제거할까요?",
  "settings.sync.devices.removeBody":
    "기기를 제거하면 동기화에서 로그아웃돼요. 이미 가지고 있는 보관함 사본은 사라지지 않아요. 기기의 접근을 완전히 막으려면 마스터 비밀번호와 비밀 키를 변경하세요.",
  "settings.sync.devices.removeConfirm": "기기 제거",
  "settings.sync.devices.folderNote":
    "기기의 접근을 막으려면 마스터 비밀번호와 비밀 키를 변경하세요.",
  "settings.sync.phone.title": "휴대폰 설정",
  "settings.sync.phone.body":
    "휴대폰이 스캔할 코드를 표시해서 비밀 키를 입력하지 않아도 돼요. 휴대폰에는 여전히 마스터 비밀번호가 필요해요.",
  "settings.sync.phone.button": "휴대폰 설정",
  "settings.sync.phone.reauth":
    "코드에 비밀 키가 들어 있으므로 마스터 비밀번호가 필요해요.",
  "settings.sync.phone.dialogTitle": "휴대폰으로 이 코드를 스캔하세요",
  "settings.sync.phone.steps":
    "휴대폰에서 Mosaic을 열고 설정을 선택한 다음 이 코드를 스캔하세요.",
  "settings.sync.phone.hint":
    "코드는 60초 후에 또는 이 창을 벗어나면 숨겨져요. 코드를 사진으로 공유하지 마세요.",
  "settings.sync.phone.canvas": "휴대폰용 설정 코드",
  "settings.sync.phone.hidden.time": "60초가 지나서 코드가 숨겨졌어요.",
  "settings.sync.phone.hidden.away": "이 창을 벗어나서 코드가 숨겨졌어요.",
  "settings.sync.phone.again": "다시 표시",
  "settings.sync.phone.folderHint":
    "휴대폰이 클라우드 저장소에 로그인해서 같은 폴더를 열어요.",
  "settings.sync.totp.title": "인증 앱",
  "settings.sync.totp.body":
    "누군가 동기화 계정에 로그인할 때 인증 앱의 코드를 요구해요. 서버 계정을 보호하는 것이며 보관함을 보호하지는 않아요. 보관함은 마스터 비밀번호와 비밀 키로 계속 암호화돼요.",
  "settings.sync.totp.on": "인증 앱 코드가 켜져 있어요.",
  "settings.sync.totp.off": "인증 앱 코드가 꺼져 있어요.",
  "settings.sync.totp.enable": "켜기…",
  "settings.sync.totp.disable": "끄기…",
  "settings.sync.totp.beginTitle": "인증 앱 코드 켜기",
  "settings.sync.totp.beginBody": "시작하려면 마스터 비밀번호를 입력하세요.",
  "settings.sync.totp.begin": "계속",
  "settings.sync.totp.setupBody":
    "이 키를 인증 앱에 추가한 다음 앱에 표시되는 6자리 코드를 입력하세요.",
  "settings.sync.totp.secret": "설정 키",
  "settings.sync.totp.confirm": "켜기",
  "settings.sync.totp.disableTitle": "인증 앱 코드 끄기",
  "settings.sync.totp.disableBody": "마스터 비밀번호와 현재 코드를 입력하세요.",
  "settings.sync.totp.disableConfirm": "끄기",
  "settings.sync.totp.enabledToast": "인증 앱 코드가 켜져 있어요.",
  "settings.sync.totp.disabledToast": "인증 앱 코드가 꺼져 있어요.",
  "settings.sync.key.title": "보안 키",
  "settings.sync.key.body":
    "보안 키는 동기화 계정에 로그인할 때 두 번째 단계가 될 수 있어요. 서버의 계정을 보호하는 것이며 보관함을 보호하지는 않아요. 키를 추가하거나 사용하면 브라우저가 열려요.",
  "settings.sync.key.none": "추가된 보안 키가 없어요.",
  "settings.sync.key.added": "{when}에 추가됨",
  "settings.sync.key.lastUsed": "마지막 사용 {when}",
  "settings.sync.key.neverUsed": "아직 사용 안 함",
  "settings.sync.key.add": "보안 키 추가…",
  "settings.sync.key.addTitle": "보안 키 추가",
  "settings.sync.key.addBody":
    "키의 이름을 정하고 마스터 비밀번호를 입력하세요. 그러면 Mosaic이 브라우저를 열고 그곳에서 키를 사용해요.",
  "settings.sync.key.label": "키 이름",
  "settings.sync.key.labelHint": "예: “책상 키”. 최대 64자.",
  "settings.sync.key.addConfirm": "계속",
  "settings.sync.key.addedToast": "보안 키를 추가했어요.",
  "settings.sync.key.noBackup":
    "보안 키가 유일한 두 번째 단계가 돼요. 이 키를 잃어버리면 동기화 계정에 다시 로그인하지 못할 수 있어요. 백업을 권장해요. 두 번째 키나 인증 앱의 코드를 사용하세요.",
  "settings.sync.key.recommend":
    "두 가지 방법을 권장해요. 예를 들어 키 두 개, 또는 키 하나와 인증 앱 코드예요.",
  "settings.sync.key.remove": "제거",
  "settings.sync.key.removeLabel": "{name} 제거",
  "settings.sync.key.removeTitle": "{name}을(를) 제거할까요?",
  "settings.sync.key.removeBody":
    "브라우저에서 보안 키로 한 번 확인해요. 그 후에는 이 키로 로그인할 수 없어요.",
  "settings.sync.key.removeLast":
    "마지막 두 번째 단계예요. 2단계 인증이 꺼지고, 이후 동기화 계정은 마스터 비밀번호와 비밀 키만으로 보호돼요.",
  "settings.sync.key.removeCode": "인증 앱 코드",
  "settings.sync.key.removeCodeHint":
    "키를 바로 제거하려면 현재 코드를 입력하세요. 비워 두면 대신 브라우저에서 보안 키로 확인해요.",
  "settings.sync.key.removeConfirm": "키 제거",
  "settings.sync.key.removedToast": "보안 키를 제거했어요.",
  "settings.sync.key.confirmTitle": "보안 키로 확인",
  "settings.sync.key.confirmBody":
    "키를 제거하려면 두 번째 단계가 필요해요. 마스터 비밀번호를 입력한 다음 브라우저에서 키로 확인하세요.",
  "settings.sync.key.step.methods": "이 계정의 로그인 방식을 확인하는 중…",
  "settings.sync.key.step.useKey": "보안 키 사용 (브라우저가 열려요)",
  "settings.sync.key.step.keyOnlyTitle": "보안 키 사용",
  "settings.sync.key.step.keyOnlyBody":
    "이 계정은 보안 키로 보호돼요. Mosaic이 브라우저에서 페이지를 열고 그곳에서 키를 사용해요.",
  "settings.sync.key.step.or": "또는 코드 사용",
  "settings.sync.key.wait.login": "보안 키로 로그인",
  "settings.sync.key.wait.register": "보안 키 추가",
  "settings.sync.key.wait.confirm": "보안 키로 확인",
  "settings.sync.key.wait.lead":
    "브라우저에서 동기화 서버의 Mosaic 페이지가 열렸어요. 그곳에 이 코드를 입력한 다음 보안 키를 사용하세요.",
  "settings.sync.key.wait.codeLabel": "브라우저에 입력할 코드",
  "settings.sync.key.wait.codeSpoken": "브라우저에 입력할 코드: {digits}",
  "settings.sync.key.wait.expires": "이 단계는 {time}에 만료돼요.",
  "settings.sync.key.wait.waiting": "보안 키를 기다리는 중…",
  "settings.sync.key.wait.done": "완료.",
  "settings.sync.key.wait.expired": "이 단계의 시간이 지났어요. 다시 시작하세요.",
  "settings.sync.key.wait.failed":
    "단계가 실패했어요. 코드를 너무 많이 틀렸거나 보안 키가 받아들여지지 않았어요. 아무것도 변경되지 않았어요. 다시 시작하세요.",
  "settings.sync.key.wait.reopen": "브라우저 페이지 다시 열기",
  "settings.sync.key.wait.retry": "다시 시작",
  "sync.warning.region": "동기화 경고",
  "sync.warning.unknownDevice": "다른 기기",
  "sync.warning.unknownVault": "내 보관함 중 하나",
  "sync.warning.freezeSuspected":
    "{device}의 변경 사항이 하루 넘게 도착하지 않았어요. 다른 기기가 켜져 있고 온라인인지 확인하세요.",
  "sync.warning.missingFiles":
    "{device}의 파일이 폴더에 없어요. 클라우드 서비스가 동기화를 마치게 한 다음 다시 동기화하세요.",
  "sync.warning.noCommonView":
    "이 기기와 {device}이(가) 이틀 동안 같은 서버 데이터를 보지 못했어요. 서버가 서로 다른 버전을 보여 주고 있을 수 있어요.",
  "sync.warning.rollbackDetected":
    "동기화 데이터가 이 기기보다 오래된 것 같아요. 서버나 폴더를 확인한 다음 “확인했어요”를 선택하세요.",
  "sync.warning.identityDuplicated":
    "예를 들어 백업을 복원한 후처럼, 다른 Mosaic 사본이 이 기기의 ID로 쓰고 있어요. 동기화를 일시 중지했어요. 사본이 하나만 사용되는지 확인한 다음 “확인했어요”를 선택하세요.",
  "sync.warning.serverInconsistent":
    "서버가 이 기기에 다른 기기와 다른 데이터를 보여 줬어요. 서버를 확인한 다음 “확인했어요”를 선택하세요.",
  "sync.warning.serverRollback":
    "서버에 있는 {vault}의 사본이 이 기기의 것보다 오래되었어요. 서버를 확인한 다음 “확인했어요”를 선택하세요.",
  "sync.warning.undecryptable":
    "{count, plural, other {받은 항목 #개}}를 이 기기에서 읽을 수 없었어요. 모든 기기에서 Mosaic을 업데이트한 다음 다시 동기화하세요.",
  "sync.warning.hardwareVaultsLocal":
    "하드웨어로 보호되는 보관함은 이 기기에만 있어요. {count, plural, other {보관함 #개는}} 동기화되지 않아요.",
  "sync.warning.attachmentsNotRepaired":
    "서버나 폴더의 {count, plural, other {파일 #개}}를 이 기기에서 복구할 수 없었어요. 파일이 아직 있는 기기에서 항목을 열고 다시 추가하세요.",
  "sync.warning.acknowledge": "확인했어요…",
  "sync.acknowledge.title": "확인이 필요한 동기화 경고를 지울까요?",
  "sync.acknowledge.body":
    "서버나 폴더를 직접 확인한 후에만 진행하세요. 확인이 필요한 모든 경고가 지워지고 동기화가 계속돼요. 데이터가 실제로 이전 상태로 되돌려졌거나 변경되었더라도 이후에는 Mosaic이 알 수 없어요.",
  "sync.acknowledge.eachDevice": "모든 기기에서 각각 이 작업을 하세요.",
  "toast.syncAttention":
    "동기화가 다른 기기의 일부 항목을 병합하거나 복구했어요. 무엇이 바뀌었는지 확인해 보세요.",
  "sync.revoked.rekey":
    "다른 기기에서 이 계정의 키가 갱신되었어요. 가장 최근 비상 키트의 새 비밀 키로 로그인하세요.",
  "sync.revoked.passwordChange":
    "다른 기기에서 마스터 비밀번호가 변경되었어요. 새 비밀번호로 로그인하세요.",
  "sync.revoked.deviceRemoved":
    "다른 기기에서 이 기기가 계정에서 제거되었어요. 다시 추가하려면 다시 로그인하세요.",
  "sync.revoked.operator":
    "서버 운영자가 이 세션을 종료했어요. 계속 동기화하려면 다시 로그인하세요.",
} satisfies Record<keyof typeof en, string>;
