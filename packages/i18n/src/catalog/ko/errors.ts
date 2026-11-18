/**
 * Korean UI strings: errors. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { errors as en } from "../en/errors.ts";

export const errors = {
  "error.notImplemented": "아직 사용할 수 없어요. Mosaic의 이후 버전에서 제공될 예정이에요.",
  "error.noAccount":
    "이 기기에는 아직 계정이 없어요. 계정을 만들거나 비상 키트로 로그인하세요.",
  "error.accountExists": "이 기기에는 이미 Mosaic 계정이 있어요. 대신 잠금을 해제하세요.",
  "error.locked": "Mosaic이 잠겨 있어요. 계속하려면 잠금을 해제하세요.",
  "error.alreadyUnlocked": "Mosaic은 이미 잠금 해제되어 있어요. 하던 작업을 이어서 하세요.",
  "error.wrongCredentials":
    "마스터 비밀번호 또는 비밀 키가 올바르지 않아요. Caps Lock과 키보드 언어를 확인한 다음 다시 시도하세요.",
  "error.secretKeyRequired":
    "이 기기에는 비밀 키가 없어요. 비상 키트에서 입력하세요.",
  "error.secretKeyMalformed":
    "비밀 키가 아니에요. 내 비밀 키는 “M1-”로 시작하며 비상 키트에 있어요.",
  "error.secretKeyMistyped":
    "이 비밀 키에 오타가 있어요. 비상 키트와 한 글자씩 비교해 보세요.",
  "error.emptyPassword": "마스터 비밀번호가 비어 있어요. 계속하려면 입력하세요.",
  "error.quickUnlockUnavailable":
    "이 기기에는 빠른 잠금 해제가 설정되어 있지 않아요. 마스터 비밀번호로 잠금을 해제하세요.",
  "error.storeInUse":
    "이 컴퓨터에서 Mosaic이 이미 열려 있어요. 다른 창을 닫은 다음 다시 시도하세요.",
  "error.storeUnavailable":
    "Mosaic이 이 기기에서 보관함 파일을 열 수 없었어요. 디스크를 사용할 수 있고 여유 공간이 있는지 확인한 다음 Mosaic을 다시 시작하세요.",
  "error.vaultNotFound": "이 보관함은 더 이상 없어요. 다른 기기에서 삭제되었을 수 있어요.",
  "error.vaultUnavailable":
    "이 세션에서는 이 보관함을 열 수 없어요. 하드웨어 키로 잠금을 해제하거나 동기화한 다음 다시 시도하세요.",
  "error.itemNotFound":
    "이 항목은 더 이상 없어요. 다른 기기에서 이동되었거나 삭제되었을 수 있어요.",
  "error.itemDeleted":
    "이 항목은 완전히 삭제되어 열 수 없어요. 다른 항목을 선택하세요.",
  "error.sameVault": "항목이 이미 그 보관함에 있어요. 다른 보관함을 선택하세요.",
  "error.noOtp": "이 항목에는 일회용 코드가 없어요. 항목을 편집해 추가하세요.",
  "error.fieldEmpty":
    "이 필드가 비어 있어서 복사하거나 표시할 내용이 없어요. 항목을 편집해 채우세요.",
  "error.invalidInput":
    "이 양식에 올바르지 않은 내용이 있어요. 강조 표시된 필드를 확인한 다음 다시 시도하세요.",
  "error.sshKeyInvalid":
    "OpenSSH 개인 키가 아니에요. BEGIN과 END 줄을 포함해 키 전체를 붙여 넣으세요.",
  "error.reauthRequired": "이 변경에는 마스터 비밀번호가 필요해요. 확인하려면 입력하세요.",
  "error.attachmentTooLarge": "파일이 50 MiB보다 커요. 더 작은 파일을 선택하세요.",
  "error.attachmentQuota":
    "파일이 사용 가능한 1 GiB를 넘게 돼요. 첨부 파일을 일부 제거한 다음 다시 시도하세요.",
  "error.attachmentNotFound": "이 파일은 더 이상 항목에 속해 있지 않아요. 항목을 다시 불러오세요.",
  "error.attachmentUnavailable":
    "이 기기에서 이 파일을 열 수 없어요. 아직 동기화되지 않았거나 손상되었을 수 있어요.",
  "error.attachmentFileUnreadable": "이 파일을 읽을 수 없어요. 다른 파일을 선택하세요.",
  "error.attachmentSaveFailed": "그 위치에 파일을 저장할 수 없었어요. 다른 폴더를 선택하세요.",
  "error.attachmentPreviewUnsupported": "이 파일은 미리 볼 수 없어요. 다른 이름으로 저장을 사용해 여세요.",
  "error.attachmentPreviewTooLarge": "이 이미지는 너무 커서 미리 볼 수 없어요. 다른 이름으로 저장을 사용해 여세요.",
  "error.clipboardUnavailable":
    "Mosaic이 클립보드를 사용할 수 없었어요. 클립보드를 점유하고 있을 수 있는 앱을 닫은 다음 다시 시도하세요.",
  "error.importFileUnreadable":
    "Mosaic이 이 파일을 읽을 수 없었어요. 파일이 아직 있고 다른 앱에서 열려 있지 않은지 확인한 다음 다시 시도하세요.",
  "error.importDeleteNotAllowed":
    "먼저 파일을 가져오세요. Mosaic은 내보내기 파일의 항목이 보관함에 들어온 후에만 파일을 삭제해요.",
  "error.importDeleteFailed":
    "파일을 삭제할 수 없었어요. 다른 앱에서 열려 있거나 읽기 전용일 수 있어요. 직접 삭제한 다음 휴지통을 비우세요.",
  "error.importFormatInvalid":
    "이 파일은 선택한 형식이 아니에요. 맞는 형식을 선택하거나 파일을 다시 내보내세요.",
  "error.importPreviewExpired": "이 가져오기가 취소되었어요. 처음부터 다시 하려면 파일을 다시 선택하세요.",
  "error.sshRequestExpired":
    "이 SSH 요청은 이미 끝났어요. 아직 필요하면 명령을 다시 실행하세요.",
  "error.sshAgentUnavailable":
    "SSH 에이전트를 시작할 수 없었어요. 다른 SSH 에이전트가 실행 중이지 않은지 확인한 다음 다시 켜세요.",
  "error.unlockDelayed": "연속으로 너무 많이 틀렸어요. 몇 초 기다린 다음 다시 시도하세요.",
  "error.tooManyAttempts":
    "너무 많이 틀렸어요. Mosaic이 잠겼어요. 계속하려면 마스터 비밀번호로 잠금을 해제하세요.",
  "error.sshAllowTooSoon":
    "요청을 읽기에는 너무 빨랐어요. 키와 프로그램을 확인한 다음 허용을 다시 선택하세요.",
  "error.browserRequestExpired":
    "이 브라우저 요청은 이미 끝났어요. 아직 필요하면 확장 프로그램에서 다시 요청하세요.",
  "error.browserAllowTooSoon":
    "요청을 읽기에는 너무 빨랐어요. 내용을 확인한 다음 다시 선택하세요.",
  "error.browserPairingNotFound":
    "이 브라우저는 더 이상 연결되어 있지 않아요. 설정을 다시 열어 현재 목록을 확인하세요.",
  "error.browserHostMissing":
    "이 Mosaic에는 브라우저 연결 구성 요소가 설치되어 있지 않아서 복구할 것이 없어요. 브라우저를 연결하려면 설치 프로그램으로 Mosaic을 설치하세요.",
  "error.agentOriginInvalid":
    "에이전트는 https://로 시작하고 https://api.example.com처럼 사이트만 나타내는 주소로만 키를 보낼 수 있어요. 주소를 확인하세요.",
  "error.agentBindingInvalid":
    "Mosaic이 에이전트용으로 이 설정을 사용할 수 없어요. 필드, 사이트, 키 전송 방식을 확인하세요.",
  "error.agentValueUnusable":
    "이 값이 너무 짧거나 줄바꿈이 있어서 에이전트가 사용할 수 없어요. 키는 한 줄에 12자 이상이어야 해요.",
  "error.agentVaultShared":
    "에이전트는 내 소유이면서 공유되지 않은 보관함의 항목만 사용할 수 있어요. 먼저 항목을 내 보관함 중 하나로 옮기세요.",
  "error.agentAccessOff": "에이전트 접근이 꺼져 있어요. 설정의 에이전트에서 켜세요.",
  "error.agentRequestExpired":
    "이 에이전트 요청은 이미 끝났어요. 아직 필요하면 에이전트가 다시 요청할 수 있어요.",
  "error.agentAllowTooSoon":
    "요청을 읽기에는 너무 빨랐어요. 내용을 확인한 다음 다시 선택하세요.",
  "error.agentPairingNotFound":
    "이 에이전트는 더 이상 연결되어 있지 않아요. 설정을 다시 열어 현재 목록을 확인하세요.",
  "error.agentEndpointUnavailable":
    "Mosaic이 에이전트가 사용하는 연결을 열 수 없었어요. 다른 Mosaic을 모두 닫은 다음 다시 시도하세요.",
  "error.kitNotVerified":
    "먼저 새 비상 키트를 저장하고 마지막 그룹을 확인하세요. 그 전에는 Mosaic이 아무것도 변경하지 않아요.",
  "error.kitGroupMismatch":
    "새 비밀 키의 마지막 그룹이 아니에요. 새 비상 키트를 확인하세요.",
  "error.rotationInProgress":
    "다른 키 변경이 아직 열려 있어요. 키 교체 계속으로 끝내거나 먼저 취소하세요.",
  "error.rotationNone": "계속할 키 변경이 없어요.",
  "error.rotationNotFound":
    "이 비밀번호 변경은 더 이상 열려 있지 않아요. 다른 기기나 브라우저에서 닫혔을 수 있어요. 항목을 다시 열어 상태를 확인하세요.",
  "error.rotationLimit":
    "이 항목의 비밀번호 변경이 이미 4개 열려 있어요. 먼저 항목에서 하나를 끝내거나 닫으세요.",
  "error.rotationOpen":
    "이 로그인에는 열려 있는 비밀번호 변경이 있어요. 휴지통에서 복원한 다음 항목에서 작동함 또는 작동 안 함을 선택하세요. 그 후에 완전히 삭제할 수 있어요.",
  "error.rotationNotSupported":
    "Mosaic이 이 변경을 안내할 수 없어요. https 주소가 있는 로그인이 필요해요. 사이트에서 비밀번호를 직접 변경하세요.",
  "error.historyNotFound":
    "이 이전 비밀번호는 더 이상 기록에 없어요. 항목을 다시 열어 현재 목록을 확인하세요.",
  "error.originNotRelated":
    "이 페이지는 이 로그인의 사이트에 속하지 않아서 Mosaic이 추가하지 않았어요. 추가해야 한다면 항목의 주소를 편집하세요.",
  "error.breachConsentRequired":
    "유출 확인이 꺼져 있어요. 먼저 켜고 전송되는 내용에 동의하세요.",
  "error.breachRunning":
    "유출 확인이 이미 실행 중이에요. 끝날 때까지 기다리거나 먼저 중지하세요.",
  "error.rotationTooFar":
    "이 변경은 이미 계정에 반영되어서 취소할 수 없어요. 계속해서 끝내세요.",
  "error.rotationJumpTooLarge":
    "이 기기는 다른 기기보다 너무 뒤처져서 따라잡을 수 없어요. 다시 설정하세요. 이 기기에서 계정을 제거한 다음 비상 키트로 다시 참여하세요.",
  "error.rotationNotApplied":
    "변경이 계정에 반영되지 않아서 아무것도 바뀌지 않았어요. 다시 시작하거나 취소하세요.",
  "error.secretKeyUnchanged":
    "이 비밀 키는 더 이상 사용할 수 없어요. 변경을 진행한 기기에서 저장한 비상 키트의 비밀 키를 사용하세요.",
  "error.passwordUnchanged": "새 마스터 비밀번호는 현재 비밀번호와 달라야 해요.",
  "error.profileDowngrade":
    "이 수준은 계정이 지금 사용하는 수준보다 강하지 않아요. 더 높은 수준을 선택하세요.",
  "error.profileTooSlow":
    "이 수준으로 잠금을 해제하면 이 기기에서 2초 넘게 걸려요. 더 낮은 수준을 선택하거나 그래도 업그레이드하세요.",
  "error.rekeyNeedsCleanSync":
    "키 재설정을 하려면 동기화가 최신이고 경고가 없어야 해요. 지금 동기화하고 경고가 있으면 확인한 다음 다시 시도하세요.",
  "error.rekeyMemberKeyMismatch":
    "공유 보관함의 모든 구성원의 보안 코드를 아직 비교하지 않아서 그 보관함을 갱신할 수 없어요. 보관함의 공유 설정에서 코드를 비교한 다음 다시 시도하세요.",
  "error.rekeyBusy": "Mosaic이 작업하는 동안 보관함이 계속 변경되었어요. 잠시 기다린 다음 다시 시도하세요.",
  "error.rekeyOutcomeUnknown":
    "계정이 새 키를 받아들였는지 Mosaic이 확인할 수 없었어요. 키 교체 계속을 선택해 확인하세요.",
  "error.rekeyNeedsServer":
    "키 재설정에는 동기화 서버나 폴더가 필요한데 연결할 수 없어요. 연결을 확인한 다음 다시 시도하세요.",
  "error.rekeyNoAccountFile":
    "동기화 폴더에서 이 비밀번호와 비밀 키로 열 수 있는 것이 없어요. 새 비상 키트를 확인하세요.",
  "error.syncAccountRekeyed":
    "다른 기기에서 계정의 키를 갱신했어요. 계속하려면 새 비상 키트의 비밀 키를 사용하세요.",
  "error.syncEpochRollback":
    "동기화 백엔드가 이 기기보다 오래된 계정 키를 보여 주고 있어요. 아무것도 변경하지 않았어요.",
  "error.syncRekeyRefused":
    "동기화 백엔드가 제시한 키는 이 기기의 키 다음 키가 아니에요. 아무것도 변경하지 않았어요. 다른 기기를 확인하세요.",
  "error.pickerStale": "이 목록이 오래되었어요. 입력란을 다시 클릭하세요.",
  "error.pickerTooSoon": "잠시 기다린 다음 다시 확인하세요.",
  "error.programFillUnavailable": "이 기기에서는 프로그램 자동 입력을 사용할 수 없어요.",
  "error.busy": "Mosaic이 마지막 단계를 아직 마무리하고 있어요. 잠시 기다린 다음 다시 시도하세요.",
  "error.internal":
    "예기치 않은 문제로 이 작업이 중단되었어요. 다시 시도하고, 계속되면 Mosaic을 다시 시작하세요.",
  "error.importTooLarge":
    "이 파일은 너무 크거나 복잡해서 가져올 수 없어요. 한 번에 더 적은 항목을 내보낸 다음 다시 시도하세요.",
  "error.importUnsupported":
    "Mosaic이 이 형식 버전을 읽을 수 없어요. 최신 형식으로 다시 내보낸 다음 시도하세요 (KeePass는 AES 또는 ChaCha20을 사용하는 KDBX 4).",
  "error.importEncrypted":
    "이 내보내기 파일은 비밀번호로 보호되어 있어요. 암호화하지 않고 다시 내보내 가져온 다음 파일을 삭제하세요.",
  "error.sshAgentEndpointInUse":
    "다른 SSH 에이전트가 이미 실행 중이에요 (Windows에서는 OpenSSH Authentication Agent 서비스). 그것을 중지한 다음 Mosaic의 SSH 에이전트를 다시 켜세요.",
  "error.cancelled": "취소했어요. 아무것도 바뀌지 않았어요.",
  "error.quickUnlockExpired":
    "빠른 잠금 해제가 만료되었어요. 마스터 비밀번호로 잠금을 해제한 다음 설정에서 다시 켜세요.",
  "error.quickUnlockInvalidated":
    "이 기기에서 빠른 잠금 해제가 작동하지 않아 꺼졌어요. 마스터 비밀번호로 잠금을 해제하세요.",
  "error.passwordRequired":
    "이번에는 마스터 비밀번호를 입력하세요. 그 후에는 빠른 잠금 해제가 다시 작동해요.",
  "error.hardwareKeyUnsupported": "이 보안 키나 시스템은 Mosaic에서 사용할 수 없어요.",
  "error.hardwareKeyNotFound": "보안 키를 찾을 수 없어요. 키를 꽂은 다음 다시 시도하세요.",
  "error.hardwareKeyTimeout": "시간 안에 키를 터치하지 않았어요. 다시 시도하세요.",
  "error.hardwareKeyPinRequired": "보안 키의 PIN을 입력하세요.",
  "error.hardwareKeyPinInvalid": "PIN이 올바르지 않아요. 여러 번 틀리면 키가 스스로 잠겨요.",
  "error.hardwareKeyPinBlocked":
    "PIN을 너무 많이 틀려서 이 키가 잠겼어요. 제조사 도구로 초기화하세요. 그러면 등록 정보는 사라져요.",
  "error.hardwareKeyNoPin": "먼저 제조사 도구로 이 키에 PIN을 설정하세요.",
  "error.hardwareKeyNotRecognized":
    "이 키는 여기에 등록되어 있지 않아요. 다른 키나 마스터 비밀번호를 사용하세요.",
  "error.hardwareKeyLimit": "허용되는 최대 개수(8개)의 키를 등록했어요. 먼저 하나를 제거하세요.",
  "error.hardwareKeyLast":
    "이 보관함의 유일한 키예요. 제거하기 전에 다른 키를 등록하세요.",
  "error.itemTypeNotAllowed": "이 보관함에는 일회용 코드만 넣을 수 있어요.",
  "error.otpScanUnsupported":
    "이 시스템에서는 화면 스캔을 사용할 수 없어요. 대신 스크린샷을 붙여넣으세요.",
  "error.otpQrNotFound": "QR 코드를 찾을 수 없어요. 코드 전체가 보이는지 확인한 다음 다시 시도하세요.",
  "error.otpQrNotOtp":
    "이 QR 코드는 2단계 인증 설정 코드가 아니에요. 사이트의 2단계 인증 설정 페이지에서 코드를 찾으세요.",
  "error.otpQrMigration":
    "Google Authenticator 내보내기 코드예요. Mosaic은 아직 읽을 수 없어요. 각 계정을 해당 사이트에서 다시 설정하거나 설정 키를 입력하세요.",
  "error.otpQrMultiple":
    "설정 코드가 둘 이상 보여요. 원하는 것 하나만 표시한 다음 다시 시도하세요.",
  "error.otpQrInvalid":
    "이 설정 코드는 Mosaic이 지원하지 않는 설정을 사용해요. 대신 설정 키를 입력하세요.",
  "error.otpUriInvalid": "올바른 otpauth:// 링크가 아니에요. 링크 전체를 다시 복사하세요.",
  "error.otpKeyInvalid":
    "올바르지 않은 키예요. 영문자 A~Z와 숫자 2~7만 사용해요. 오타가 없는지 확인하세요.",
  "error.otpImageUnreadable": "Mosaic이 이 이미지를 읽을 수 없어요. PNG 또는 JPEG 파일을 사용하세요.",
  "error.otpImageTooLarge": "이 이미지가 너무 커요. QR 코드만 담은 스크린샷을 사용하세요.",
  "error.otpClipboardNoImage":
    "클립보드에 이미지가 없어요. QR 코드의 스크린샷을 찍은 다음 여기에 붙여넣으세요.",
  "error.otpPendingExpired": "스캔한 코드를 더 이상 사용할 수 없어요. 다시 스캔하세요.",
  "error.syncNotConfigured": "이 기기에는 아직 동기화가 설정되어 있지 않아요. 먼저 설정에서 설정하세요.",
  "error.syncAlreadyConfigured":
    "동기화가 이미 설정되어 있어요. 다른 서버나 폴더를 사용하려면 먼저 연결을 해제하세요.",
  "error.syncServerUnreachable":
    "Mosaic이 서버에 연결할 수 없었어요. 주소와 인터넷 연결을 확인한 다음 다시 시도하세요.",
  "error.syncServerInvalid":
    "Mosaic 서버처럼 보이지 않아요. 주소를 확인하세요. https://sync.example.com 같은 형태여야 해요.",
  "error.syncServerInsecure":
    "Mosaic은 중간에서 아무도 데이터를 읽거나 바꿀 수 없도록 https로만 서버에 연결해요. https:// 주소를 사용하세요.",
  "error.syncServerMismatch": "이 계정이 동기화하는 서버가 아니에요. 주소를 확인하세요.",
  "error.syncServerIncompatible":
    "이 서버는 내 Mosaic과 맞지 않는 버전으로 실행되고 있어요. Mosaic을 업데이트하거나 서버 소유자에게 업데이트를 요청하세요.",
  "error.syncRegistrationClosed":
    "이 서버는 새 동기화 계정을 받지 않아요. 기존 계정으로 로그인하거나 서버 소유자에게 문의하세요.",
  "error.syncInviteInvalid":
    "이 초대 코드가 올바르지 않거나 이미 사용되었어요. 코드를 확인하거나 서버 소유자에게 새 코드를 요청하세요.",
  "error.syncAccountExists":
    "이 서버에 이미 동기화 계정이 있어요. 대신 로그인을 선택하세요.",
  "error.syncAccountMismatch":
    "그 서버의 동기화 계정은 다른 Mosaic 계정의 것이에요. 주소와 이메일을 확인하세요.",
  "error.syncTwoFactorRequired":
    "이 동기화 계정에는 인증 앱의 코드가 필요해요. 6자리 코드를 입력하세요.",
  "error.syncTwoFactorInvalid":
    "코드가 올바르지 않거나 만료되었어요. 인증 앱의 현재 6자리 코드를 입력하세요.",
  "error.syncSignedOut":
    "서버가 이 기기를 로그아웃시켰어요. 계속 동기화하려면 다시 로그인하세요. 변경 사항은 이 기기에 안전하게 있어요.",
  "error.syncRateLimited": "시도가 너무 많아요. 몇 분 기다린 다음 다시 시도하세요.",
  "error.syncQuotaExceeded":
    "서버나 폴더에 데이터를 저장할 공간이 남아 있지 않아요. 공간을 확보하거나 서버 소유자에게 더 요청하세요.",
  "error.syncFolderUnavailable":
    "Mosaic이 이 폴더를 사용할 수 없어요. 폴더가 있는지, 클라우드 서비스가 동기화를 마쳤는지, Mosaic이 쓸 수 있는지 확인하세요.",
  "error.syncFolderNotEmpty":
    "이 폴더에 다른 파일이 있어서 Mosaic이 쓰지 않아요. 빈 폴더를 선택하거나 새로 만드세요.",
  "error.syncFolderNoAccount":
    "이 폴더에는 이 계정의 동기화 데이터가 없어요. 다른 기기가 동기화하는 폴더를 선택하세요.",
  "error.syncFolderDamaged":
    "이 폴더의 일부 동기화 파일이 손상되었어요. 클라우드 서비스가 동기화를 마치게 하거나 백업에서 폴더를 복원한 다음 다시 시도하세요.",
  "error.syncPasswordOutcomeUnknown":
    "서버가 마스터 비밀번호를 변경했는지 확인할 수 없었어요. 동기화로 어느 쪽이 적용되는지 확인될 때까지 두 비밀번호를 모두 보관하세요.",
  "error.syncWebauthnUnavailable":
    "서버가 지금은 이 작업에 보안 키를 사용할 수 없거나 이 계정에 보안 키가 없어요. 대신 인증 앱 코드를 사용하세요.",
  "error.syncWebauthnLimit":
    "이 계정에는 서버가 허용하는 최대 개수의 보안 키가 이미 있어요. 더 이상 사용하지 않는 키를 제거한 다음 다시 시도하세요.",
  "error.syncHandoffExpired": "브라우저 단계의 시간이 지났어요. 다시 시작하세요.",
  "error.syncHandoffFailed":
    "브라우저에서 틀린 코드나 실패한 확인이 너무 많았어요. 단계를 다시 시작하세요.",
  "error.syncPasswordNeedsServer":
    "마스터 비밀번호를 변경하려면 동기화 서버가 필요한데 연결할 수 없어요. 인터넷 연결을 확인한 다음 다시 시도하세요.",
  "error.syncDeviceIsCurrent": "사용 중인 기기는 제거할 수 없어요. 대신 연결 해제를 사용하세요.",
  "error.syncChoiceExpired": "폴더 선택 시간이 지났어요. 폴더를 다시 선택하세요.",
  "error.syncFreshLoginExpired": "비밀번호 확인 시간이 지났어요. 마스터 비밀번호를 다시 입력하세요.",
  "error.syncPasswordRollback": "서버가 이전 마스터 비밀번호를 제시했어요. 서버를 확인하세요.",
  "error.syncPasswordChanged":
    "서버나 폴더에서 마스터 비밀번호가 변경되었어요. 아래에 새 비밀번호를 입력하세요.",
  "error.sharingNotHosted":
    "공유에는 Mosaic 서버가 필요해요. 먼저 동기화 설정에서 서버를 연결하세요.",
  "error.setupCodeNeedsServer": "휴대폰은 동기화되는 계정에만 참여할 수 있어요.",
  "error.sharingContactUnverified":
    "먼저 이 사람의 보안 코드를 비교하세요. Mosaic은 확인한 사람과만 공유해요.",
  "error.sharingKeyChanged":
    "이 사람의 키가 바뀌었거나 코드가 일치하지 않아요. 계속하기 전에 다시 조회하고 새 코드를 비교하세요.",
  "error.sharingRoleForbidden":
    "이 보관함에서의 내 역할로는 할 수 없어요. 소유자나 관리자에게 요청하세요.",
  "error.sharingInviteExists":
    "이 사람과 보관함에 대한 초대가 이미 대기 중이에요. 새로 보내려면 먼저 철회하세요.",
  "error.sharingInviteStale":
    "그사이 초대가 변경되었어요. 예를 들어 키 변경 후일 수 있어요. 다시 여세요.",
  "error.sharingInviteExpired":
    "이 초대는 만료되었거나 철회되었어요. 보낸 사람에게 다시 초대해 달라고 요청하세요.",
  "error.sharingInviteUnreadable":
    "이 초대를 열 수 없어요. 손상되었거나 이 계정용이 아닐 수 있어요. 보낸 사람에게 다시 초대해 달라고 요청하세요.",
  "error.sharingInviteNotInRoster":
    "보관함의 구성원 목록이 이 초대와 일치하지 않아서 Mosaic이 열지 않았어요. 보낸 사람에게 다시 초대해 달라고 요청하세요.",
  "error.sharingOwnerCannotLeave":
    "소유자는 보관함을 나갈 수 없어요. 대신 다른 사람들을 제거하세요.",
  "error.sharingAlreadyMember": "이 사람은 이미 보관함의 구성원이에요.",
  "error.sharingReinviteBlocked":
    "최근에 이 사람을 제거했어요. 보관함 키가 바뀐 후에 다시 초대할 수 있어요. 동기화한 다음 다시 시도하세요.",
  "error.sharingInviteLimit": "대기 중인 초대가 너무 많아요. 응답을 기다리거나 일부를 철회하세요.",
  "error.sharingVaultKnown":
    "이 기기에는 이미 그 보관함이 있어서 Mosaic이 그 보관함의 키를 받아들이지 않았어요. 보낸 사람에게 어떤 보관함을 공유했는지 물어보세요.",
  "error.syncHasSharedVaults":
    "이 기기에 공유 보관함이 있는 동안에는 서버를 바꿀 수 없어요. 먼저 공유 보관함에서 나가거나 소유자가 나를 제거하도록 하세요.",
  "error.emergencyWaiting":
    "대기 중인 요청이 있어서 지금은 변경할 수 없어요. 요청을 거부하거나 기다리세요.",
  "error.emergencyCooldown":
    "마지막 요청이 거부되었어요. 대기 기간인 하루가 지나면 다시 요청할 수 있어요.",
  "error.emergencyNotGranted":
    "요청할 것이 없어요. 열려 있는 요청이 없거나 상대가 접근 권한을 부여하지 않았어요.",
  "error.shortcutUnavailable":
    "다른 프로그램이 이미 이 단축키를 사용하고 있거나 Mosaic이 등록할 수 없어요. 다른 조합을 선택하세요.",
  "error.shortcutUnsupported":
    "이 시스템에서는 앱이 전역 단축키를 설정할 수 없어요. 대신 Mosaic 창에서 여세요.",
} satisfies Record<keyof typeof en, string>;
