/**
 * Korean UI strings: settings. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { settings as en } from "../en/settings.ts";

export const settings = {
  "settings.title": "설정",
  "settings.security": "보안",
  "settings.autoLock": "사용하지 않으면 잠금",
  "settings.autoLock.never": "안 함",
  "settings.autoLock.minutes": "{minutes, plural, other {#분}}",
  "settings.lockOnSleep": "컴퓨터가 절전 모드일 때 잠금",
  "settings.lockOnScreenLock": "화면이 잠길 때 잠금",
  "settings.screenLock.unsupported":
    "이 시스템에서는 Mosaic이 화면이 잠기는 시점을 알 수 없어요. 사용하지 않을 때와 컴퓨터가 깨어날 때는 그래도 잠겨요.",
  "settings.lockTrigger.required":
    "잠금 조건을 하나 이상 켜 두세요. 이 시스템에서는 Mosaic이 화면 잠금을 감지할 수 없어요.",
  "settings.clipboard": "클립보드 지우기",
  "settings.clipboard.never": "안 함",
  "settings.clipboard.seconds": "{seconds, plural, other {#초}} 후",
  "settings.appearance": "화면 모드",
  "settings.theme": "테마",
  "settings.theme.system": "시스템 설정 따르기",
  "settings.theme.light": "라이트",
  "settings.theme.dark": "다크",
  "settings.language": "언어",
  "settings.language.system": "시스템 설정 따르기",
  "settings.language.machineNote": "AI의 도움으로 번역했어요. 어색한 부분이 있으면 알려 주세요.",
  "settings.import": "가져오기",
  "settings.import.body":
    "브라우저나 다른 비밀번호 관리자에서 로그인을 가져와요. Mosaic이 내보내기 과정을 안내하고, 이후 파일을 삭제하는 것도 도와줘요.",
  "settings.import.open": "비밀번호 가져오기",
  "settings.ssh": "SSH 에이전트",
  "settings.ssh.enable": "Mosaic의 SSH 키 사용",
  "settings.ssh.hint":
    "기본적으로 꺼져 있어요. 에이전트는 Mosaic이 잠금 해제된 동안에만 실행되며 사용할 때마다 먼저 물어봐요.",
  "settings.ssh.status": "{count, plural, =0 {제공 중인 키 없음} other {제공 중인 키 #개}}",
  "settings.ssh.endpoint": "SSH_AUTH_SOCK 설정값",
  "settings.ssh.skipped": "제공되지 않는 키",
  "settings.kit": "비상 키트",
  "settings.kit.save": "비상 키트 다시 저장",
  "settings.saved": "설정을 저장했어요.",
  "reauth.title": "마스터 비밀번호로 확인",
  "reauth.body": "이 변경은 Mosaic의 보안을 낮추므로 마스터 비밀번호가 필요해요.",
  "reauth.bodyKit":
    "비밀 키가 파일에 표시될 예정이라 Mosaic에 마스터 비밀번호가 필요해요.",
  "reauth.password": "마스터 비밀번호",
  "reauth.submit": "확인",
  "settings.intro": "이 기기에서 Mosaic이 잠기고, 비밀 정보를 지우고, 키를 보호하는 방식이에요.",
  "settings.section.autoLock": "자동 잠금",
  "settings.section.autoLock.body": "직접 잠그지 않아도 Mosaic이 스스로 잠기는 시점이에요.",
  "settings.section.clipboard": "클립보드",
  "settings.section.clipboard.body": "복사한 비밀 정보는 이 시간이 지나면 지워져요.",
  "settings.appearance.body": "Mosaic의 테마와 언어예요.",
  "settings.ssh.body": "프로그램이 보관함의 SSH 키로 로그인할 수 있게 해요. 사용할 때마다 직접 승인해요.",
  "settings.kit.body": "비상 키트의 새 사본을 저장해요. 마스터 비밀번호가 필요해요.",
  "settings.quick": "빠른 잠금 해제",
  "settings.quick.body": "마스터 비밀번호 외에 더 빠르게 다시 들어가는 방법이에요.",
  "settings.quick.windowsHello": "Windows Hello",
  "settings.quick.touchId": "Touch ID",
  "settings.quick.biometric.body":
    "얼굴, 지문 또는 PIN으로 잠금을 해제해요. 다시 시작한 후에는 마스터 비밀번호가 여전히 필요해요.",
  "settings.quick.biometric.bodyMac":
    "지문으로 잠금을 해제해요. 다시 시작한 후에는 마스터 비밀번호가 여전히 필요해요.",
  "settings.quick.expires": "{date, date, medium}에 Mosaic이 마스터 비밀번호를 다시 물어봐요.",
  "settings.quick.maxAge": "마스터 비밀번호를 최소한 이 주기로 요구",
  "settings.quick.maxAgeDays": "{days, plural, other {#일}}마다",
  "settings.quick.biometricLinux": "Linux에서는 생체 인식 잠금 해제를 사용할 수 없어요.",
  "settings.quick.biometricUnavailable": "이 기기에 설정되어 있지 않아요.",
  "settings.quick.keys": "보안 키",
  "settings.quick.keysCount": "{count, plural, =0 {등록된 키 없음} other {#개 등록됨}}",
  "settings.quick.keysUnsupported": "이 시스템에서는 아직 보안 키를 지원하지 않아요.",
  "settings.quick.keyRow": "보안 키 · {tag} · {addedAt, date, medium}에 추가됨",
  "settings.quick.addKey": "보안 키 추가",
  "settings.quick.addKeyBody":
    "추가할 키를 꽂고 PIN을 입력하세요. 그런 다음 키가 깜박이면 터치하세요.",
  "settings.quick.manageKeys": "관리",
  "settings.quick.removeKey": "제거",
  "settings.quick.removeKeyNamed": "보안 키 {tag} 제거",
  "settings.quick.removeKeyConfirm": "보안 키 {tag}을(를) 제거할까요?",
  "settings.quick.keyRisk":
    "이 키와 PIN을 가진 사람은 누구나 마스터 비밀번호 없이 보관함을 열 수 있어요.",
  "reauth.bodyQuick": "Mosaic에 더 쉽게 들어갈 수 있게 되므로 마스터 비밀번호가 필요해요.",
  "reauth.bodyKey": "이 보관함에 키를 추가하려면 마스터 비밀번호가 필요해요.",
  "settings.passkeyExport": "패스키 내보내기",
  "settings.passkeyExport.body":
    "다른 앱으로 옮기기 위해 보관함의 패스키를 FIDO CXF 파일로 저장해요.",
  "settings.passkeyExport.vault": "보관함",
  "settings.passkeyExport.warning":
    "이 파일에는 패스키가 암호화되지 않은 상태로 들어 있어요. 파일을 가진 사람은 누구나 나로 로그인할 수 있어요. 가져온 후에는 삭제하세요.",
  "settings.passkeyExport.none": "이 보관함에는 내보낼 패스키가 없어요.",
  "settings.passkeyExport.button": "패스키 내보내기",
  "settings.passkeyExport.reauth":
    "파일에 패스키가 암호화되지 않은 상태로 들어가므로 내보내려면 마스터 비밀번호가 필요해요.",
  "settings.passkeyExport.saved":
    "{count, plural, other {패스키 #개를 내보냈어요.}} 가져온 후에는 파일을 삭제하세요.",
  "reauth.bodyMove":
    "보안 키로 보호된 보관함에서 항목을 옮기려면 마스터 비밀번호가 필요해요.",
  "toast.settingsReset":
    "보안 설정이 Mosaic 밖에서 변경되어 안전한 기본값으로 재설정되었어요.",
  "settings.shortcut.title": "빠른 검색 단축키",
  "settings.shortcut.body": "어느 프로그램에서든 이 조합을 누르면 Mosaic 검색이 열려요.",
  "settings.shortcut.label": "단축키",
  "settings.shortcut.hint":
    "입력란을 클릭한 다음 원하는 키를 누르세요. Ctrl, Alt 또는 ⌘(Mac)에 키 하나를 더 누르면 돼요.",
  "settings.shortcut.save": "단축키 저장",
  "settings.shortcut.reset": "기본값 사용",
  "settings.shortcut.saved": "단축키를 저장했어요: {shortcut}.",
  "settings.shortcut.invalid":
    "Mosaic에서 사용할 수 없는 단축키예요. Ctrl, Alt 또는 ⌘(Mac)와 키 하나를 함께 누르세요.",
  "settings.shortcut.unsupported":
    "이 시스템에서는 앱이 전역 단축키를 설정할 수 없어서 여기에서 변경할 수 없어요.",
} satisfies Record<keyof typeof en, string>;
