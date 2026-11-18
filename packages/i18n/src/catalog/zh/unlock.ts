/**
 * Simplified Chinese UI strings: unlock. Same keys as English (checked by the type).
 * ICU plural categories of Simplified Chinese: other (CLDR; `other` is required).
 */
import type { unlock as en } from "../en/unlock.ts";

export const unlock = {
  "lock.unlockTitle": "解锁 Mosaic",
  "lock.masterPassword": "主密码",
  "lock.wrongPassword": "主密码不正确。请检查大写锁定和键盘布局，然后重试。",
  "lock.locked": "Mosaic 已锁定。",
  "session.unlocked": "Mosaic 已解锁。",
  "lock.secretKey": "机密密钥",
  "lock.secretKeyHint": "这台设备上还没有你的机密密钥。请从紧急工具包中输入，Mosaic 会在此设备上记住它。",
  "lock.submit": "解锁",
  "lock.working": "正在解锁…",
  "lock.reason.manual": "你已锁定 Mosaic。",
  "lock.reason.idle": "一段时间没有操作，Mosaic 已自动锁定。",
  "lock.reason.sleep": "电脑进入睡眠时，Mosaic 已锁定。",
  "lock.reason.screenLock": "Mosaic 已随屏幕一同锁定。",
  "lock.or": "或",
  "lock.quick.biometricWindows": "使用 Windows Hello",
  "lock.quick.biometricMac": "使用触控 ID",
  "lock.quick.biometricLinux": "使用生物识别",
  "lock.quick.hardwareKey": "使用安全密钥",
  "lock.autoLockInfo": "闲置 {minutes, plural, other {# 分钟}}后自动锁定。",
  "lock.forgot.title": "忘记主密码了？",
  "lock.forgot.body": "Mosaic 无法重置主密码，因为只有你知道它。请看看是否把它写在了紧急工具包上。否则，请创建新账户。",
  "lock.quick.waitingWindows": "正在等待 Windows Hello…",
  "lock.quick.waitingMac": "正在等待触控 ID…",
  "lock.quick.touchKey": "请插入你的密钥，并在它闪烁时触碰。",
  "lock.quick.pin": "安全密钥 PIN",
  "lock.quick.pinSubmit": "继续",
  "lock.quick.cancel": "取消",
} satisfies Record<keyof typeof en, string>;
