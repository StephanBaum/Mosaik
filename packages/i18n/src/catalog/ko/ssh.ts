/**
 * Korean UI strings: ssh. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { ssh as en } from "../en/ssh.ts";

export const ssh = {
  "ssh.title": "앱이 SSH 키를 사용하려고 해요",
  "ssh.key": "키",
  "ssh.fingerprint": "지문",
  "ssh.client": "요청한 곳",
  "ssh.client.unknown": "알 수 없는 프로그램",
  "ssh.expires": "{seconds, plural, other {#초}} 후 자동으로 거부돼요",
  "ssh.deny": "거부",
  "ssh.allowOnce": "한 번 허용",
  "ssh.allowFor15Minutes": "15분 동안 허용",
  "ssh.queue": "{count, plural, other {대기 중인 요청 #개 더 있음}}",
  "ssh.signFailed.passphraseRequired":
    "키가 암호화되어 있지만 항목에 암호 문구가 없어요. 항목의 비밀번호 필드에 추가하세요.",
  "ssh.signFailed.wrongPassphrase":
    "항목의 암호 문구로 키를 열 수 없어요. 항목의 비밀번호 필드에서 바로잡으세요.",
  "ssh.signFailed.keyMalformed":
    "이 항목의 개인 키를 읽을 수 없어요. OpenSSH 형식으로 다시 붙여 넣으세요.",
  "ssh.signFailed.kdfTooExpensive":
    "키의 암호 문구 보호가 Mosaic이 허용하는 것보다 많은 반복을 사용해요. “ssh-keygen -p -a 64”로 다시 암호화하세요.",
  "ssh.signFailed.publicKeyMismatch":
    "개인 키가 항목의 공개 키와 일치하지 않아요. 두 필드를 확인하세요.",
  "ssh.signFailed.unsupported": "Mosaic은 Ed25519와 ECDSA P-256 키로만 서명해요.",
  "ssh.signFailed.internal": "서명에 실패했어요. 다시 시도하세요.",
  "ssh.skip.unsupported": "제공 안 함: Mosaic은 Ed25519와 ECDSA P-256 키로만 서명해요.",
  "ssh.skip.malformed": "제공 안 함: 키를 읽을 수 없어요.",
  "ssh.skip.duplicate": "제공 안 함: 다른 항목에 같은 키가 있어요.",
  "ssh.description": "앱이 이 키로 로그인하도록 Mosaic SSH 에이전트에 서명을 요청했어요.",
  "ssh.origin.forwarded": "연결 중인 서버가 이 키를 사용하려고 해요.",
  "ssh.origin.bindFailed":
    "Mosaic이 이 요청이 어디에서 왔는지 알 수 없어요. 연결의 호스트 확인에 실패했어요. 예상한 요청일 때만 허용하세요.",
  "ssh.escDenies": "거부하려면",
} satisfies Record<keyof typeof en, string>;
