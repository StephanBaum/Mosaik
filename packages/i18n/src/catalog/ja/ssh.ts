/**
 * Japanese UI strings: ssh. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { ssh as en } from "../en/ssh.ts";

export const ssh = {
  "ssh.title": "アプリが SSH キーを使用しようとしています",
  "ssh.key": "キー",
  "ssh.fingerprint": "フィンガープリント",
  "ssh.client": "要求元",
  "ssh.client.unknown": "不明なプログラム",
  "ssh.expires": "{seconds, plural, other {#秒}}後に自動的に拒否されます",
  "ssh.deny": "拒否",
  "ssh.allowOnce": "今回のみ許可",
  "ssh.allowFor15Minutes": "15分間許可",
  "ssh.queue": "{count, plural, other {他に#件のリクエストが待機中です}}",
  "ssh.signFailed.passphraseRequired":
    "キーは暗号化されていますが、項目にパスフレーズがありません。項目の「パスワード」欄に入力してください。",
  "ssh.signFailed.wrongPassphrase":
    "項目のパスフレーズではキーのロックを解除できません。項目の「パスワード」欄を修正してください。",
  "ssh.signFailed.keyMalformed":
    "この項目の秘密鍵を読み取れません。OpenSSH 形式でもう一度貼り付けてください。",
  "ssh.signFailed.kdfTooExpensive":
    "キーのパスフレーズ保護が、Mosaic の許容を超える反復回数を使用しています。「ssh-keygen -p -a 64」で再暗号化してください。",
  "ssh.signFailed.publicKeyMismatch":
    "秘密鍵が項目の公開鍵と一致しません。両方の欄を確認してください。",
  "ssh.signFailed.unsupported": "Mosaic が署名できるのは Ed25519 と ECDSA P-256 のキーのみです。",
  "ssh.signFailed.internal": "署名に失敗しました。もう一度お試しください。",
  "ssh.skip.unsupported": "提供されません: Mosaic が署名できるのは Ed25519 と ECDSA P-256 のキーのみです。",
  "ssh.skip.malformed": "提供されません: キーを読み取れません。",
  "ssh.skip.duplicate": "提供されません: 別の項目に同じキーがあります。",
  "ssh.description": "アプリが、このキーでサインインするよう Mosaic の SSH エージェントに要求しました。",
  "ssh.origin.forwarded": "接続中のサーバーが、このキーの使用を要求しています。",
  "ssh.origin.bindFailed":
    "このリクエストの送信元を Mosaic が判断できません。接続のホスト確認に失敗しました。心当たりがある場合にのみ許可してください。",
  "ssh.escDenies": "で拒否",
} satisfies Record<keyof typeof en, string>;
