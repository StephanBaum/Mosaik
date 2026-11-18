/**
 * Korean UI strings: unlock. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { unlock as en } from "../en/unlock.ts";

export const unlock = {
  "lock.unlockTitle": "Mosaic 잠금 해제",
  "lock.masterPassword": "마스터 비밀번호",
  "lock.wrongPassword":
    "마스터 비밀번호가 올바르지 않아요. Caps Lock과 키보드 언어를 확인한 다음 다시 시도하세요.",
  "lock.locked": "Mosaic이 잠겨 있어요.",
  "session.unlocked": "Mosaic이 잠금 해제되었어요.",
  "lock.secretKey": "비밀 키",
  "lock.secretKeyHint":
    "이 기기에는 아직 비밀 키가 없어요. 비상 키트에서 입력하면 Mosaic이 이 기기에 기억해요.",
  "lock.submit": "잠금 해제",
  "lock.working": "잠금 해제 중…",
  "lock.reason.manual": "직접 Mosaic을 잠갔어요.",
  "lock.reason.idle": "한동안 사용하지 않아 Mosaic이 자동으로 잠겼어요.",
  "lock.reason.sleep": "컴퓨터가 절전 모드로 들어가서 Mosaic이 잠겼어요.",
  "lock.reason.screenLock": "화면이 잠겨서 Mosaic도 함께 잠겼어요.",
  "lock.or": "또는",
  "lock.quick.biometricWindows": "Windows Hello 사용",
  "lock.quick.biometricMac": "Touch ID 사용",
  "lock.quick.biometricLinux": "생체 인식 사용",
  "lock.quick.hardwareKey": "보안 키 사용",
  "lock.autoLockInfo": "사용하지 않으면 {minutes, plural, other {#분}} 후 자동으로 잠겨요.",
  "lock.forgot.title": "마스터 비밀번호를 잊으셨나요?",
  "lock.forgot.body":
    "마스터 비밀번호는 본인만 알기 때문에 Mosaic이 재설정할 수 없어요. 비상 키트에 적어 두었는지 확인하세요. 그렇지 않다면 새 계정을 만드세요.",
  "lock.quick.waitingWindows": "Windows Hello를 기다리는 중…",
  "lock.quick.waitingMac": "Touch ID를 기다리는 중…",
  "lock.quick.touchKey": "키를 꽂고 깜박이면 터치하세요.",
  "lock.quick.pin": "보안 키 PIN",
  "lock.quick.pinSubmit": "계속",
  "lock.quick.cancel": "취소",
} satisfies Record<keyof typeof en, string>;
