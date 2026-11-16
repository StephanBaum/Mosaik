/**
 * Korean UI strings: browser. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { browser as en } from "../en/browser.ts";

export const browser = {
  "browser.pair.title": "브라우저가 Mosaic에 연결을 요청해요",
  "browser.pair.description":
    "방금 Mosaic 확장 프로그램에서 연결을 눌렀고 이 코드가 표시된 경우에만 허용하세요.",
  "browser.pair.code": "코드",
  "browser.pair.browser": "브라우저",
  "browser.pair.label": "이 브라우저의 이름",
  "browser.pair.labelHint": "이 이름은 설정에서 볼 수 있어요.",
  "browser.pair.deny": "거부",
  "browser.pair.allow": "허용",
  "browser.expires": "{seconds, plural, other {#초}} 후 자동으로 닫혀요",
  "browser.escDenies": "거부하려면",
  "browser.hint.chrome": "Chrome",
  "browser.hint.edge": "Edge",
  "browser.hint.brave": "Brave",
  "browser.hint.opera": "Opera",
  "browser.hint.vivaldi": "Vivaldi",
  "browser.hint.chromium": "Chromium",
  "browser.hint.firefox": "Firefox",
  "browser.hint.other": "브라우저",
  "browser.family.chromium": "Chromium 기반 브라우저",
  "browser.family.firefox": "Firefox",
  "browser.escCancels": "취소하려면",
  "browser.save.titleNew": "이 로그인을 Mosaic에 저장할까요?",
  "browser.save.titleUpdate": "Mosaic의 이 로그인을 업데이트할까요?",
  "browser.save.description":
    "{browser}에서 로그인 저장을 제안했어요. 저장을 선택해야만 Mosaic이 기록해요.",
  "browser.save.descriptionUpdate":
    "{browser}에서 이 로그인의 새 비밀번호를 제안했어요. 비밀번호 업데이트를 선택해야만 Mosaic이 변경해요.",
  "browser.save.site": "웹사이트",
  "browser.save.username": "사용자 이름",
  "browser.save.noUsername": "없음",
  "browser.save.password": "비밀번호",
  "browser.save.passwordHidden": "숨김",
  "browser.save.target": "저장 방식",
  "browser.save.updateOption": "“{title}” 업데이트",
  "browser.save.newOption": "새 로그인",
  "browser.save.itemTitle": "제목",
  "browser.save.vault": "보관함",
  "browser.save.allowHttp": "암호화되지 않은 이 페이지에서 자동 입력 허용",
  "browser.save.allowHttpHint":
    "이 사이트는 HTTPS를 사용하지 않아요. 같은 네트워크에 있는 누구나 입력한 내용을 읽을 수 있어요.",
  "browser.save.cancel": "취소",
  "browser.save.saveNew": "저장",
  "browser.save.update": "비밀번호 업데이트",
  "browser.passkey.secure": "보안 연결 (https)",
  "browser.passkey.getTitle": "패스키로 {rpId}에 로그인",
  "browser.passkey.createTitle": "새 패스키 저장",
  "browser.passkey.excludedTitle": "이미 저장된 패스키",
  "browser.passkey.createBody": "Mosaic에서 {rpId}의 패스키를 만들까요?",
  "browser.passkey.excludedBody": "Mosaic에 이미 {rpId}의 패스키가 있어요.",
  "browser.passkey.uvNote":
    "사이트에서 사용자 확인을 요구했어요. 마스터 비밀번호를 입력해 본인임을 확인하세요.",
  "browser.passkey.password": "마스터 비밀번호",
  "browser.passkey.account": "계정",
  "browser.passkey.noName": "이름 없음",
  "browser.passkey.vault": "보관함",
  "browser.passkey.signIn": "패스키로 로그인",
  "browser.passkey.save": "패스키 저장",
  "browser.passkey.anotherDevice": "다른 기기 또는 보안 키 사용",
  "browser.passkey.cancel": "취소",
  "browser.passkey.ok": "확인",
  "browser.passkey.escCloses": "닫으려면",
  "browser.passkey.passwordNeeded": "마스터 비밀번호를 입력해 본인임을 확인하세요.",
  "browser.settings.title": "브라우저",
  "browser.settings.body":
    "이 앱의 로그인을 자동으로 입력하는 브라우저 확장 프로그램이에요. 연결은 여기에서 하나씩 승인해요.",
  "browser.settings.empty":
    "아직 연결된 브라우저가 없어요. Mosaic 확장 프로그램을 설치한 다음 확장 프로그램에서 연결을 선택하세요.",
  "browser.settings.connected": "현재 연결됨",
  "browser.settings.lastUsed": "마지막 사용 {when}",
  "browser.settings.neverUsed": "아직 사용 안 함",
  "browser.settings.disconnect": "연결 해제",
  "browser.settings.disconnectNamed": "{name} 연결 해제",
  "browser.settings.disconnectTitle": "“{name}” 연결을 해제할까요?",
  "browser.settings.disconnectBody":
    "확장 프로그램에서 다시 연결하기 전에는 이 브라우저가 로그인을 자동으로 입력할 수 없어요.",
  "browser.settings.endpointInUse":
    "다른 프로그램이나 다른 Mosaic 창이 이미 브라우저 연결을 사용하고 있어요. 그것을 닫은 다음 Mosaic을 다시 시작하세요.",
  "browser.settings.endpointUnavailable":
    "Mosaic이 브라우저 연결을 열 수 없었어요. Mosaic을 다시 시작하세요. 계속되면 사용자 폴더에 쓰기 권한이 있는지 확인하세요.",
  "browser.settings.registryUnreadable":
    "저장된 브라우저 연결을 읽을 수 없어요. 이 문제가 해결될 때까지 Mosaic은 연결을 변경하지 않아요.",
  "browser.connector.title": "브라우저 연결이 다른 곳을 가리키고 있어요",
  "browser.connector.body":
    "{browser}의 연결이 Mosaic이 아닌 다른 프로그램을 시작해요: {path}. 이전 버전의 Mosaic이거나 스스로 등록한 다른 프로그램일 수 있어요. 복구하면 이 Mosaic을 다시 가리키게 돼요.",
  "browser.connector.item": "{browser}: {path}",
  "browser.connector.repair": "복구",
  "browser.connector.repaired": "브라우저 연결을 복구했어요.",
} satisfies Record<keyof typeof en, string>;
