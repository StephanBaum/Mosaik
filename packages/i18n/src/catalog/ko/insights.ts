/**
 * Korean UI strings: insights. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { insights as en } from "../en/insights.ts";

export const insights = {
  "insights.title": "보안",
  "insights.intro":
    "확인은 이 기기에서 실행돼요. 마지막 확인: {checkedAt, date, medium} {checkedAt, time, short}, {count, plural, other {항목 #개}} 대상.",
  "insights.checkAgain": "다시 확인",
  "insights.summary": "요약",
  "insights.fixFirst": "먼저 해결하세요",
  "insights.showAll": "{count, plural, other {결과 #개}} 모두 보기",
  "insights.empty": "문제가 없어요. 이 페이지를 열 때마다 Mosaic이 다시 확인해요.",
  "insights.kind.weak": "취약",
  "insights.kind.weak.description": "추측하기 쉬움",
  "insights.kind.reused": "재사용",
  "insights.kind.reused.description": "{groups, plural, other {#개 그룹}}",
  "insights.kind.old": "오래됨 (1년 초과)",
  "insights.kind.old.description": "1년 넘게 변경하지 않음",
  "insights.kind.missingTotp": "2단계 인증 없음",
  "insights.kind.missingTotp.description": "사이트가 지원하지만 설정하지 않았어요",
  "insights.kind.insecureUrl": "보안되지 않은 HTTP",
  "insights.kind.insecureUrl.description": "비밀번호를 암호화하지 않고 전송해요",
  "insights.kind.breached": "유출됨",
  "insights.kind.breached.description": "직접 켠 유출 확인에서 발견됨",
  "insights.finding.breached":
    "이 비밀번호가 알려진 데이터 유출에서 발견되었어요. 사용한 모든 곳에서 지금 변경하세요.",
  "insights.finding.weak": "추측하기 쉬워요. 생성된 비밀번호로 변경하세요.",
  "insights.finding.reused":
    "{count, plural, other {다른 항목 #개}}에서도 사용 중이에요. 사이트마다 서로 다른 비밀번호를 사용하세요.",
  "insights.finding.old": "1년 넘게 변경하지 않았어요. 아직 이 계정을 사용한다면 변경하세요.",
  "insights.finding.missingTotp":
    "{site}에서 2단계 인증 코드를 제공해요. 그곳에서 켠 다음 코드를 Mosaic에 저장하세요.",
  "insights.finding.insecureUrl":
    "일반 HTTP를 사용해요: {url}. 사이트가 지원한다면 https://로 바꾸세요.",
  "insights.severity.high": "높음",
  "insights.severity.medium": "보통",
  "insights.severity.low": "낮음",
  "insights.action.changePassword": "비밀번호 변경",
  "insights.action.setUpTotp": "2단계 인증 설정",
  "insights.action.reviewUrl": "검토",
  "insights.attribution.twoFactor": "2단계 인증 데이터: 2FA Directory (2fa.directory), MIT 라이선스.",
  "insights.counts": "{count, plural, =0 {문제 없음} other {문제 #개}}",
  "insights.documentation": "켜는 방법: {url}",
  "insights.actionFor": "{action}: {title}",
} satisfies Record<keyof typeof en, string>;
