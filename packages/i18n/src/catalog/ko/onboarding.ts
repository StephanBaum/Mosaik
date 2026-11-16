/**
 * Korean UI strings: onboarding. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { onboarding as en } from "../en/onboarding.ts";

export const onboarding = {
  "onboarding.secretKey.title": "내 비밀 키",
  "onboarding.secretKey.body":
    "비밀 키와 마스터 비밀번호가 함께 있어야 보관함을 열 수 있어요. 둘 중 어느 것도 복구해 드릴 수 없어요.",
  "onboarding.secretKey.verify": "비밀 키를 저장했는지 확인하려면 비밀 키의 마지막 그룹을 입력하세요.",
  "onboarding.secretKey.mistyped":
    "일치하지 않아요. 비상 키트의 마지막 그룹을 확인하세요.",
  "onboarding.step": "{total}단계 중 {current}단계",
  "onboarding.create.title": "계정 만들기",
  "onboarding.create.body":
    "마스터 비밀번호와 이 기기에서 생성하는 비밀 키가 함께 있어야 보관함을 열 수 있어요. 둘 다 서버로 전송되지 않아요.",
  "onboarding.create.email": "이메일",
  "onboarding.create.password": "마스터 비밀번호",
  "onboarding.create.passwordHint": "서로 관련 없는 단어 네 개가 좋아요. Mosaic에서만 사용하세요.",
  "onboarding.create.confirm": "마스터 비밀번호 확인",
  "onboarding.create.mismatch":
    "두 비밀번호가 일치하지 않아요. 두 입력란에 같은 마스터 비밀번호를 입력하세요.",
  "onboarding.create.submit": "계정 만들기",
  "onboarding.create.working": "계정을 만드는 중이에요. 몇 초 걸려요.",
  "onboarding.kit.accountId": "계정 ID",
  "onboarding.kit.secretKey": "비밀 키",
  "onboarding.kit.lastGroup": "비밀 키의 마지막 그룹",
  "onboarding.kit.confirm": "확인",
  "kit.title": "비상 키트 저장",
  "kit.body":
    "키트는 비밀 키가 들어 있는 PDF예요. 마스터 비밀번호와 함께 있으면 새 기기에서 보관함을 열 수 있어요. 이 컴퓨터와 떨어진 안전한 곳에 보관하세요.",
  "kit.secretKey": "내 비밀 키",
  "kit.save": "비상 키트 저장 (PDF)",
  "kit.saved": "저장했어요. 파일을 안전한 곳에 보관하세요.",
  "kit.cancelled": "아직 저장하지 않았어요. 계속하려면 키트를 저장하세요.",
  "kit.recover.alreadySaved": "이미 저장했어요",
  "kit.recover.title": "비상 키트 다시 저장",
  "kit.recover.body":
    "아직 비상 키트를 확인하지 않았어요. 마스터 비밀번호를 입력해 다시 저장한 다음 비밀 키의 마지막 그룹을 확인하세요.",
  "onboarding.import.title": "비밀번호 가져오기",
  "onboarding.import.body":
    "Chrome, Edge, Firefox, Safari 등의 브라우저나 1Password, Bitwarden, LastPass, Dashlane, Keeper, NordPass, Proton Pass, RoboForm, KeePass, Enpass에서 가져올 수 있어요. 몇 분 걸려요. 나중에 설정에서 해도 돼요.",
  "onboarding.import.start": "다른 앱에서 가져오기",
  "onboarding.import.skip": "나중에",
  "onboarding.done.title": "준비를 마쳤어요",
  "onboarding.done.body": "보관함이 준비되었어요. 잠금 버튼으로 언제든 잠글 수 있어요.",
  "onboarding.browser.title": "브라우저 확장 프로그램 설치",
  "onboarding.browser.body":
    "브라우저에서 로그인을 자동으로 입력해요. 확장 프로그램은 클릭하거나 단축키를 사용할 때만 입력해요.",
  "onboarding.browser.steps": "브라우저 연결 방법",
  "onboarding.browser.step1": "브라우저에 Mosaic 확장 프로그램을 설치하세요.",
  "onboarding.browser.step2": "확장 프로그램을 열고 연결을 선택하세요.",
  "onboarding.browser.step3":
    "코드가 일치하는지 확인한 다음 여기에서 열리는 창에서 허용을 선택하세요.",
  "onboarding.browser.notInStores":
    "확장 프로그램이 아직 브라우저 스토어에 없어요. 나중에 설정 → 브라우저에서 연결할 수 있어요.",
  "onboarding.browser.getFor": "{browser}용 받기",
  "onboarding.browser.connectedList": "연결된 브라우저",
  "onboarding.browser.waiting": "확장 프로그램을 기다리는 중이에요. 아직 연결된 브라우저가 없어요.",
  "onboarding.browser.connected": "{name} 연결됨",
  "onboarding.browser.done": "완료",
  "onboarding.browser.skip": "나중에",
  "onboarding.progress": "설정 진행 상황",
  "onboarding.stepName.create": "계정 만들기",
  "onboarding.stepName.kit": "비상 키트",
  "onboarding.stepName.verify": "키트 확인",
  "onboarding.stepName.import": "가져오기 (선택)",
  "onboarding.stepName.browser": "브라우저 확장 프로그램 (선택)",
  "kit.preview": "비상 키트 미리 보기",
  "kit.continueHint": "계속하려면 키트를 저장하세요.",
  "kit.warning":
    "이 키트와 마스터 비밀번호가 없으면 누구도 보관함을 복구할 수 없어요. 저희도 마찬가지예요.",
  "onboarding.join.link": "이미 Mosaic을 사용하고 있어요",
  "onboarding.join.title": "이 기기를 내 계정에 추가",
  "onboarding.join.body":
    "마스터 비밀번호와 비상 키트의 비밀 키로 로그인하세요. 그러면 보관함이 이 기기로 다운로드되어 복호화돼요.",
  "onboarding.join.tabs": "동기화 데이터는 어디에 있나요?",
  "onboarding.join.tab.server": "Mosaic 서버",
  "onboarding.join.tab.folder": "클라우드 폴더",
  "onboarding.join.serverUrl": "서버 주소",
  "onboarding.join.email": "동기화 계정 이메일",
  "onboarding.join.password": "마스터 비밀번호",
  "onboarding.join.secretKey": "비밀 키",
  "onboarding.join.secretKeyHint": "비상 키트에 있어요. M1로 시작해요.",
  "onboarding.join.deviceName": "이 기기의 이름",
  "onboarding.join.chooseFolder": "폴더 선택…",
  "onboarding.join.folder": "폴더",
  "onboarding.join.noAccounts":
    "이 폴더에는 Mosaic 계정이 없어요. 다른 기기가 동기화하는 폴더를 선택하세요.",
  "onboarding.join.account": "계정",
  "onboarding.join.accountOption": "계정 {id}",
  "onboarding.join.submit": "이 기기 추가",
  "onboarding.join.working": "로그인하고 보관함을 다운로드하는 중…",
  "onboarding.join.twoFactor.title": "코드 입력",
  "onboarding.join.twoFactor.body":
    "동기화 계정에 2단계 인증을 사용하고 있어요. 인증 앱의 6자리 코드를 입력하세요.",
  "onboarding.join.twoFactor.code": "6자리 코드",
  "onboarding.join.twoFactor.submit": "확인",
  "onboarding.join.back": "대신 새 계정 만들기",
} satisfies Record<keyof typeof en, string>;
