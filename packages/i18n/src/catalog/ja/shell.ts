/**
 * Japanese UI strings: shell. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { shell as en } from "../en/shell.ts";

export const shell = {
  "shell.emergencyKit.title": "Mosaic 緊急キット",
  "shell.emergencyKit.intro":
    "このページは、鍵のかかる引き出しなど安全な場所に保管してください。新しいデバイスで保管庫のロックを解除するには、シークレットキーとマスターパスワードの両方が必要です。どちらも、Mosaic を含め誰にも復元できません。",
  "shell.emergencyKit.email": "アカウントのメールアドレス",
  "shell.emergencyKit.accountId": "アカウント ID",
  "shell.emergencyKit.signInUrl": "サインイン用アドレス",
  "shell.emergencyKit.signInUrlLocal": "このデバイスのみ（同期サーバーは未設定）",
  "shell.emergencyKit.secretKey": "シークレットキー",
  "shell.emergencyKit.masterPassword": "マスターパスワード",
  "shell.emergencyKit.masterPasswordHint":
    "このページを安全な場所に保管できる場合にのみ、線の上に手書きしてください。そうでなければ、記憶にとどめてください。",
  "shell.emergencyKit.footer":
    "Mosaic は監査前のソフトウェアです。外部のセキュリティ監査機関によるレビューはまだ受けていません。",
  "shell.shortcut.conflict":
    "{shortcut} は別のアプリですでに使われています。そのアプリを終了するかショートカットを変更してから Mosaic を再起動すると、クイック検索を使えます。",
  "keys.ctrl": "Ctrl",
  "keys.shift": "Shift",
  "keys.alt": "Alt",
  "keys.space": "Space",
  "keys.esc": "Esc",
  "keys.enter": "Enter",
} satisfies Record<keyof typeof en, string>;
