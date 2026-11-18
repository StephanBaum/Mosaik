/**
 * Simplified Chinese UI strings: palette. Same keys as English (checked by the type).
 * ICU plural categories of Simplified Chinese: other (CLDR; `other` is required).
 */
import type { palette as en } from "../en/palette.ts";

export const palette = {
  "palette.title": "命令面板",
  "palette.search": "搜索项目和命令",
  "palette.items": "项目",
  "palette.commands": "命令",
  "palette.empty": "没有匹配的结果。换个词试试。",
  "palette.cmd.newItem": "新建项目",
  "palette.cmd.generator": "打开生成器",
  "palette.cmd.security": "打开安全报告",
  "palette.cmd.settings": "打开设置",
  "palette.cmd.import": "从其他应用导入",
  "palette.cmd.share": "共享保险库…",
  "palette.cmd.invites": "邀请",
  "palette.cmd.lock": "锁定 Mosaic",
  "palette.open": "打开命令面板",
  "palette.hint.move": "移动",
  "palette.hint.open": "打开",
  "palette.hint.close": "关闭",
} satisfies Record<keyof typeof en, string>;
