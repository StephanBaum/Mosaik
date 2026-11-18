/**
 * Korean UI strings: credentialRotation. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { credentialRotation as en } from "../en/credentialRotation.ts";

export const credentialRotation = {
  "credentialRotation.tablist": "보안 보기",
  "credentialRotation.tab.overview": "개요",
  "credentialRotation.tab.rotate": "지금 변경",
  "credentialRotation.tab.dueHidden": ", 변경할 항목 {count}개",
  "credentialRotation.queue.title": "지금 변경",
  "credentialRotation.queue.intro":
    "먼저 바꿀 가치가 있는 비밀번호예요. Mosaic이 안내하지만 각 비밀번호는 사이트에서 직접 변경해요.",
  "credentialRotation.queue.empty": "지금 변경할 항목이 없어요.",
  "credentialRotation.queue.count": "{count, plural, other {변경할 비밀번호 #개}}",
  "credentialRotation.queue.list": "변경할 비밀번호",
  "credentialRotation.queue.snoozedHeading": "다시 알림 대기 중",
  "credentialRotation.queue.snoozedUntil": "{date, date, medium}까지 미룸",
  "credentialRotation.queue.reason.weak": "취약",
  "credentialRotation.queue.reason.reused": "재사용",
  "credentialRotation.queue.reason.old": "오래됨",
  "credentialRotation.queue.reason.breached": "유출됨",
  "credentialRotation.queue.reason.unfinished": "변경이 끝나지 않음",
  "credentialRotation.queue.change": "비밀번호 변경",
  "credentialRotation.queue.continue": "변경 계속",
  "credentialRotation.queue.editItem": "항목 편집",
  "credentialRotation.queue.manualHint":
    "Mosaic이 이 항목은 안내할 수 없어요. 항목을 열어 직접 변경하세요.",
  "credentialRotation.queue.actionFor": "{action}: {title}",
  "credentialRotation.queue.snooze": "미루기",
  "credentialRotation.queue.snoozeFor": "{title} 미루기",
  "credentialRotation.queue.snoozeDays": "{days, plural, other {#일}}",
  "credentialRotation.queue.snoozed": "{days, plural, other {#일}} 동안 미뤘어요.",
  "credentialRotation.queue.dismiss": "무시",
  "credentialRotation.queue.dismissFor": "{title} 무시",
  "credentialRotation.queue.dismissed": "무시했어요. 비밀번호가 바뀌면 다시 나타나요.",
  "credentialRotation.queue.undo": "실행 취소",
  "credentialRotation.panel.title": "{title}의 비밀번호 변경",
  "credentialRotation.panel.step": "{total}단계 중 {current}단계",
  "credentialRotation.panel.stays":
    "{title}의 변경은 열려 있어요. 새 비밀번호는 작동함 또는 작동 안 함을 선택할 때까지 항목에 보관돼요.",
  "credentialRotation.panel.cancel": "취소",
  "credentialRotation.steps.new.title": "새 비밀번호 선택",
  "credentialRotation.steps.new.intro":
    "Mosaic은 새 비밀번호를 어디에든 사용하기 전에 이 항목에 저장해서 잃어버리지 않게 해요. 현재 비밀번호는 확인할 때까지 그대로 유지돼요.",
  "credentialRotation.steps.new.label": "새 비밀번호",
  "credentialRotation.steps.new.use": "이 비밀번호 사용",
  "credentialRotation.steps.new.saving": "저장 중…",
  "credentialRotation.steps.site.title": "사이트에서 변경",
  "credentialRotation.steps.site.pageLabel": "변경 페이지",
  "credentialRotation.steps.site.body":
    "브라우저에서 사이트의 비밀번호 페이지를 열고 도구 모음의 Mosaic을 클릭한 다음 변경 양식 채우기를 선택하세요. Mosaic은 양식을 절대 제출하지 않아요.",
  "credentialRotation.steps.site.copyLink": "변경 페이지 링크 복사",
  "credentialRotation.steps.site.linkCopied": "링크를 복사했어요.",
  "credentialRotation.steps.site.linkHint":
    "링크는 비밀이 아니에요. 사이트에 그 주소의 페이지가 없으면 비밀번호 설정을 직접 찾으세요.",
  "credentialRotation.steps.site.noLink":
    "이 로그인에는 https 주소가 없어서 Mosaic이 제안할 변경 페이지가 없어요. 사이트에서 비밀번호를 직접 변경하세요.",
  "credentialRotation.steps.site.copyPassword": "새 비밀번호 복사",
  "credentialRotation.steps.site.next": "시도해 봤어요",
  "credentialRotation.steps.result.title": "사이트가 새 비밀번호를 받아들였나요?",
  "credentialRotation.steps.done.failed": "현재 비밀번호 유지됨",
  "credentialRotation.steps.done.notSure": "변경이 열려 있음",
  "credentialRotation.steps.result.body":
    "Mosaic은 사이트를 볼 수 없어요. 이 항목에 올바른 비밀번호가 남도록 결과를 알려 주세요. 어느 쪽이든 아무것도 삭제되지 않아요.",
  "credentialRotation.steps.result.working": "저장 중…",
  "credentialRotation.result.worked": "작동함",
  "credentialRotation.result.failed": "작동 안 함",
  "credentialRotation.result.notSure": "아직 모르겠어요",
  "credentialRotation.result.workedDone":
    "완료했어요. 새 비밀번호가 이제 이 항목의 비밀번호예요. 이전 비밀번호는 기록에 있어요.",
  "credentialRotation.result.failedDone":
    "현재 비밀번호를 유지해요. 사이트가 실제로 적용했을 수도 있으니 새 비밀번호는 확인되지 않음으로 기록에 보관돼요.",
  "credentialRotation.result.notSureDone":
    "이 항목의 변경은 열려 있어요. 결과를 알게 되면 돌아오세요.",
  "credentialRotation.banner.title": "{date, date, medium}에 비밀번호 변경을 시작했어요",
  "credentialRotation.banner.prepared":
    "새 비밀번호는 확인할 때까지 여기에 보관돼요. 아직 어떤 페이지에도 전달되지 않았어요.",
  "credentialRotation.banner.delivered":
    "새 비밀번호는 확인할 때까지 여기에 보관돼요. 브라우저 확장 프로그램이 사이트의 양식에 입력했어요.",
  "credentialRotation.banner.reveal": "새 비밀번호 표시",
  "credentialRotation.banner.hide": "새 비밀번호 숨기기",
  "credentialRotation.banner.copy": "새 비밀번호 복사",
  "credentialRotation.banner.newPassword": "새 비밀번호",
  "credentialRotation.banner.copyLink": "변경 페이지 링크 복사",
  "credentialRotation.history.title": "비밀번호 기록",
  "credentialRotation.history.list": "이전 비밀번호",
  "credentialRotation.history.entry": "{date, date, medium}에 교체됨",
  "credentialRotation.history.unconfirmed": "확인되지 않음",
  "credentialRotation.history.unconfirmedEntry": "{date, date, medium}에 시도함",
  "credentialRotation.history.unconfirmedHint":
    "사이트가 이 비밀번호를 적용했을 수도 있어요. 현재 비밀번호로 로그인되지 않으면 복원하세요.",
  "credentialRotation.history.reveal": "{date, date, medium}의 비밀번호 표시",
  "credentialRotation.history.hide": "{date, date, medium}의 비밀번호 숨기기",
  "credentialRotation.history.restore": "복원",
  "credentialRotation.history.restoreFor": "{date, date, medium}의 비밀번호 복원",
  "credentialRotation.history.restored": "복원했어요. 이전 비밀번호는 기록에 있어요.",
  "credentialRotation.policy.label": "이 비밀번호를 변경하도록 알림",
  "credentialRotation.policy.none": "안 함",
  "credentialRotation.policy.every": "{days}일마다",
  "credentialRotation.policy.help":
    "알림을 설정하면 항목이 지금 변경 목록에 들어가요. Mosaic은 알림을 보내거나 이를 위해 요청을 보내지 않아요.",
  "credentialRotation.url.help":
    "선택 사항이에요. 이 로그인과 같은 사이트에 있는 비밀번호 페이지의 https 주소예요. Mosaic은 브라우저 확장 프로그램에서 변경 페이지 열기를 선택할 때만 이 주소를 열어요.",
  "credentialRotation.url.invalid":
    "Mosaic이 이 주소를 사용하지 않아요. https://로 시작하고 이 로그인과 같은 사이트에 있어야 해요.",
  "credentialRotation.settings.title": "비밀번호 알림",
  "credentialRotation.settings.body":
    "Mosaic이 잠금 해제 후 변경할 가치가 있는 비밀번호가 몇 개인지 알려 줄 수 있어요.",
  "credentialRotation.settings.reminders": "잠금 해제 후 변경할 비밀번호 수 표시",
  "credentialRotation.settings.remindersHint":
    "앱 안의 메시지로만 표시돼요. 알림도, 네트워크 요청도 없어요.",
  "credentialRotation.reminder.toast": "{count, plural, other {변경할 비밀번호 #개}}",
  "credentialRotation.reminder.show": "보기",
  "credentialRotation.dialog.beginTitle": "비밀번호 변경을 시작할까요?",
  "credentialRotation.dialog.beginBody":
    "{browser}에서 {host}의 {title} 비밀번호를 변경하려고 해요. Mosaic은 새 비밀번호를 이 로그인과 함께 보관하고, 사이트가 받아들였다고 확인할 때까지 현재 비밀번호를 유지해요.",
  "credentialRotation.dialog.site": "웹사이트",
  "credentialRotation.dialog.login": "로그인",
  "credentialRotation.dialog.password": "새 비밀번호",
  "credentialRotation.dialog.hidden": "숨김",
  "credentialRotation.dialog.show": "표시",
  "credentialRotation.dialog.hide": "숨기기",
  "credentialRotation.dialog.regenerate": "다른 비밀번호 생성",
  "credentialRotation.dialog.regenerated": "새 비밀번호를 생성했어요.",
  "credentialRotation.dialog.allow": "이 비밀번호 사용",
  "credentialRotation.dialog.cancel": "취소",
  "credentialRotation.dialog.decideLater": "나중에 결정",
  "credentialRotation.dialog.untitled": "이 로그인",
  "credentialRotation.dialog.attempt":
    "{at, date, medium} {at, time, short}에 시작됨 · {stage}{n, plural, =0 {} other { · 다른 변경 #개 진행 중}}",
  "credentialRotation.dialog.stagePrepared": "아직 새 비밀번호를 받은 페이지가 없어요",
  "credentialRotation.dialog.stageDelivered": "새 비밀번호가 페이지에 입력되었어요",
  "credentialRotation.dialog.resultTitle": "{host}이(가) 새 비밀번호를 받아들였나요?",
  "credentialRotation.dialog.resultBody":
    "{browser}에서 변경이 작동했다고 알려 왔어요. 확인하면 새 비밀번호가 {title}의 비밀번호가 되고, 현재 비밀번호는 기록으로 이동해요.",
  "credentialRotation.dialog.confirm": "작동함",
  "credentialRotation.dialog.keepOld": "이전 비밀번호 유지",
  "credentialRotation.dialog.failedTitle": "이전 비밀번호를 유지할까요?",
  "credentialRotation.dialog.failedBody":
    "{browser}에서 변경이 작동하지 않았다고 알려 왔어요. Mosaic은 {title}의 현재 비밀번호를 유지해요. 사이트가 그래도 변경했을 수 있으니 새 값은 기록에 남아요.",
  "credentialRotation.addOrigin.title": "{title}에 {host}을(를) 추가할까요?",
  "credentialRotation.addOrigin.body":
    "Mosaic이 {host}에서도 이 로그인을 입력해요. {host}이(가) 정말 같은 사이트의 일부일 때만 추가하세요.",
  "credentialRotation.addOrigin.idn":
    "이 주소는 특수 문자를 사용해요. 브라우저에는 {unicode}(으)로 표시될 수 있어요. 주의해서 확인하세요.",
  "credentialRotation.addOrigin.add": "주소 추가",
  "credentialRotation.addOrigin.cancel": "나중에",
} satisfies Record<keyof typeof en, string>;
