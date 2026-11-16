/**
 * Korean UI strings: shell. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { shell as en } from "../en/shell.ts";

export const shell = {
  "shell.emergencyKit.title": "Mosaic 비상 키트",
  "shell.emergencyKit.intro":
    "이 페이지를 잠금장치가 있는 서랍처럼 안전한 곳에 보관하세요. 비밀 키와 마스터 비밀번호가 함께 있어야 새 기기에서 보관함을 열 수 있어요. 누구도, Mosaic도 둘 중 어느 것도 대신 복구해 줄 수 없어요.",
  "shell.emergencyKit.email": "계정 이메일",
  "shell.emergencyKit.accountId": "계정 ID",
  "shell.emergencyKit.signInUrl": "로그인 주소",
  "shell.emergencyKit.signInUrlLocal": "이 기기에서만 사용 (아직 동기화 서버가 설정되지 않음)",
  "shell.emergencyKit.secretKey": "비밀 키",
  "shell.emergencyKit.masterPassword": "마스터 비밀번호",
  "shell.emergencyKit.masterPasswordHint":
    "이 페이지를 안전한 곳에 보관할 때만 줄에 직접 적으세요. 그렇지 않으면 외워 두세요.",
  "shell.emergencyKit.footer":
    "Mosaic은 감사 전 소프트웨어예요. 외부 보안 감사자의 검토를 아직 받지 않았어요.",
  "shell.shortcut.conflict":
    "다른 앱이 이미 {shortcut}을(를) 사용하고 있어요. 그 앱을 닫거나 단축키를 바꾼 다음 Mosaic을 다시 시작해야 빠른 검색을 사용할 수 있어요.",
  "keys.ctrl": "Ctrl",
  "keys.shift": "Shift",
  "keys.alt": "Alt",
  "keys.space": "Space",
  "keys.esc": "Esc",
  "keys.enter": "Enter",
} satisfies Record<keyof typeof en, string>;
