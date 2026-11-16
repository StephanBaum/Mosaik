/**
 * Korean UI strings: otp. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { otp as en } from "../en/otp.ts";

export const otp = {
  "otp.section.label": "일회용 코드 (2단계 인증)",
  "otp.section.empty": "사이트의 2단계 인증 설정 페이지에 있는 코드를 추가하세요.",
  "otp.section.required": "이 항목을 저장하려면 일회용 코드를 추가하세요.",
  "otp.source.screen": "화면 스캔",
  "otp.source.clipboard": "스크린샷 붙여넣기",
  "otp.source.file": "이미지 선택…",
  "otp.source.manual": "스캔할 수 없나요? 키 입력",
  "otp.source.screenHint":
    "Mosaic이 잠시 물러나 화면을 한 번 살펴보고 코드 외에는 아무것도 보관하지 않아요.",
  "otp.source.clipboardHint":
    "{platform, select, mac {Cmd+Ctrl+Shift+4를 누르고 QR 코드를 선택한 다음 여기에 붙여넣으세요.} windows {Win+Shift+S를 누르고 QR 코드를 선택한 다음 여기에 붙여넣으세요.} other {QR 코드의 스크린샷을 복사한 다음 여기에 붙여넣으세요.}}",
  "otp.dialog.title": "일회용 코드 설정",
  "otp.working.screen": "화면에서 QR 코드를 찾는 중…",
  "otp.working.image": "QR 코드를 읽는 중…",
  "otp.manual.title": "설정 키 입력",
  "otp.manual.input": "설정 키 또는 otpauth:// 링크",
  "otp.manual.inputHint":
    "사이트에서 QR 코드 옆에 표시해 줘요. 보통 “스캔할 수 없나요?” 또는 “직접 입력” 아래에 있어요. 공백은 상관없어요.",
  "otp.manual.issuer": "서비스",
  "otp.manual.account": "계정",
  "otp.manual.advanced": "고급",
  "otp.manual.algorithm": "알고리즘",
  "otp.manual.digits": "자릿수",
  "otp.manual.period": "새 코드 주기",
  "otp.manual.periodValue": "{seconds}초",
  "otp.manual.advancedHint":
    "사이트에서 지시한 경우에만 변경하세요. 대부분의 사이트는 SHA-1, 6자리, 30초를 사용해요.",
  "otp.manual.continue": "계속",
  "otp.confirm.title": "올바른 코드인가요?",
  "otp.confirm.issuer": "서비스",
  "otp.confirm.account": "계정",
  "otp.confirm.details":
    "{scheme, select, hotp {카운터 기반, {digits}자리} steam {Steam Guard, 5자} other {{digits}자리, {period}초마다 새 코드}}",
  "otp.confirm.algorithmValue":
    "{algorithm, select, sha256 {SHA-256} sha512 {SHA-512} other {SHA-1}}",
  "otp.confirm.codeLabel": "현재 코드",
  "otp.confirm.codeHint": "사이트에서 설정을 마치려고 코드를 요구하면 이 코드를 입력하세요.",
  "otp.confirm.hotpNote":
    "카운터 기반 코드는 저장한 후에 표시돼요. 그래야 카운터가 사이트와 맞춰져요.",
  "otp.confirm.weak":
    "이 비밀 값은 비정상적으로 짧아요. 작동은 하지만 이 사이트의 2단계 인증 보호가 보통보다 약해요.",
  "otp.confirm.use": "이 코드 사용",
  "otp.confirm.back": "다시 스캔",
  "otp.confirm.copy": "코드 복사",
  "otp.summary.noIssuer": "일회용 코드",
  "otp.summary.pending": "이 항목과 함께 저장돼요.",
  "otp.summary.removed": "저장하면 일회용 코드가 제거돼요.",
  "otp.action.replace": "교체",
  "otp.action.remove": "일회용 코드 제거",
  "otp.action.keep": "유지",
  "otp.action.cancelSetup": "추가 안 함",
  "otp.storage.label": "코드를 보관할 위치",
  "otp.storage.item": "이 항목과 함께",
  "otp.storage.itemTradeoff":
    "편리하지만 이 보관함을 여는 사람은 누구나 비밀번호와 두 번째 인증 요소를 함께 얻게 돼요.",
  "otp.storage.hardware": "{vault}에 (보안 키 필요)",
  "otp.storage.hardwareClosed": "먼저 보안 키로 {vault}을(를) 여세요.",
  "otp.toast.clipboardCleared": "스크린샷을 읽었어요. Mosaic이 클립보드를 지웠어요.",
  "otp.toast.savedInVault": "코드를 {vault}에 저장했어요.",
  "otp.clipboardCopies":
    "{platform, select, windows {클립보드 기록이나 캡처 도구에 스크린샷 사본이 남아 있으면 그곳에서 삭제하세요.} mac {스크린샷을 파일로도 저장했다면 삭제하세요.} other {클립보드 관리자에 스크린샷 사본이 남아 있으면 그곳에서 삭제하세요.}}",
} satisfies Record<keyof typeof en, string>;
