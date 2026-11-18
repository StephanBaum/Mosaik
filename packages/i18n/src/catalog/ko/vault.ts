/**
 * Korean UI strings: vault. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { vault as en } from "../en/vault.ts";

export const vault = {
  "vault.itemCount": "{count, plural, =0 {항목 없음} other {항목 #개}}",
  "vault.deleteConfirm": "“{title}”을(를) 삭제할까요? 30일 동안 휴지통에서 복원할 수 있어요.",
  "vault.defaultName": "개인",
  "vault.unnamed": "이름 없는 보관함",
  "vault.vaults": "보관함",
  "vault.allItems": "모든 항목",
  "vault.items": "항목",
  "vault.search": "보관함 검색",
  "vault.untitled": "제목 없음",
  "vault.noResults":
    "“{query}”와(과) 일치하는 항목이 없어요. 철자를 확인하거나 웹사이트로 검색해 보세요.",
  "vault.new": "새 항목",
  "vault.selectItem": "항목을 선택하면 자세한 내용을 볼 수 있어요.",
  "vault.trashEmpty": "휴지통이 비어 있어요. 삭제한 항목은 30일 동안 여기에 남아요.",
  "vault.favorite": "즐겨찾기",
  "vault.unfavorite": "즐겨찾기에서 제거",
  "vault.moveTo": "보관함으로 이동",
  "vault.allVaults": "모든 보관함",
  "vault.category.favorites": "즐겨찾기",
  "vault.category.logins": "로그인",
  "vault.category.passkeys": "패스키",
  "vault.category.codes": "일회용 코드",
  "vault.category.cards": "카드",
  "vault.category.notes": "보안 메모",
  "vault.category.ssh": "SSH 키",
  "vault.flag.otp": "2단계 인증",
  "vault.moveToNamed": "{name}(으)로 이동",
  "vault.emptyTitle": "보관함이 비어 있어요",
  "vault.emptyBody":
    "로그인, 패스키, 카드 또는 메모를 추가하세요. 모든 내용은 동기화되기 전에 이 기기에서 암호화돼요.",
  "vault.addFirst": "첫 항목 추가",
  "vault.paletteTip.before": "언제든",
  "vault.paletteTip.after": "키를 눌러 검색하거나 명령을 실행하세요.",
  "trash.retention": "삭제한 항목은 30일 동안 보관돼요.",
  "trash.rowMeta": "{deletedAt, date, medium}에 삭제됨 · {days, plural, other {#일}} 남음",
  "trash.detailMeta": "{kind} · {vault} · {deletedAt, date, medium}에 삭제됨",
  "trash.purgeConfirm.title": "“{title}”을(를) 완전히 삭제할까요?",
  "trash.purgeConfirm.body": "되돌릴 수 없어요.",
  "vault.hardware.placeholderName": "코드 (보안 키)",
  "vault.hardware.title": "이 보관함에는 보안 키가 필요해요",
  "vault.hardware.body":
    "누군가 마스터 비밀번호와 비밀 키를 알게 되더라도 여기의 코드는 안전하지만, 보안 키 없이는 나도 볼 수 없어요.",
  "vault.hardware.open": "보안 키 사용",
  "vault.hardware.lockedNote": "잠겨 있어요. 다른 보관함은 잠금 해제 상태로 유지돼요.",
  "vault.hardware.lockNow": "이 보관함 잠금",
  "vault.hardware.closesIn": "{time} 후 잠겨요",
  "vault.hardware.backupTitle": "백업 키 등록",
  "vault.hardware.backupBody":
    "등록된 키가 1개예요. 2개를 권장해요. 유일한 키를 잃어버리면 이 보관함을 다시 열 수 없어요.",
  "vault.hardware.createOption": "보안 키로 보호 (코드 전용)",
  "vault.hardware.tradeoff":
    "이 보관함의 코드는 매번 보안 키가 필요하므로 마스터 비밀번호만 도난당해서는 열 수 없지만, 등록된 키를 모두 잃어버리면 이 코드도 잃게 돼요.",
  "vault.hardware.removeNote":
    "키를 제거해도 이 보관함의 키는 바뀌지 않아요. 키를 도난당했다면 코드를 새 보관함으로 옮기세요.",
  "vault.hardware.moveNote":
    "이미 사용 중인 코드를 이 보관함으로 옮겨도 이전 사본은 보호되지 않아요. 이전 항목, 기록, 백업, 내보내기 파일이 그래요. 가능하다면 서비스 제공업체에서 새 비밀 값을 발급받아 그것을 추가하세요.",
  "vault.hardware.pinBody":
    "키에 PIN이 있으면 입력한 다음 키가 깜박일 때 터치하세요.",
  "vault.hardware.keys": "키",
  "vault.hardware.keysLabel": "이 보관함의 키",
  "vault.hardware.eyebrow": "하드웨어로 보호되는 코드",
  "vault.hardware.selfLocked": "이 보관함이 자동으로 잠겼어요. 보안 키를 사용해 다시 여세요.",
  "vault.hardware.closingSoon":
    "이 보관함은 30초 이내에 잠겨요. 그 후에는 보안 키를 사용해 다시 여세요.",
  "vault.hardware.keysTitle": "이 보관함의 키",
  "vault.hardware.codesOnly": "일회용 코드 전용",
  "vault.create": "새 보관함",
  "vault.create.name": "보관함 이름",
  "vault.create.submit": "보관함 만들기",
  "vault.menu": "보관함 작업",
  "vaultKey.menu": "보관함 키 갱신…",
  "vaultKey.title": "{vault}의 키 갱신",
  "vaultKey.body":
    "Mosaic이 이 보관함의 새 키를 만들어 접근 권한이 있는 모든 기기와 사람에게 전달해요. 이미 가진 사본은 계속 읽을 수 있고, 새 키는 지금부터 변경되는 내용을 보호해요. 누군가 접근 권한을 잃었거나 기기를 분실했을 때 사용하세요.",
  "vaultKey.attachments": "첨부 파일의 키도 함께 갱신",
  "vaultKey.attachments.hint":
    "이 보관함의 모든 파일에도 새 키를 지정해요. 파일이 많거나 크면 더 오래 걸려요.",
  "vaultKey.reauth": "보관함 키를 갱신하려면 마스터 비밀번호가 필요해요.",
  "vaultKey.submit": "키 갱신",
  "vaultKey.working": "키를 갱신하는 중…",
  "vaultKey.progress": "첨부 파일 키 갱신 중: {total}개 중 {done}개.",
  "vaultKey.done": "보관함 키를 갱신했어요.",
  "vaultKey.doneAttachments":
    "보관함 키를 갱신했고 첨부 파일 키도 갱신했어요.",
  "vaultKey.unfinished":
    "보관함 키를 갱신했어요. 첨부 파일 키 갱신은 나중에 이어서 진행돼요. 그 전에 Mosaic이 종료되면 다음에 키 교체 계속을 선택하세요.",
  "vaultKey.unreachable":
    "{count, plural, other {첨부 파일 #개가 아직 이 기기에 없어서 이전 키가 유지돼요.}}",
  "vaultKey.close": "닫기",
  "vault.meta.title": "보관함 정보가 줄어들었어요",
  "vault.meta.body":
    "두 기기가 “{vault}”의 {fields}을(를) 동시에 변경했고, 합치면 너무 길어서 저장할 수 없었어요. Mosaic이 한 버전을 유지했거나 해당 항목을 비워 두었어요. 확인하고 필요하면 다시 입력하세요.",
  "vault.edit.title": "보관함 편집",
  "vault.meta.edit": "보관함 편집",
  "vault.meta.field.name": "이름",
  "vault.meta.field.icon": "아이콘",
  "vault.meta.field.description": "설명",
  "vault.meta.dismiss": "닫기",
} satisfies Record<keyof typeof en, string>;
