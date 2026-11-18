/**
 * Simplified Chinese UI strings: shell. Same keys as English (checked by the type).
 * ICU plural categories of Simplified Chinese: other (CLDR; `other` is required).
 */
import type { shell as en } from "../en/shell.ts";

export const shell = {
  "shell.emergencyKit.title": "Mosaic 紧急工具包",
  "shell.emergencyKit.intro": "请把此页面存放在安全的地方，比如上锁的抽屉里。你的机密密钥和主密码合在一起，才能在新设备上解锁你的保险库。任何人，包括 Mosaic，都无法为你找回其中任何一个。",
  "shell.emergencyKit.email": "账户邮箱",
  "shell.emergencyKit.accountId": "账户 ID",
  "shell.emergencyKit.signInUrl": "登录地址",
  "shell.emergencyKit.signInUrlLocal": "仅限此设备（尚未设置同步服务器）",
  "shell.emergencyKit.secretKey": "机密密钥",
  "shell.emergencyKit.masterPassword": "主密码",
  "shell.emergencyKit.masterPasswordHint": "只有当此页面会一直放在安全的地方时，才建议手写在横线上。否则请只记在脑子里。",
  "shell.emergencyKit.footer": "Mosaic 是审计前软件：尚未经过外部安全审计人员的审查。",
  "shell.shortcut.conflict": "另一个应用已在使用 {shortcut}。请关闭该应用或更改它的快捷键，然后重启 Mosaic 以使用快速搜索。",
  "keys.ctrl": "Ctrl",
  "keys.shift": "Shift",
  "keys.alt": "Alt",
  "keys.space": "空格",
  "keys.esc": "Esc",
  "keys.enter": "Enter",
} satisfies Record<keyof typeof en, string>;
