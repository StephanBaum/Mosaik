/**
 * Korean UI strings: ext. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { ext as en } from "../en/ext.ts";

export const ext = {
  "ext.manifest.name": "Mosaic",
  "ext.manifest.description":
    "Mosaic 보관함의 로그인을 자동으로 입력해요. 마스터 비밀번호는 데스크톱 앱에만 있어요.",
  "ext.command.fill": "Mosaic으로 로그인 자동 입력",
  "ext.command.generate": "Mosaic으로 비밀번호 생성",
  "ext.save.badgeTitle": "로그인을 저장할까요?",
  "ext.loading": "불러오는 중…",
  "ext.site.secure": "현재 사이트, 보안 연결 (https): {host}",
  "ext.site.insecure": "현재 사이트, 암호화되지 않음 (http): {host}",
  "ext.site.none": "이 탭에는 웹사이트가 없어요",
  "ext.nav.label": "보기",
  "ext.nav.logins": "로그인 및 검색",
  "ext.nav.generator": "비밀번호 생성기",
  "ext.footer.open": "Mosaic 열기",
  "ext.footer.more": "더 많은 옵션",
  "ext.footer.connected": "Mosaic에 연결됨",
  "ext.footer.locked": "Mosaic이 잠겨 있어요",
  "ext.footer.offline": "연결되지 않음",
  "ext.more.saveOffers": "로그인 저장 제안",
  "ext.more.saveOffersHint":
    "확장 프로그램이 제출한 로그인을 알아차리려면 모든 웹사이트에 대한 접근 권한이 필요해요. Mosaic 앱에서 확인하기 전에는 아무것도 저장되지 않아요.",
  "ext.passkeys.toggle": "패스키에 Mosaic 사용",
  "ext.passkeys.hint":
    "패스키 요청에 응답하려면 모든 웹사이트에 대한 접근 권한이 필요해요. 각 요청은 Mosaic 앱에서 확인해요.",
  "ext.passkeys.permissionDenied":
    "패스키 요청에 응답하려면 Mosaic에 웹사이트 접근 권한이 필요한데 허용하지 않았어요.",
  "ext.more.disconnect": "Mosaic 연결 해제",
  "ext.more.disconnectTitle": "이 브라우저의 연결을 해제할까요?",
  "ext.more.disconnectBody":
    "다시 연결하기 전에는 이 브라우저가 로그인을 자동으로 입력할 수 없어요. 새 연결은 Mosaic 앱에서 승인해요.",
  "ext.action.cancel": "취소",
  "ext.neverSave.menu": "저장을 제안하지 않는 사이트…",
  "ext.neverSave.title": "저장을 제안하지 않음",
  "ext.neverSave.intro":
    "Mosaic은 이 사이트들에서 로그인 저장을 제안하지 않아요. 사이트를 제거하면 그곳에서 다시 물어봐요.",
  "ext.neverSave.empty":
    "아직 사이트가 없어요. Mosaic이 로그인 저장을 제안할 때 “이 사이트에서는 안 함”을 선택해 추가할 수 있어요.",
  "ext.neverSave.remove": "제거",
  "ext.neverSave.removeLabel": "목록에서 {origin} 제거",
  "ext.neverSave.removed": "{origin}을(를) 제거했어요. Mosaic이 그곳에서 다시 로그인 저장을 제안할 수 있어요.",
  "ext.neverSave.close": "닫기",
  "ext.connect.checking": "Mosaic에 연결하는 중…",
  "ext.connect.hostMissing.title": "이 브라우저에는 Mosaic이 설정되어 있지 않아요",
  "ext.connect.hostMissing.body":
    "이 컴퓨터에 Mosaic 앱을 설치한 다음 브라우저를 다시 시작하세요.",
  "ext.connect.hostMissing.dev": "개발자: {command}을(를) 실행한 다음 브라우저를 다시 시작하세요.",
  "ext.connect.hostForbidden.title": "이 브라우저는 연결할 수 없어요",
  "ext.connect.hostForbidden.body":
    "Mosaic은 공식 Mosaic 확장 프로그램과만 통신해요. 브라우저 스토어에서 설치하세요.",
  "ext.connect.desktopNotRunning.title": "이 컴퓨터에서 Mosaic을 여세요",
  "ext.connect.desktopNotRunning.body":
    "확장 프로그램은 Mosaic 앱을 통해 작동해요. 앱을 시작한 다음 다시 확인하세요.",
  "ext.connect.check": "다시 확인",
  "ext.connect.notPaired.title": "Mosaic에 연결",
  "ext.connect.notPaired.body":
    "이 브라우저를 이 컴퓨터의 Mosaic 앱에 연결하세요. 앱에서 승인해요.",
  "ext.connect.start": "연결",
  "ext.connect.pairing.title": "Mosaic에서 확인",
  "ext.connect.pairing.body": "Mosaic에 같은 코드가 표시되는지 확인한 다음 그곳에서 허용을 클릭하세요.",
  "ext.connect.pairing.code": "코드",
  "ext.connect.pairing.waiting": "Mosaic 앱에서 응답을 기다리는 중…",
  "ext.connect.protocolMismatch.title": "Mosaic 업데이트",
  "ext.connect.protocolMismatch.body":
    "확장 프로그램과 Mosaic 앱의 버전이 서로 달라요. 둘 다 최신 버전으로 업데이트하세요.",
  "ext.connect.error.title": "Mosaic에 연결할 수 없어요",
  "ext.connect.error.body": "Mosaic 앱이 실행 중인지 확인한 다음 다시 시도하세요.",
  "ext.locked.title": "Mosaic이 잠겨 있어요",
  "ext.locked.body": "{host}에서 자동 입력하려면 Mosaic 앱에서 잠금을 해제하세요.",
  "ext.locked.bodyNoSite": "여기에서 사용하려면 Mosaic 앱에서 잠금을 해제하세요.",
  "ext.locked.open": "Mosaic 열기",
  "ext.locked.note": "마스터 비밀번호는 Mosaic 앱에서만 입력해요.",
  "ext.logins.forSite": "{host}용",
  "ext.logins.count": "{count, plural, other {로그인 #개}}",
  "ext.logins.sameSite": "같은 사이트",
  "ext.logins.none": "{host}에 저장된 로그인이 없어요.",
  "ext.logins.noneHint": "Mosaic은 전체 주소를 비교하므로 비슷하게 생긴 사이트는 일치하지 않아요.",
  "ext.logins.unsupported": "이 페이지에서는 Mosaic이 자동 입력할 수 없어요. 보관함을 검색해 로그인을 복사하세요.",
  "ext.logins.shortcut": "{shortcut}을(를) 누르면 첫 번째 일치 항목을 입력해요.",
  "ext.logins.frameNote":
    "Mosaic은 {host}에서만 자동 입력하며, 다른 사이트에서 삽입된 프레임 안에서는 입력하지 않아요.",
  "ext.item.untitled": "제목 없음",
  "ext.item.noUsername": "사용자 이름 없음",
  "ext.item.twoFactor": "2단계 인증",
  "ext.item.fill": "입력",
  "ext.item.fillLabel": "{name} 입력",
  "ext.item.copyMenu": "{name}의 복사 옵션",
  "ext.item.copyUsername": "사용자 이름 복사",
  "ext.item.copyPassword": "비밀번호 복사",
  "ext.item.copyCode": "코드 복사",
  "ext.item.blocked": "차단됨: 이 페이지는 암호화되지 않았고 이 로그인은 이를 허용하지 않아요.",
  "ext.search.label": "다른 항목",
  "ext.search.placeholder": "보관함 검색",
  "ext.search.note":
    "결과는 이 팝업 안에만 있어요. 페이지는 입력하는 로그인 하나만 받아요.",
  "ext.search.none": "찾을 수 없어요.",
  "ext.search.copy": "복사",
  "ext.search.copyOnly": "복사만 가능: 이 사이트용으로 저장되지 않음",
  "ext.shortcut.copyUser": "{platform, select, mac {⌘C} other {Ctrl+C}}",
  "ext.shortcut.copyPassword": "{platform, select, mac {⇧⌘C} other {Ctrl+Shift+C}}",
  "ext.shortcut.copyCode": "{platform, select, mac {⌥⌘C} other {Ctrl+Alt+C}}",
  "ext.search.results": "{count, plural, other {결과 #개}}",
  "ext.frame.cross":
    "포커스가 있는 입력란이 다른 사이트의 프레임 안에 있어요. Mosaic은 그곳에 입력하지 않아요.",
  "ext.frameConfirm.title": "삽입된 프레임 안에 입력할까요?",
  "ext.frameConfirm.body":
    "입력란이 페이지 자체가 아니라 {host}의 프레임 안에 있어요. 이 프레임을 신뢰할 때만 여기에 입력하세요.",
  "ext.frameConfirm.confirm": "프레임에 입력",
  "ext.warning.idn.title": "주소를 확인하세요",
  "ext.warning.idn.body":
    "이 주소는 특수 문자를 사용해서 다른 사이트처럼 보일 수 있어요. 로그인하기 전에 주소 표시줄을 확인하세요.",
  "ext.warning.lookalike.body":
    "이 사이트는 {saved}이(가) 아니에요. 비슷하게 생겼는데, 흔한 피싱 수법이에요. 이 탭을 닫거나 {saved}을(를) 직접 입력하세요.",
  "ext.warning.youAreOn": "현재 사이트",
  "ext.warning.writtenAs": "표기",
  "ext.warning.savedFor": "저장된 사이트",
  "ext.warning.insecure.title": "이 페이지는 암호화되지 않았어요",
  "ext.warning.insecure.body":
    "이 페이지는 http를 사용해요. 같은 네트워크의 다른 사람이 비밀번호를 읽을 수 있어요. Mosaic은 허용한 로그인에 대해서만 여기에 입력해요.",
  "ext.otp.title": "일회용 코드 입력",
  "ext.otp.next": "{name}의 비밀번호를 입력했어요. 다음은 2단계 인증 코드예요.",
  "ext.otp.for": "{name}의 코드",
  "ext.otp.fill": "코드 입력",
  "ext.otp.copy": "코드 복사",
  "ext.otp.note": "코드는 Mosaic에서 입력란으로 바로 들어가요. 이 팝업에는 표시되지 않아요.",
  "ext.save.titleNew": "{host}의 이 로그인을 저장할까요?",
  "ext.save.titleUpdate": "{host}에 저장된 로그인을 업데이트할까요?",
  "ext.save.generated": "Mosaic이 {host}용으로 생성한 비밀번호예요.",
  "ext.save.updates": "“{title}”을(를) 업데이트해요.",
  "ext.save.username": "사용자 이름",
  "ext.save.noUsername": "없음",
  "ext.save.password": "비밀번호",
  "ext.save.passwordHidden": "숨김. Mosaic 앱에서 볼 수 있어요.",
  "ext.save.insecure": "이 페이지는 암호화되지 않았어요 (http).",
  "ext.save.action": "Mosaic에 저장…",
  "ext.save.notNow": "나중에",
  "ext.save.never": "이 사이트에서는 안 함",
  "ext.save.waiting": "Mosaic 앱에서 확인하세요. 확인하기 전에는 아무것도 저장되지 않아요.",
  "ext.save.saved": "Mosaic에 저장했어요.",
  "ext.save.updated": "Mosaic에서 업데이트했어요.",
  "ext.save.cancelled": "저장하지 않았어요.",
  "ext.save.dismissed": "취소했어요. 비밀번호는 잊어버려요.",
  "ext.save.neverDone": "Mosaic이 {host}에서 다시는 저장을 제안하지 않아요.",
  "ext.save.permissionAsk":
    "Mosaic이 브라우저에 웹사이트 접근 권한을 요청해요. 이 기능을 켜려면 허용하세요.",
  "ext.generator.title": "{host}용 비밀번호 생성",
  "ext.generator.titleGeneric": "비밀번호 생성",
  "ext.generator.value": "생성된 비밀번호",
  "ext.generator.regenerate": "다른 비밀번호 생성",
  "ext.generator.type": "비밀번호 유형",
  "ext.generator.random": "무작위",
  "ext.generator.passphrase": "암호 문구",
  "ext.generator.include": "포함",
  "ext.generator.rules.page": "이 페이지의 비밀번호 규칙을 따라요",
  "ext.generator.rules.siteQuirks": "이 사이트의 알려진 규칙을 따라요",
  "ext.generator.fill": "입력",
  "ext.generator.fillHint":
    "입력하려면 페이지의 새 비밀번호 입력란을 클릭한 다음 이 팝업을 다시 여세요.",
  "ext.generator.copy": "복사",
  "ext.generator.expired": "그 비밀번호는 더 이상 보관되어 있지 않아서 새로 생성했어요.",
  "ext.generator.filled": "입력했어요. Mosaic이 저장을 제안할 거예요.",
  "ext.copied": "복사했어요. {seconds, plural, other {#초}} 후 지워져요.",
  "ext.copiedNoClear": "복사했어요.",
  "ext.filled": "입력했어요.",
  "ext.filledCode": "코드를 입력했어요.",
  "ext.error.invalidRequest":
    "Mosaic이 이 요청을 이해할 수 없었어요. 확장 프로그램과 Mosaic 앱을 업데이트하세요.",
  "ext.error.notPaired": "이 브라우저는 Mosaic에 연결되어 있지 않아요. 다시 연결하세요.",
  "ext.error.protocolVersion":
    "확장 프로그램과 Mosaic 앱의 버전이 서로 달라요. 둘 다 업데이트하세요.",
  "ext.error.busy": "Mosaic이 다른 요청을 처리하는 중이에요. 잠시 후 다시 시도하세요.",
  "ext.error.locked": "Mosaic이 잠겨 있어요. Mosaic 앱에서 잠금을 해제하세요.",
  "ext.error.rateLimited":
    "짧은 시간에 요청이 너무 많았어요. 잠시 기다린 다음 다시 시도하세요.",
  "ext.error.originMismatch":
    "이 로그인은 이 웹사이트용으로 저장되어 있지 않아서 Mosaic이 여기에 입력하지 않아요.",
  "ext.error.insecureOrigin":
    "이 페이지는 암호화되지 않았고 로그인이 이 페이지에 입력하는 것을 허용하지 않아요. 아무것도 입력하지 않았어요.",
  "ext.error.crossOriginFrame":
    "그 입력란은 다른 사이트의 프레임 안에 있어요. Mosaic은 그곳에 입력하지 않아요.",
  "ext.error.itemNotFound": "이 항목은 더 이상 없어요. 이동되었거나 삭제되었을 수 있어요.",
  "ext.error.vaultUnavailable": "그 보관함이 닫혀 있어요. 먼저 Mosaic 앱에서 여세요.",
  "ext.error.noOtp": "이 항목에는 일회용 코드가 없어요.",
  "ext.error.otpNotFillable":
    "이 코드는 사용할 때마다 증가해요(HOTP). 여기에서는 입력하거나 복사할 수 없어요. Mosaic 앱에서 사용하세요.",
  "ext.error.fieldEmpty": "이 필드가 비어 있어서 입력하거나 복사할 내용이 없어요.",
  "ext.error.clipboardUnavailable":
    "Mosaic이 클립보드를 사용할 수 없었어요. 다시 시도하거나 Mosaic 앱에서 복사하세요.",
  "ext.error.cancelled": "Mosaic 앱에서 취소했어요. 준비되면 다시 시도하세요.",
  "ext.error.passkeyNotFound": "이 사이트의 패스키가 Mosaic에 없어요.",
  "ext.error.passkeyExcluded": "Mosaic에 이 계정의 패스키가 이미 있어요.",
  "ext.error.rpIdMismatch":
    "이 사이트는 그 도메인의 패스키를 사용할 수 없어서 Mosaic이 응답하지 않았어요.",
  "ext.error.passkeyUnsupported":
    "이 사이트가 요청한 옵션으로는 Mosaic이 패스키를 만들 수 없어요.",
  "ext.error.pairingDenied":
    "Mosaic 앱에서 연결이 거부되었어요. 연결을 선택해 다시 시도한 다음 허용을 선택하세요.",
  "ext.error.pairingExpired":
    "연결 요청 시간이 지났어요. 연결을 선택해 다시 시작하세요.",
  "ext.error.pairingConflict": "다른 연결 요청이 열려 있었어요. 연결을 선택해 다시 시도하세요.",
  "ext.error.pairingBlocked": "거부된 연결이 너무 많아요. 10분 기다린 다음 다시 시도하세요.",
  "ext.error.pairingLimit":
    "이 컴퓨터에 연결된 브라우저가 너무 많아요. Mosaic 설정에서 하나를 연결 해제하세요.",
  "ext.error.internal":
    "Mosaic에서 예기치 않은 문제가 발생했어요. 다시 시도하고, 반복되면 Mosaic 앱을 다시 시작하세요.",
  "ext.error.desktopNotRunning": "이 컴퓨터에서 Mosaic이 실행 중이 아니에요. 실행한 다음 다시 시도하세요.",
  "ext.error.desktopNotAuthentic":
    "응답한 앱은 이 브라우저가 연결된 Mosaic 앱이 아니에요. 아무것도 전송하지 않았어요.",
  "ext.error.timeout":
    "Mosaic이 제때 응답하지 않았어요. Mosaic 앱에 열려 있는 대화 상자가 있다면 그곳에서 응답하세요.",
  "ext.error.frameTooLarge": "Mosaic으로 보내기에는 데이터가 너무 많았어요.",
  "ext.error.hostMissing":
    "이 브라우저에는 Mosaic 도우미가 설치되어 있지 않아요. Mosaic 앱을 설치한 다음 브라우저를 다시 시작하세요.",
  "ext.error.hostForbidden": "이 브라우저나 확장 프로그램은 Mosaic과 통신할 수 없어요.",
  "ext.error.hostExited": "Mosaic과의 연결이 닫혔어요. 다시 시도하세요.",
  "ext.error.unsupportedPage": "이런 종류의 페이지에서는 Mosaic이 자동 입력할 수 없어요.",
  "ext.error.noFields": "여기에서 로그인 입력란을 찾을 수 없어요. 입력란을 클릭한 다음 다시 시도하세요.",
  "ext.error.noMatch": "이 사이트에 저장된 로그인이 Mosaic에 없어요.",
  "ext.error.frameNeedsConfirmation":
    "입력란이 프레임 안에 있어요. Mosaic 팝업을 열고 프레임을 확인해야 입력할 수 있어요.",
  "ext.error.frameOpaque": "입력란이 샌드박스 프레임 안에 있어요. Mosaic은 그곳에 입력하지 않아요.",
  "ext.error.originChanged":
    "Mosaic이 입력하는 동안 페이지가 바뀌었어요. 아무것도 입력하지 않았어요. 다시 시도하세요.",
  "ext.error.pageHidden": "페이지가 보이지 않아서 Mosaic이 입력하지 않았어요.",
  "ext.error.permissionDenied":
    "로그인을 알아차리려면 Mosaic에 웹사이트 접근 권한이 필요한데 허용하지 않았어요.",
  "ext.error.generatedExpired": "Mosaic이 더 이상 그 비밀번호를 보관하고 있지 않아요. 새로 생성하세요.",
  "ext.rotation.card.title": "비밀번호 변경",
  "ext.rotation.card.for": "{name}용",
  "ext.rotation.card.started": "{when}에 시작됨",
  "ext.rotation.card.delivered": "새 비밀번호를 이미 한 번 입력했어요.",
  "ext.rotation.card.note":
    "새 비밀번호는 Mosaic 앱에서만 표시돼요. Mosaic은 양식을 대신 제출하지 않아요.",
  "ext.rotation.card.list": "진행 중인 비밀번호 변경",
  "ext.rotation.change": "비밀번호 변경…",
  "ext.rotation.changeLabel": "{name}의 비밀번호 변경…",
  "ext.rotation.openPage": "변경 페이지 열기",
  "ext.rotation.fillChange": "변경 양식 채우기",
  "ext.rotation.fillPending": "새 비밀번호를 입력해 로그인",
  "ext.rotation.worked": "작동함",
  "ext.rotation.failed": "작동 안 함",
  "ext.rotation.fresh": "새 비밀번호로 다시 시작",
  "ext.rotation.waiting": "Mosaic 앱에서 응답을 기다리는 중…",
  "ext.rotation.started": "시작했어요. Mosaic 앱에서 이 비밀번호 사용을 선택하세요.",
  "ext.rotation.startCancelled": "아무것도 시작하지 않았어요.",
  "ext.rotation.filled": "변경 양식을 채웠어요. 확인한 다음 직접 제출하세요.",
  "ext.rotation.opened": "변경 페이지를 열었어요.",
  "ext.rotation.confirmed": "완료했어요. 새 비밀번호가 이제 Mosaic에 저장되어 있어요.",
  "ext.rotation.keptOld": "Mosaic이 이전 비밀번호를 유지했어요.",
  "ext.rotation.resultCancelled": "아무것도 바뀌지 않았어요. 변경은 아직 열려 있어요.",
  "ext.rotation.offer.title": "관련 주소",
  "ext.rotation.offer.body":
    "이 페이지는 {title}에 저장되지 않은 {host}에 있어요. Mosaic이 그곳에 입력할 수 있도록 추가할까요?",
  "ext.rotation.offer.add": "{host} 추가",
  "ext.rotation.offer.added": "추가했어요. 이제 변경 내용이 여기에 표시돼요.",
  "ext.rotation.offer.cancelled": "추가하지 않았어요.",
  "ext.error.rotationNotFound":
    "이 비밀번호 변경은 더 이상 열려 있지 않아요. 로그인에서 새로 시작하세요.",
  "ext.error.originNotRelated":
    "이 주소는 이 로그인과 같은 사이트에 있지 않아서 Mosaic이 추가하지 않았어요.",
  "ext.error.rotationLimit":
    "이 로그인에는 끝나지 않은 변경이 이미 4개 있어요. 먼저 Mosaic 앱에서 하나를 끝내거나 취소하세요.",
  "ext.error.changeFormNotFound":
    "여기에서 Mosaic이 비밀번호 변경 양식을 찾지 못했어요. 계정의 비밀번호 페이지를 연 다음 다시 시도하세요.",
  "ext.error.changeFormAmbiguous":
    "Mosaic이 어느 입력란이 무엇인지 구분할 수 없었어요. Mosaic에서 새 비밀번호를 복사해 직접 붙여넣으세요.",
} satisfies Record<keyof typeof en, string>;
