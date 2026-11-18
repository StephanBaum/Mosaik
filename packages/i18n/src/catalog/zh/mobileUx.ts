/**
 * Simplified Chinese UI strings: mobileUx. Same keys as English (checked by the type).
 * ICU plural categories of Simplified Chinese: other (CLDR; `other` is required).
 */
import type { mobileUx as en } from "../en/mobileUx.ts";

export const mobileUx = {
  "mobile.sync.warningsBanner": "同步时发现问题。请打开“设置”查看。",
  "mobile.provider.errorTitle": "Mosaic 无法继续",
  "mobile.onboarding.details.keyWiped": "你离开应用时，Mosaic 已清除扫描到的机密密钥。请重新扫描二维码，或手动输入密钥。",
  "mobile.common.showField": "显示{field}",
  "mobile.common.hideField": "隐藏{field}",
  "mobile.error.storeInUse": "Mosaic 正在这部手机的其他位置运行。请稍等片刻，然后重试。",
  "mobile.error.noAccount": "这部手机上还没有设置 Mosaic。请打开 Mosaic 进行设置。",
  "mobile.error.wrongCredentials": "主密码或机密密钥不正确。请检查是否输错，以及键盘语言，然后重试。",
  "mobile.error.invalidInput": "你输入的内容有误。请检查后重试。",
  "mobile.error.vaultUnavailable": "目前无法打开此保险库。请同步后重试，或重新解锁 Mosaic。",
  "mobile.unlock.quickOptionTouchId": "下次使用触控 ID 解锁",
  "mobile.detail.readCode": "读取验证码",
  "mobile.detail.labelValue": "{label}：{value}",
  "mobile.autofill.warningWithName": "请先核对网址 | {name}",
  "mobile.autofill.linkEntry": "关联 {login} 并填充",
  "mobile.link.imported": "已为此应用导入",
  "mobile.link.confirmTitle": "将 {login} 关联到 {app}？",
  "mobile.link.confirmSubtitle": "应用 ID {appId} · 签名 {fingerprint}",
  "mobile.link.confirmFallbackHint": "关联后，请再次点按该输入框进行填充。",
  "mobile.link.importedNotVerified": "已导入的登录信息。网站尚未确认此应用。",
} satisfies Record<keyof typeof en, string>;
