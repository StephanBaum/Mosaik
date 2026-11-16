/**
 * Korean UI strings: importGuide. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { importGuide as en } from "../en/importGuide.ts";

export const importGuide = {
  "import.grid.title": "지금 비밀번호는 어디에 있나요?",
  "import.grid.browsers": "브라우저",
  "import.grid.managers": "비밀번호 관리자",
  "import.grid.other": "그 밖의 방법",
  "import.source.chrome": "Chrome",
  "import.source.edge": "Edge",
  "import.source.brave": "Brave",
  "import.source.opera": "Opera",
  "import.source.vivaldi": "Vivaldi",
  "import.source.arc": "Arc",
  "import.source.firefox": "Firefox",
  "import.source.safari": "Safari",
  "import.source.applePasswords": "Apple 비밀번호",
  "import.source.onePassword": "1Password",
  "import.source.bitwarden": "Bitwarden",
  "import.source.keepass": "KeePass",
  "import.source.keepassxcCsv": "KeePassXC (CSV)",
  "import.source.enpass": "Enpass",
  "import.source.lastpass": "LastPass",
  "import.source.dashlane": "Dashlane",
  "import.source.keeper": "Keeper",
  "import.source.nordpass": "NordPass",
  "import.source.protonPass": "Proton Pass",
  "import.source.roboform": "RoboForm",
  "import.source.otherFile": "다른 CSV 또는 파일",
  "import.guide.chrome.1":
    "Chrome에서 이 주소를 여세요. 복사해서 주소 표시줄에 붙여넣으면 돼요.",
  "import.guide.chrome.2":
    "“비밀번호 내보내기” 옆의 “파일 다운로드”를 클릭하고 컴퓨터 비밀번호로 확인하세요.",
  "import.guide.chrome.3": "Chrome이 “Chrome Passwords.csv”를 다운로드 폴더에 저장해요.",
  "import.guide.edge.1": "Edge에서 이 주소를 여세요. 복사해서 주소 표시줄에 붙여넣으면 돼요.",
  "import.guide.edge.2":
    "“저장된 암호” 옆의 “⋯”를 클릭한 다음 “암호 내보내기”를 클릭하고 확인하세요.",
  "import.guide.edge.3": "Edge가 CSV 파일을 다운로드 폴더에 저장해요.",
  "import.guide.brave.1": "Brave에서 이 주소를 여세요. 복사해서 주소 표시줄에 붙여넣으면 돼요.",
  "import.guide.brave.2":
    "“비밀번호 내보내기” 옆의 “파일 다운로드”를 클릭하고 컴퓨터 비밀번호로 확인하세요.",
  "import.guide.brave.3": "Brave가 CSV 파일을 다운로드 폴더에 저장해요.",
  "import.guide.opera.1": "Opera에서 이 주소를 여세요. 복사해서 주소 표시줄에 붙여넣으면 돼요.",
  "import.guide.opera.2":
    "“비밀번호 내보내기” 옆의 “파일 다운로드”를 클릭하고 컴퓨터 비밀번호로 확인하세요.",
  "import.guide.opera.3": "Opera가 CSV 파일을 다운로드 폴더에 저장해요.",
  "import.guide.vivaldi.1":
    "Vivaldi에서 이 주소를 여세요. 복사해서 주소 표시줄에 붙여넣으면 돼요.",
  "import.guide.vivaldi.2":
    "“비밀번호 내보내기” 옆의 “파일 다운로드”를 클릭하고 컴퓨터 비밀번호로 확인하세요.",
  "import.guide.vivaldi.3": "Vivaldi가 CSV 파일을 다운로드 폴더에 저장해요.",
  "import.guide.arc.1": "Arc에서 이 주소를 여세요. 복사해서 주소 표시줄에 붙여넣으면 돼요.",
  "import.guide.arc.2":
    "“비밀번호 내보내기” 옆의 “파일 다운로드”를 클릭하고 컴퓨터 비밀번호로 확인하세요.",
  "import.guide.arc.3": "Arc가 CSV 파일을 다운로드 폴더에 저장해요.",
  "import.guide.firefox.1":
    "Firefox에서 이 주소를 여세요. 복사해서 주소 표시줄에 붙여넣으면 돼요.",
  "import.guide.firefox.2":
    "오른쪽 위의 “⋯”를 클릭한 다음 “비밀번호 내보내기…”를 클릭하고 확인하세요.",
  "import.guide.firefox.3": "“logins.csv”를 다운로드 폴더에 저장하세요.",
  "import.guide.safari.1":
    "Safari → 설정 → 암호 (macOS 14 이하. macOS 15 이상에서는 Apple 비밀번호 앱을 사용하세요).",
  "import.guide.safari.2": "목록 아래의 “⋯”를 클릭한 다음 “모든 암호 내보내기…”를 클릭하세요.",
  "import.guide.safari.3": "CSV 파일을 다운로드 폴더에 저장하세요.",
  "import.guide.applePasswords.1": "비밀번호 앱을 여세요.",
  "import.guide.applePasswords.2": "파일 → “모든 암호를 파일로 내보내기…”.",
  "import.guide.applePasswords.3": "CSV 파일을 다운로드 폴더에 저장하세요.",
  "import.guide.onePassword.1": "1Password 앱을 열고 잠금을 해제하세요.",
  "import.guide.onePassword.2": "파일 → 내보내기 → 내 계정을 선택한 다음 비밀번호를 입력하세요.",
  "import.guide.onePassword.3": "“1PUX”를 선택하고 “데이터 내보내기”를 클릭하세요.",
  "import.guide.bitwarden.1": "웹 보관함 또는 앱에서: 도구 → 보관함 내보내기.",
  "import.guide.bitwarden.2": "파일 형식으로 “.json”을 선택하세요 (“암호화됨”은 안 돼요).",
  "import.guide.bitwarden.3": "마스터 비밀번호로 확인하세요.",
  "import.guide.keepass.1": "데이터베이스 파일(.kdbx)을 그대로 사용하세요.",
  "import.guide.keepass.2": "다음 단계에서 파일의 비밀번호를 입력해요. 비밀번호는 이 기기에만 남아요.",
  "import.guide.keepassxcCsv.1": "KeePassXC: 데이터베이스 → 내보내기 → CSV 파일…",
  "import.guide.keepassxcCsv.2":
    "다운로드 폴더에 저장하세요. .kdbx 파일이 더 많은 정보를 담고 있으니 가능하면 “KeePass”를 선택하세요.",
  "import.guide.enpass.1": "Enpass: 메뉴 → 파일 → 내보내기.",
  "import.guide.enpass.2":
    "보관함을 선택하고 형식으로 “.json”을 고른 다음 마스터 비밀번호를 입력하세요.",
  "import.guide.enpass.3": "다운로드 폴더에 저장하세요.",
  "import.guide.lastpass.1": "lastpass.com에 로그인하고 고급 옵션 → 내보내기를 여세요.",
  "import.guide.lastpass.2": "마스터 비밀번호를 입력하세요. CSV 파일이 다운로드돼요.",
  "import.guide.lastpass.3":
    "브라우저에 파일 대신 텍스트가 표시되면 페이지를 “lastpass.csv”로 저장하세요.",
  "import.guide.dashlane.1": "브라우저에서 Dashlane을 열고 내 계정 → 설정 → 데이터 내보내기로 이동하세요.",
  "import.guide.dashlane.2": "“CSV”를 선택하세요. Dashlane이 ZIP 파일을 다운로드해요.",
  "import.guide.dashlane.3": "ZIP 파일은 그대로 두세요.",
  "import.guide.keeper.1": "Keeper 웹 보관함 → 설정 → 내보내기.",
  "import.guide.keeper.2": "“JSON”을 선택하고 (사용자 지정 필드와 2단계 인증 코드가 유지돼요) 내보내세요.",
  "import.guide.nordpass.1": "NordPass → 설정 → 항목 내보내기.",
  "import.guide.nordpass.2":
    "마스터 비밀번호로 확인하고 CSV 파일을 다운로드 폴더에 저장하세요.",
  "import.guide.protonPass.1": "Proton Pass → 설정 → 내보내기.",
  "import.guide.protonPass.2":
    "“ZIP”을 선택하고 암호화는 끄세요. Mosaic은 PGP로 암호화된 내보내기 파일을 읽을 수 없어요.",
  "import.guide.protonPass.3": "잠금을 해제하고 다운로드 폴더에 저장하세요.",
  "import.guide.roboform.1": "RoboForm → 설정 → 계정 및 데이터 → 내보내기.",
  "import.guide.roboform.2": "“CSV”와 “로그인”을 선택한 다음 다운로드 폴더에 저장하세요.",
  "import.guide.otherFile.1": "머리글 행이 있는 CSV 파일이나 위의 형식 중 하나로 내보내세요.",
  "import.guide.otherFile.2": "파일을 선택하세요. Mosaic이 열을 대응시켜요.",
  "import.guide.copyUrl": "주소 복사",
  "import.guide.urlCopied": "주소를 복사했어요.",
  "import.guide.choose": "내보내기 파일 선택",
  "import.guide.dropHint": "…또는 파일 하나를 여기에 끌어다 놓으세요. 파일은 이 기기에서만 읽어요.",
  "import.guide.back": "뒤로",
  "import.detect.detected": "인식됨: {source}",
  "import.detect.mismatch":
    "이 파일은 {source}에서 내보낸 것 같아요. Mosaic이 그렇게 읽을 거예요.",
  "import.detected.chromium": "Chrome, Edge 또는 다른 Chromium 브라우저",
  "import.detected.firefox": "Firefox",
  "import.detected.safari": "Safari 또는 Apple 비밀번호",
  "import.detected.bitwardenCsv": "Bitwarden (CSV)",
  "import.detected.bitwarden": "Bitwarden",
  "import.detected.enpass": "Enpass",
  "import.detected.keepass": "KeePass",
  "import.detected.keepassxcCsv": "KeePassXC (CSV)",
  "import.detected.onePassword": "1Password",
  "import.detected.cxf": "FIDO Credential Exchange 파일",
  "import.detected.lastpass": "LastPass",
  "import.detected.dashlane": "Dashlane",
  "import.detected.keeper": "Keeper",
  "import.detected.nordpass": "NordPass",
  "import.detected.protonPass": "Proton Pass",
  "import.detected.roboform": "RoboForm",
  "import.detected.csv": "CSV 파일",
  "import.lost.title": "가져오지 못하는 것",
  "import.lost.passkeys":
    "패스키: {source}은(는) 내보내기에 패스키를 넣지 않아요. 새 패스키를 설정할 때까지 {source}을(를) 유지하세요.",
  "import.lost.attachments":
    "첨부 파일: {source}은(는) 내보내기에 첨부 파일을 넣지 않아요. 직접 추가하세요.",
  "import.lost.history": "이전 비밀번호(기록): 이 내보내기에는 들어 있지 않아요.",
  "import.lost.roboformExtras":
    "신원 정보, 보안 메모, 북마크: 따로 내보내세요. 로그인 파일에 포함되지 않아요.",
  "import.lost.entriesSkipped":
    "{count, plural, other {항목 #개를 읽을 수 없어 제외했어요.}}",
  "import.lost.fieldsDropped":
    "일부 항목에 Mosaic이 보관하는 것보다 많은 필드가 있어요. 초과한 필드는 제외돼요.",
  "import.lost.notesTruncated":
    "일부 메모가 한 항목에 담을 수 있는 길이보다 길어요. 넘치는 부분은 제외돼요.",
  "import.lost.attachmentsSkipped":
    "일부 첨부 파일이 없거나, 너무 크거나, 첨부 파일 저장 공간을 넘어요. 직접 추가하세요.",
  "import.cleanup.title": "정리",
  "import.cleanup.delete": "지금 {name} 삭제",
  "import.cleanup.deleted": "{name}을(를) 덮어쓰고 삭제했어요.",
  "import.cleanup.deleteFailed": "{name}을(를) 삭제할 수 없었어요. 직접 삭제하세요.",
  "import.cleanup.keepFile":
    "일부 항목을 가져오지 못했거나 완전히 가져오지 못했어요. 빠진 것을 확인할 때까지 {name}을(를) 유지한 다음 직접 삭제하세요.",
  "import.cleanup.ssdNote":
    "SSD와 동기화 폴더에서는 이전 사본이 버전 기록, 백업, 휴지통에 남아 있을 수 있어요. 그곳도 확인하세요.",
  "import.cleanup.synced":
    "이 파일은 {service} 폴더에 있어요. {service}에 이전 버전이 남아 있을 수 있으니 그곳에서도 삭제하세요.",
  "import.synced.oneDrive": "OneDrive",
  "import.synced.dropbox": "Dropbox",
  "import.synced.googleDrive": "Google Drive",
  "import.synced.icloud": "iCloud",
  "import.synced.box": "Box",
  "import.checklist.title": "이전 앱을 떠나기 전에",
  "import.checklist.browserSaving":
    "{source}에서 비밀번호 저장과 자동 로그인을 꺼서 Mosaic과 함께 저장을 제안하지 않게 하세요.",
  "import.checklist.firefoxSaving":
    "about:preferences#privacy를 열고 “비밀번호를 저장할지 묻기”를 해제해서 Firefox가 Mosaic과 함께 저장을 제안하지 않게 하세요.",
  "import.checklist.safariSaving":
    "Safari → 설정 → 자동 완성: “사용자 이름 및 암호”를 해제해서 Safari가 Mosaic과 함께 자동으로 채우지 않게 하세요.",
  "import.checklist.oldExtension": "{source}의 브라우저 확장 프로그램을 꺼서 하나의 앱만 자동 입력하게 하세요.",
  "import.checklist.deleteOld":
    "Mosaic에서 모든 것을 확인한 후 {source}의 데이터를 삭제하세요.",
} satisfies Record<keyof typeof en, string>;
