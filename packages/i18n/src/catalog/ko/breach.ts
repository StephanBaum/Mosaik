/**
 * Korean UI strings: breach. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { breach as en } from "../en/breach.ts";

export const breach = {
  "breach.card.title": "유출 확인",
  "breach.card.off": "꺼져 있어요. 직접 켜지 않으면 Mosaic은 Have I Been Pwned에 연결하지 않아요.",
  "breach.card.about":
    "비밀번호가 알려진 데이터 유출에 포함되었는지 확인해요. 비밀번호마다 변환된 코드의 앞 5자만 이 기기를 떠나요.",
  "breach.mode.label": "알려진 데이터 유출과 대조",
  "breach.mode.off": "끔",
  "breach.mode.manual": "직접 선택할 때만",
  "breach.mode.weekly": "잠금 해제 후 매주",
  "breach.card.checkNow": "지금 확인",
  "breach.card.stop": "확인 중지",
  "breach.card.turnOff": "끄기",
  "breach.progress.starting": "비밀번호를 확인하는 중…",
  "breach.progress.step": "확인 중… {total}개 중 {done}개",
  "breach.result.never": "아직 확인하지 않았어요.",
  "breach.result.last":
    "마지막 확인: {date, date, medium}. {found, plural, =0 {유출에서 발견된 비밀번호가 없어요.} other {유출에서 발견된 비밀번호가 #개 있어요.}}",
  "breach.result.finished":
    "{checked, plural, other {비밀번호 #개}}를 확인했어요. {found, plural, =0 {유출에서 발견된 것은 없어요.} other {#개가 유출에서 발견되었어요.}}",
  "breach.result.upToDate": "모두 최근에 확인했어요. 새로 조회할 항목이 없어요.",
  "breach.error.offline":
    "Have I Been Pwned에 연결할 수 없어요. 지난 결과를 표시하고 있어요. 온라인이 되면 다시 시도하세요.",
  "breach.error.rateLimited":
    "Have I Been Pwned가 Mosaic에 속도를 줄여 달라고 요청했어요. 몇 분 후에 다시 시도하세요.",
  "breach.error.protocol":
    "Have I Been Pwned가 Mosaic이 신뢰할 수 없는 응답을 보내서 일부 비밀번호를 확인하지 못했어요. 나중에 다시 시도하세요.",
  "breach.error.cancelled": "확인이 중지되었어요. 이미 끝난 결과는 유지돼요.",
  "breach.consent.title": "비밀번호를 알려진 데이터 유출과 대조할까요?",
  "breach.consent.body":
    "Mosaic은 각 비밀번호를 되돌릴 수 없는 코드(해시)로 변환하고, 그 코드의 앞 5자만 Have I Been Pwned(api.pwnedpasswords.com)로 보내요. 이 서비스는 내 IP 주소와 5자 조각만 알 수 있어요. 비밀번호, 이메일 주소, 비밀번호가 어느 사이트용인지는 절대 볼 수 없어요. 결과는 이 기기에만 남아요.",
  "breach.consent.detail":
    "5자 조각 하나는 다른 많은 비밀번호와도 같기 때문에 응답으로 내 비밀번호를 알 수 없어요. 그래도 다른 서비스로 보내는 요청이므로 동의하기 전에는 꺼져 있어요. 언제든 끌 수 있어요.",
  "breach.consent.agree": "켜고 지금 확인",
  "breach.consent.decline": "나중에",
} satisfies Record<keyof typeof en, string>;
