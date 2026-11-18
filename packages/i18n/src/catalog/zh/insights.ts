/**
 * Simplified Chinese UI strings: insights. Same keys as English (checked by the type).
 * ICU plural categories of Simplified Chinese: other (CLDR; `other` is required).
 */
import type { insights as en } from "../en/insights.ts";

export const insights = {
  "insights.title": "安全",
  "insights.intro": "检查在此设备上运行。上次检查时间：{checkedAt, date, medium} {checkedAt, time, short}，共 {count, plural, other {# 个项目}}。",
  "insights.checkAgain": "重新检查",
  "insights.summary": "摘要",
  "insights.fixFirst": "请先处理这些",
  "insights.showAll": "显示全部 {count, plural, other {# 项发现}}",
  "insights.empty": "没有发现问题。每次打开此页面时，Mosaic 都会重新检查。",
  "insights.kind.weak": "弱密码",
  "insights.kind.weak.description": "容易被猜到",
  "insights.kind.reused": "重复使用",
  "insights.kind.reused.description": "{groups, plural, other {# 组}}",
  "insights.kind.old": "过旧（超过 1 年）",
  "insights.kind.old.description": "超过一年没有更改",
  "insights.kind.missingTotp": "缺少两步验证",
  "insights.kind.missingTotp.description": "该网站支持，但你还没有设置",
  "insights.kind.insecureUrl": "不安全的 HTTP",
  "insights.kind.insecureUrl.description": "会以未加密方式发送你的密码",
  "insights.kind.breached": "已泄露",
  "insights.kind.breached.description": "由你主动开启的泄露检查发现",
  "insights.finding.breached": "此密码出现在已知的数据泄露事件中。请立即更改，并在所有用到它的地方更换。",
  "insights.finding.weak": "容易被猜到。请改用生成的密码。",
  "insights.finding.reused": "另外还有 {count, plural, other {# 个项目}}也在使用它。请为每个网站设置单独的密码。",
  "insights.finding.old": "超过一年没有更改。如果你还在使用此账户，请更改密码。",
  "insights.finding.missingTotp": "{site} 提供两步验证码。请在该网站开启，并把验证码保存到 Mosaic 中。",
  "insights.finding.insecureUrl": "使用的是普通 HTTP：{url}。如果网站支持，请改用 https://。",
  "insights.severity.high": "高",
  "insights.severity.medium": "中",
  "insights.severity.low": "低",
  "insights.action.changePassword": "更改密码",
  "insights.action.setUpTotp": "设置两步验证",
  "insights.action.reviewUrl": "查看",
  "insights.attribution.twoFactor": "两步验证数据：2FA Directory (2fa.directory)，MIT 许可证。",
  "insights.counts": "{count, plural, =0 {没有问题} other {# 个问题}}",
  "insights.documentation": "开启方法：{url}",
  "insights.actionFor": "{action}：{title}",
} satisfies Record<keyof typeof en, string>;
