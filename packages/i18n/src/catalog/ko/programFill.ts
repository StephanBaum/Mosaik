/**
 * Korean UI strings: programFill. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { programFill as en } from "../en/programFill.ts";

export const programFill = {
  "programFill.chip.label": "Mosaic으로 자동 입력",
  "programFill.chip.locked": "자동 입력하려면 Mosaic 잠금 해제",
  "programFill.list.title": "{program}의 로그인",
  "programFill.list.none": "{program}의 로그인이 아직 없어요",
  "programFill.list.signerChanged": "이 프로그램의 서명자가 바뀌었어요. 다시 연결할까요?",
  "programFill.list.fileChanged": "연결한 후 이 프로그램이 변경되었어요. 다시 연결할까요?",
  "programFill.list.suggestions": "추천",
  "programFill.list.search": "로그인 검색",
  "programFill.list.link": "로그인 연결…",
  "programFill.list.block": "이 프로그램에서는 제안 안 함",
  "programFill.link.packaged": "{name} · 스토어 앱 · 게시자 {publisher}",
  "programFill.link.packagedNoPublisher": "{name} · 스토어 앱",
  "programFill.link.signed": "{exe} · {signer} 서명, Windows에서 확인함",
  "programFill.link.unsigned": "{exe} · 서명되지 않음 · {path}",
  "programFill.link.unsignedNote":
    "Mosaic은 이 파일에만 자동 입력해요. 파일이 바뀌면 Mosaic이 다시 물어봐요.",
  "programFill.link.lookalike": "주의: 이 이름은 이전에 연결한 {similar}와(과) 비슷해 보여요.",
  "programFill.link.hashChange": "연결한 파일 {old}…, 이 파일 {new}…",
  "programFill.link.login": "로그인: {title}",
  "programFill.link.confirm": "연결하고 로그인",
  "programFill.link.confirmFill": "연결하고 자동 입력",
  "programFill.link.confirmUnsigned": "서명되지 않은 이 프로그램 연결",
  "programFill.link.confirmAgain": "연결하려면 한 번 더 확인",
  "programFill.notice.elevated":
    "Mosaic은 관리자 권한으로 실행되는 프로그램에는 자동 입력할 수 없어요. 대신 Mosaic에서 비밀번호를 복사하세요.",
  "programFill.notice.noField": "Mosaic이 여기에서 로그인 입력란을 찾을 수 없어요.",
  "programFill.notice.aborted":
    "창이 바뀌어서 Mosaic이 입력을 중단했어요. 입력란을 확인하세요.",
  "programFill.notice.abortedCleared":
    "창이 바뀌어서 Mosaic이 입력을 중단하고 입력했던 내용을 지웠어요.",
  "programFill.notice.modifiersHeld":
    "Shift, Ctrl, Alt, Windows 키에서 손을 뗀 다음 로그인을 다시 선택하세요.",
  "programFill.notice.notResponding":
    "이 프로그램이 제때 응답하지 않았어요. 더 이상 입력하지 않았어요.",
  "programFill.notice.close": "닫기",
  "programFill.settings.title": "프로그램에서 자동 입력",
  "programFill.settings.description":
    "Mosaic은 이 기기의 다른 프로그램에서 어느 입력란에 포커스가 있는지 확인해 로그인을 제안해요. 아무것도 저장하거나 전송하지 않아요.",
  "programFill.settings.enable": "프로그램에서 로그인 제안",
  "programFill.settings.submit": "입력 후 로그인",
  "programFill.settings.hotkey": "목록을 여는 단축키",
  "programFill.settings.hotkeyNone": "단축키 없음",
  "programFill.settings.hotkeySave": "목록 단축키 저장",
  "programFill.settings.blocked": "Mosaic이 로그인을 제안하지 않는 프로그램",
  "programFill.settings.blockedNone": "없음",
  "programFill.settings.unblock": "다시 제안",
  "programFill.settings.notWindows": "지금은 Windows에서만 사용할 수 있어요.",
  "programFill.settings.mosaicElevated":
    "Mosaic이 관리자 권한으로 실행 중이라 프로그램에 자동 입력하지 않아요. 일반 권한으로 시작하세요.",
  "programFill.settings.hostFailed":
    "Mosaic이 프로그램 감시를 시작할 수 없었어요. Mosaic을 다시 시작해 보세요.",
  "programFill.card.title": "프로그램에서도 로그인 자동 입력",
  "programFill.card.turnOn": "켜기",
  "programFill.card.notNow": "나중에",
  "programFill.item.program": "프로그램",
  "programFill.item.storeApp": "스토어 앱",
  "programFill.item.signedBy": "{signer} 서명",
  "programFill.item.unsigned": "서명되지 않음: {path}",
  "programFill.item.submit": "입력 후 로그인",
  "programFill.item.unlink": "연결 해제",
} satisfies Record<keyof typeof en, string>;
