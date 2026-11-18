/**
 * Korean UI strings: mobileFolder. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { mobileFolder as en } from "../en/mobileFolder.ts";

export const mobileFolder = {
  "mobile.sync.never": "아직 동기화하지 않음",
  "mobile.sync.last": "{time}에 동기화됨",
  "mobile.sync.localOnly": "이 보관함은 이 휴대폰에만 있어요. 동기화가 설정되어 있지 않아요.",
  "mobile.sync.pending": "{count, plural, other {업로드 대기 중인 변경 #개}}",
  "mobile.sync.now": "지금 동기화",
  "mobile.sync.working": "동기화 중",
  "mobile.sync.warnings": "동기화 경고",
  "mobile.sync.warning.freezeSuspected":
    "다른 기기의 변경 사항이 한동안 도착하지 않았어요. 다른 기기가 켜져 있고 온라인인지 확인하세요.",
  "mobile.sync.warning.noCommonView":
    "이 휴대폰과 다른 기기가 이틀 동안 같은 서버 데이터를 보지 못했어요. 서버가 서로 다른 버전을 보여 주고 있을 수 있어요. 다른 기기를 동기화한 다음 여기에서 동기화하세요.",
  "mobile.sync.warning.rollbackDetected":
    "동기화 데이터가 이 휴대폰보다 오래된 것 같아요. 다른 기기에서 Mosaic을 열어 최신 변경 사항이 있는지 확인하세요. 그런 다음 “확인했어요”를 선택하세요.",
  "mobile.sync.warning.identityDuplicated":
    "예를 들어 백업을 복원한 후처럼, 다른 Mosaic 사본이 이 휴대폰의 ID로 쓰고 있어요. 동기화를 일시 중지했어요. 사본이 하나만 사용되는지 확인한 다음 “확인했어요”를 선택하세요.",
  "mobile.sync.warning.serverInconsistent":
    "서버가 이 휴대폰에 다른 기기와 다른 데이터를 보여 줬어요. 다른 기기에서 Mosaic을 열어 최신 변경 사항이 있는지 확인하세요. 그런 다음 “확인했어요”를 선택하세요.",
  "mobile.sync.warning.serverRollback":
    "서버에 있는 보관함 중 하나의 사본이 이 휴대폰의 것보다 오래되었어요. 다른 기기에서 Mosaic을 열어 최신 변경 사항이 있는지 확인하세요. 그런 다음 “확인했어요”를 선택하세요.",
  "mobile.sync.warning.sharingAttention":
    "공유 보관함에 확인이 필요해요. 키 변경이 대기 중이거나 구성원을 확인하지 못했어요. 컴퓨터에서 Mosaic을 열어 어느 쪽인지 확인하세요. 그런 다음 “확인했어요”를 선택하세요.",
  "mobile.sync.warning.undecryptable":
    "{count, plural, other {받은 항목 #개}}를 이 휴대폰에서 읽을 수 없었어요. 모든 기기에서 Mosaic을 업데이트한 다음 다시 동기화하세요.",
  "mobile.sync.warning.accountRollback":
    "서버에 이 휴대폰이 가진 것보다 오래된 계정 키가 있어요. 아무것도 변경하지 않았어요. 다른 기기에서 Mosaic을 열어 최신 변경 사항이 있는지 확인하세요. 그런 다음 “확인했어요”를 선택하세요.",
  "mobile.sync.acknowledge": "확인했어요…",
  "mobile.sync.acknowledgeTitle": "동기화 경고를 지울까요?",
  "mobile.sync.acknowledgeBody":
    "서버를 직접 확인한 후에만 진행하세요. 모든 경고가 지워지고 동기화가 계속돼요. 데이터가 실제로 이전 상태로 되돌려졌거나 변경되었더라도 이후에는 Mosaic이 알 수 없어요. 모든 기기에서 각각 이 작업을 하세요.",
  "mobile.sync.acknowledgeConfirm": "확인했어요",
  "mobile.folder.intro": "이 계정은 {provider} 폴더를 통해 동기화돼요.",
  "mobile.folder.signIn": "{provider}에 로그인",
  "mobile.folder.scope":
    "{provider}이(가) 모든 파일에 대한 접근을 요청해요. Mosaic은 선택한 폴더만 열고 다른 것은 건드리지 않아요.",
  "mobile.folder.waiting": "브라우저에서 로그인을 마친 다음 여기로 돌아오세요.",
  "mobile.folder.serverLabel": "Nextcloud 또는 WebDAV 주소",
  "mobile.folder.serverHint": "https://cloud.example.com",
  "mobile.folder.serverConfirm":
    "Mosaic이 {server}에 로그인해요. 본인의 서버일 때만 계속하세요.",
  "mobile.folder.locating": "Mosaic 폴더를 찾는 중…",
  "mobile.folder.browseTitle": "Mosaic 폴더 선택",
  "mobile.folder.browseHint":
    "컴퓨터가 동기화하는 폴더이며 mosaic-sync.json이라는 파일이 들어 있어요.",
  "mobile.folder.currentFolder": "현재 폴더: {path}",
  "mobile.folder.noSubfolders": "여기에는 폴더가 없어요.",
  "mobile.folder.useThis": "이 폴더 사용",
  "mobile.folder.up": "상위 폴더로",
  "mobile.folder.chooseProvider": "계정을 어디에서 동기화하나요?",
  "mobile.folder.noDrive":
    "Google Drive와 iCloud Drive는 아직 Android에서 사용할 수 없어요. Dropbox, OneDrive, Nextcloud 또는 Mosaic 동기화를 사용하세요.",
  "mobile.folder.useFolder": "대신 클라우드 폴더로 동기화",
  "mobile.folder.providerDropbox": "Dropbox",
  "mobile.folder.providerOneDrive": "OneDrive",
  "mobile.folder.providerWebDav": "Nextcloud 또는 WebDAV",
  "mobile.folder.appPasswordToggle": "대신 앱 비밀번호 사용",
  "mobile.folder.davLabel": "WebDAV 폴더 주소",
  "mobile.folder.loginLabel": "사용자 이름",
  "mobile.folder.appPasswordLabel": "앱 비밀번호",
  "mobile.folder.appPasswordHint":
    "서버의 보안 설정에서 앱 비밀번호를 만드세요. 주 비밀번호는 사용하지 마세요.",
  "mobile.folder.appPasswordAction": "앱 비밀번호로 로그인",
  "mobile.folder.pickAccount": "이 휴대폰이 참여할 계정은 무엇인가요?",
  "mobile.folder.accountOption": "계정 {id}",
  "mobile.folder.linked": "로그인했어요. 다시 동기화하는 중이에요.",
  "mobile.folder.settingsLine": "{provider}로 동기화: {path}",
  "mobile.folder.relink": "다시 로그인",
  "mobile.folder.msRevokeHint":
    "Mosaic의 접근 권한을 완전히 없애려면 Microsoft 계정 페이지의 앱에서 제거하세요.",
  "mobile.reset.revokeDropbox":
    "계정을 제거했어요. Dropbox에서 Mosaic의 접근 권한을 종료하지 못했어요. Dropbox의 설정, 연결된 앱에서 직접 제거하세요.",
  "mobile.reset.revokeWebdav":
    "계정을 제거했어요. 서버의 앱 비밀번호를 삭제하지 못했어요. 서버의 설정, 보안, 기기 및 세션에서 직접 제거하세요.",
  "mobile.reset.title": "이 휴대폰에서 이 계정 제거",
  "mobile.reset.body":
    "이 휴대폰에 저장된 보관함 사본, 비밀 키, 클라우드 로그인이 삭제돼요. 클라우드 폴더나 서버에 있는 데이터는 그대로 남아요. 나중에 비밀 키와 마스터 비밀번호로 다시 참여할 수 있어요.",
  "mobile.reset.action": "계정 제거…",
  "mobile.reset.confirm": "계정 제거",
  "mobile.reset.phraseLabel": "확인",
  "mobile.reset.typePhrase": "확인하려면 {phrase}을(를) 입력하세요",
} satisfies Record<keyof typeof en, string>;
