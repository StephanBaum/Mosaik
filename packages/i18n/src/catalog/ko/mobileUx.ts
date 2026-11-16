/**
 * Korean UI strings: mobileUx. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { mobileUx as en } from "../en/mobileUx.ts";

export const mobileUx = {
  "mobile.sync.warningsBanner": "동기화에서 문제를 발견했어요. 설정을 열어 확인하세요.",
  "mobile.provider.errorTitle": "Mosaic이 계속할 수 없어요",
  "mobile.onboarding.details.keyWiped":
    "앱을 나가면서 Mosaic이 스캔한 비밀 키를 제거했어요. 코드를 다시 스캔하거나 키를 직접 입력하세요.",
  "mobile.common.showField": "{field} 표시",
  "mobile.common.hideField": "{field} 숨기기",
  "mobile.error.storeInUse":
    "이 휴대폰의 다른 곳에서 Mosaic이 사용 중이에요. 잠시 기다린 다음 다시 시도하세요.",
  "mobile.error.noAccount": "이 휴대폰에는 아직 Mosaic이 설정되어 있지 않아요. Mosaic을 열어 설정하세요.",
  "mobile.error.wrongCredentials":
    "마스터 비밀번호 또는 비밀 키가 올바르지 않아요. 오타와 키보드 언어를 확인한 다음 다시 시도하세요.",
  "mobile.error.invalidInput":
    "입력한 내용 중 올바르지 않은 것이 있어요. 입력 내용을 확인하고 다시 시도하세요.",
  "mobile.error.vaultUnavailable":
    "지금은 이 보관함을 열 수 없어요. 동기화한 다음 다시 시도하거나 Mosaic을 다시 잠금 해제하세요.",
  "mobile.unlock.quickOptionTouchId": "다음부터 Touch ID로 잠금 해제",
  "mobile.detail.readCode": "코드 읽기",
  "mobile.detail.labelValue": "{label}: {value}",
  "mobile.autofill.warningWithName": "먼저 주소를 확인하세요 | {name}",
  "mobile.autofill.linkEntry": "{login} 연결하고 자동 입력",
  "mobile.link.imported": "이 앱용으로 가져옴",
  "mobile.link.confirmTitle": "{login}을(를) {app}에 연결할까요?",
  "mobile.link.confirmSubtitle": "앱 ID {appId} · 서명 {fingerprint}",
  "mobile.link.confirmFallbackHint": "연결한 후 입력란을 다시 탭하면 자동으로 입력돼요.",
  "mobile.link.importedNotVerified": "가져온 로그인이에요. 웹사이트가 이 앱을 확인하지 않았어요.",
} satisfies Record<keyof typeof en, string>;
