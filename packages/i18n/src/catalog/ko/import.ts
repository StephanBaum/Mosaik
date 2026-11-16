/**
 * Korean UI strings: import. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { importMessages as en } from "../en/import.ts";

export const importMessages = {
  "import.format.enpass": "Enpass (.json)",
  "import.format.bitwarden": "Bitwarden (.json, 암호화되지 않음)",
  "import.format.keepass": "KeePass (.kdbx)",
  "import.format.onePassword": "1Password (.1pux)",
  "import.format.cxf": "FIDO Credential Exchange (.json): 패스키와 비밀번호",
  "import.format.dashlane": "Dashlane (.zip)",
  "import.format.keeper": "Keeper (.json)",
  "import.format.keeperCsv": "Keeper (.csv)",
  "import.format.protonPass": "Proton Pass (.zip 또는 .json, 암호화되지 않음)",
  "import.format.csv": "CSV 파일 (.csv)",
  "import.keepassPassword": "KeePass 데이터베이스의 비밀번호",
  "import.preview.ready": "{count, plural, other {항목 #개를 가져올 준비가 되었어요.}}",
  "import.preview.attachments": "{count, plural, other {첨부 파일 #개}}",
  "import.preview.duplicate":
    "이미 비슷한 항목이 있어요. 선택이 해제되어 있어서 두 번 가져오지 않아요.",
  "import.preview.duplicateInFile": "이 파일에 같은 항목이 두 번 있어요. 사본은 선택이 해제되어 있어요.",
  "import.warning.unmappedFields":
    "일부 필드는 Mosaic에 대응하는 필드가 없어요. 사용자 지정 필드로 유지했어요.",
  "import.warning.attachmentsSkipped":
    "일부 첨부 파일이 50 MiB보다 크거나, 첨부 파일 저장 공간 1 GiB를 넘거나, 내보내기 파일에 없어요. 가져온 후 직접 추가하세요.",
  "import.warning.entriesSkipped":
    "일부 항목을 읽을 수 없어 제외돼요. 이전 비밀번호 관리자에서 확인하세요.",
  "import.warning.otpUnreadable":
    "일부 일회용 코드 설정을 인식하지 못했어요. 숨김 필드로 유지했으니 그 값으로 코드를 다시 설정하세요.",
  "import.warning.otpWeak":
    "일부 일회용 코드의 비밀 값이 80비트보다 짧아서 코드를 추측하기 쉬워요. 그래도 가져왔으니, 가능할 때 새 비밀 값으로 다시 설정하세요.",
  "import.warning.passkeysNotImported":
    "이 내보내기 형식에는 패스키의 개인 키가 들어 있지 않아서 패스키를 가져올 수 없어요. 해당 항목의 나머지는 가져와요.",
  "import.warning.passkeyUnreadable": "일부 패스키를 읽을 수 없어 제외했어요.",
  "import.warning.passkeyUnsupported":
    "일부 패스키가 Mosaic이 지원하지 않는 키 유형을 사용해서 제외했어요 (ES256만 지원해요).",
  "import.warning.passkeysSkipped":
    "패스키가 너무 많아서 일부를 제외했어요 (항목당 최대 100개, 파일당 최대 10,000개).",
  "import.warning.passkeysKeepOldApp": "패스키를 모두 옮길 때까지 이전 앱을 유지하세요.",
  "import.entry.unmappedFields":
    "일부 필드는 Mosaic에 대응하는 필드가 없어요. 사용자 지정 필드로 유지했어요.",
  "import.entry.otpUnreadable":
    "일회용 코드 설정을 인식하지 못했어요. 숨김 필드로 유지했으니 그 값으로 코드를 다시 설정하세요.",
  "import.entry.otpWeak":
    "일회용 코드의 비밀 값이 80비트보다 짧아서 코드를 추측하기 쉬워요. 그래도 가져왔으니, 가능할 때 새 비밀 값으로 다시 설정하세요.",
  "import.entry.attachmentTooLarge":
    "첨부 파일이 50 MiB보다 커서 제외했어요. 가져온 후 직접 추가하세요.",
  "import.entry.attachmentMissing":
    "내보내기 파일에 첨부 파일이 없어요. 가져온 후 직접 추가하세요.",
  "import.entry.attachmentQuota":
    "첨부 파일이 첨부 파일 저장 공간 1 GiB를 넘어서 제외했어요.",
  "import.entry.passkeysNotImported":
    "이 내보내기 형식에는 이 항목의 패스키 개인 키가 들어 있지 않아서 가져올 수 없어요. 옮길 때까지 이전 앱을 유지하세요.",
  "import.entry.passkeyUnreadable":
    "이 항목의 패스키를 읽을 수 없어 제외했어요. 옮길 때까지 이전 앱을 유지하세요.",
  "import.entry.passkeyUnsupported":
    "이 항목의 패스키가 Mosaic이 지원하지 않는 키 유형을 사용해서 제외했어요 (ES256만 지원해요). 옮길 때까지 이전 앱을 유지하세요.",
  "import.entry.historyTruncated": "이전 비밀번호는 최근 32개만 유지했어요.",
  "import.entry.tagsDropped": "비어 있거나 너무 긴 태그는 제외했어요.",
  "import.entry.archived": "이전 앱에서 보관 처리되어 있었어요. “archived” 태그가 붙어요.",
  "import.entry.unknownCategory": "알 수 없는 항목 유형이에요. 보안 메모로 가져와요.",
  "import.entry.titleCleaned":
    "제목에 보이지 않는 문자나 텍스트 방향을 뒤집는 문자가 있어서 제거했어요.",
  "import.entry.passkeysLimit":
    "이 항목의 패스키가 한 번에 가져올 수 있는 개수(항목당 100개, 파일당 10,000개)보다 많아요. 초과분은 제외했어요. 옮길 때까지 이전 앱을 유지하세요.",
  "import.entry.fieldsDropped":
    "이 항목에 Mosaic이 보관하는 것보다 많은 필드가 있어요. 초과한 필드는 제외돼요.",
  "import.entry.notesTruncated":
    "이 항목의 메모가 한 항목에 담을 수 있는 길이보다 길어요. 넘치는 부분은 제외했어요.",
  "import.entry.idnHost":
    "이 주소는 도메인 이름에 특수 문자를 사용해서 다른 사이트처럼 보일 수 있어요. 사용하기 전에 확인하세요.",
  "import.entry.urlKeptAsText": "이 웹 주소를 링크로 사용할 수 없어서 텍스트로 유지했어요.",
  "import.skip.malformed": "이 항목이 손상되어 읽을 수 없어요.",
  "import.skip.empty": "이 항목이 비어 있어요.",
  "import.skip.tooLarge": "이 항목이 한 항목에 담을 수 있는 최대 크기인 1 MiB보다 커요.",
  "import.result":
    "{imported, plural, other {항목 #개를 가져왔어요.}} {skipped, plural, =0 {} other {#개는 제외했어요.}}",
  "import.flow.title": "가져오기",
  "import.flow.formatStep": "어디에서 가져오나요?",
  "import.flow.chooseFile": "내보내기 파일 선택",
  "import.flow.fileChosen": "파일: {name}",
  "import.flow.password": "KeePass 데이터베이스의 비밀번호",
  "import.flow.preview": "미리 보기",
  "import.flow.previewBusy": "파일을 읽는 중이에요. 잠시 걸릴 수 있어요.",
  "import.flow.selectAll": "모두 선택",
  "import.flow.target": "가져올 위치",
  "import.flow.commit": "{count, plural, other {항목 #개}} 가져오기",
  "import.flow.entries": "항목",
  "import.flow.skipped": "제외됨",
  "import.flow.attachments": "{count, plural, other {첨부 파일 #개}}",
  "import.flow.cancel": "가져오기 취소",
  "import.flow.noneSelected": "가져올 항목을 하나 이상 선택하세요.",
  "import.flow.resultFailed": "{failed, plural, =0 {} other {항목 #개를 저장하지 못했어요.}}",
  "import.flow.deleteReminder":
    "지금 내보내기 파일을 삭제하세요. 비밀번호가 일반 텍스트로 들어 있어요.",
  "import.flow.dropHint":
    "내보내기 파일을 선택하세요. 모든 내용은 이 기기에서 읽고 암호화해요.",
} satisfies Record<keyof typeof en, string>;
