/**
 * Simplified Chinese UI strings: breach. Same keys as English (checked by the type).
 * ICU plural categories of Simplified Chinese: other (CLDR; `other` is required).
 */
import type { breach as en } from "../en/breach.ts";

export const breach = {
  "breach.card.title": "泄露检查",
  "breach.card.off": "已关闭。除非你开启，否则 Mosaic 绝不会联系 Have I Been Pwned。",
  "breach.card.about": "对照已知数据泄露事件检查你的密码。每个密码只有加扰代码中的 5 个字符会离开此设备。",
  "breach.mode.label": "对照已知数据泄露事件检查",
  "breach.mode.off": "关闭",
  "breach.mode.manual": "仅在我选择时",
  "breach.mode.weekly": "每周一次，解锁后进行",
  "breach.card.checkNow": "立即检查",
  "breach.card.stop": "停止检查",
  "breach.card.turnOff": "关闭",
  "breach.progress.starting": "正在检查你的密码…",
  "breach.progress.step": "正在检查… {done}/{total}",
  "breach.result.never": "尚未检查。",
  "breach.result.last": "上次检查：{date, date, medium}。{found, plural, =0 {未发现密码出现在泄露事件中。} other {发现 # 个密码出现在泄露事件中。}}",
  "breach.result.finished": "已检查 {checked, plural, other {# 个密码}}。{found, plural, =0 {未发现出现在泄露事件中的密码。} other {其中 # 个出现在泄露事件中。}}",
  "breach.result.upToDate": "所有密码最近都检查过了，没有需要查询的新内容。",
  "breach.error.offline": "无法连接到 Have I Been Pwned。下方显示的是上次的结果。请在联网后重试。",
  "breach.error.rateLimited": "Have I Been Pwned 要求 Mosaic 放慢速度。请几分钟后重试。",
  "breach.error.protocol": "Have I Been Pwned 返回了 Mosaic 不信任的响应，因此部分密码未被检查。请稍后重试。",
  "breach.error.cancelled": "检查已停止。已完成的结果会保留。",
  "breach.consent.title": "对照已知数据泄露事件检查你的密码？",
  "breach.consent.body": "Mosaic 会把每个密码转换成无法还原的加扰代码（哈希），并且只把该代码的前 5 个字符发送给 Have I Been Pwned（api.pwnedpasswords.com）。该服务会获知你的 IP 地址和这些 5 字符片段，但看不到密码、你的邮箱地址，也不知道密码属于哪个网站。结果只保存在此设备上。",
  "breach.consent.detail": "许多其他密码也有相同的 5 个字符片段，因此返回的结果不会暴露你的密码。不过这仍是向另一项服务发出的请求，所以在你同意之前它会保持关闭。你可以随时关闭。",
  "breach.consent.agree": "开启并立即检查",
  "breach.consent.decline": "暂不",
} satisfies Record<keyof typeof en, string>;
