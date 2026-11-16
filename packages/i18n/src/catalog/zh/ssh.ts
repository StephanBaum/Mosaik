/**
 * Simplified Chinese UI strings: ssh. Same keys as English (checked by the type).
 * ICU plural categories of Simplified Chinese: other (CLDR; `other` is required).
 */
import type { ssh as en } from "../en/ssh.ts";

export const ssh = {
  "ssh.title": "有应用想使用你的 SSH 密钥",
  "ssh.key": "密钥",
  "ssh.fingerprint": "指纹",
  "ssh.client": "请求来源",
  "ssh.client.unknown": "未知程序",
  "ssh.expires": "{seconds, plural, other {# 秒}}后自动拒绝",
  "ssh.deny": "拒绝",
  "ssh.allowOnce": "允许一次",
  "ssh.allowFor15Minutes": "允许 15 分钟",
  "ssh.queue": "{count, plural, other {还有 # 个请求在等待}}",
  "ssh.signFailed.passphraseRequired": "该密钥已加密，但此项目没有口令。请在项目的“密码”字段中添加。",
  "ssh.signFailed.wrongPassphrase": "项目中的口令无法解锁该密钥。请在项目的“密码”字段中更正。",
  "ssh.signFailed.keyMalformed": "无法读取此项目中的私钥。请以 OpenSSH 格式重新粘贴。",
  "ssh.signFailed.kdfTooExpensive": "该密钥的口令保护所用轮数超出 Mosaic 允许的范围。请用 \"ssh-keygen -p -a 64\" 重新加密。",
  "ssh.signFailed.publicKeyMismatch": "私钥与项目的公钥不匹配。请检查这两个字段。",
  "ssh.signFailed.unsupported": "Mosaic 仅支持使用 Ed25519 和 ECDSA P-256 密钥签名。",
  "ssh.signFailed.internal": "签名失败。请重试。",
  "ssh.skip.unsupported": "未提供：Mosaic 仅支持使用 Ed25519 和 ECDSA P-256 密钥签名。",
  "ssh.skip.malformed": "未提供：无法读取该密钥。",
  "ssh.skip.duplicate": "未提供：另一个项目含有相同的密钥。",
  "ssh.description": "有应用请求 Mosaic SSH 代理使用此密钥登录。",
  "ssh.origin.forwarded": "你所连接的服务器请求使用此密钥。",
  "ssh.origin.bindFailed": "Mosaic 无法确定此请求来自哪里：连接的主机校验失败。仅在你预期会有此请求时才允许。",
  "ssh.escDenies": "拒绝",
} satisfies Record<keyof typeof en, string>;
