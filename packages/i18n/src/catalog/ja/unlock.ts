/**
 * Japanese UI strings: unlock. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { unlock as en } from "../en/unlock.ts";

export const unlock = {
  "lock.unlockTitle": "Mosaic のロックを解除",
  "lock.masterPassword": "マスターパスワード",
  "lock.wrongPassword":
    "マスターパスワードが正しくありません。Caps Lock とキーボードの配列を確認して、もう一度お試しください。",
  "lock.locked": "Mosaic はロックされています。",
  "session.unlocked": "Mosaic のロックは解除されています。",
  "lock.secretKey": "シークレットキー",
  "lock.secretKeyHint":
    "このデバイスには、まだシークレットキーがありません。緊急キットを見ながら入力すると、Mosaic がこのデバイスに記憶します。",
  "lock.submit": "ロック解除",
  "lock.working": "ロックを解除しています…",
  "lock.reason.manual": "Mosaic を手動でロックしました。",
  "lock.reason.idle": "しばらく操作がなかったため、Mosaic が自動でロックされました。",
  "lock.reason.sleep": "コンピューターがスリープしたため、Mosaic がロックされました。",
  "lock.reason.screenLock": "画面のロックに合わせて Mosaic もロックされました。",
  "lock.or": "または",
  "lock.quick.biometricWindows": "Windows Hello を使用",
  "lock.quick.biometricMac": "Touch ID を使用",
  "lock.quick.biometricLinux": "生体認証を使用",
  "lock.quick.hardwareKey": "セキュリティキーを使用",
  "lock.autoLockInfo": "操作がない状態が{minutes, plural, other {#分}}続くと自動でロックされます。",
  "lock.forgot.title": "マスターパスワードをお忘れですか？",
  "lock.forgot.body":
    "マスターパスワードを知っているのはあなただけなので、Mosaic では再設定できません。緊急キットに書き留めていないか確認してください。見つからない場合は、新しいアカウントを作成してください。",
  "lock.quick.waitingWindows": "Windows Hello を待っています…",
  "lock.quick.waitingMac": "Touch ID を待っています…",
  "lock.quick.touchKey": "キーを挿入し、点滅したら触れてください。",
  "lock.quick.pin": "セキュリティキーの PIN",
  "lock.quick.pinSubmit": "続ける",
  "lock.quick.cancel": "キャンセル",
} satisfies Record<keyof typeof en, string>;
