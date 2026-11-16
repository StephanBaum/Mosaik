/**
 * Japanese UI strings: settings. Same keys as English (checked by the type).
 * ICU plural categories of Japanese: other (CLDR; `other` is required).
 */
import type { settings as en } from "../en/settings.ts";

export const settings = {
  "settings.title": "設定",
  "settings.security": "セキュリティ",
  "settings.autoLock": "無操作でロックするまでの時間",
  "settings.autoLock.never": "しない",
  "settings.autoLock.minutes": "{minutes, plural, other {#分}}",
  "settings.lockOnSleep": "コンピューターのスリープ時にロック",
  "settings.lockOnScreenLock": "画面のロック時にロック",
  "settings.screenLock.unsupported":
    "このシステムでは、Mosaic は画面がロックされたことを検出できません。無操作の場合と、コンピューターの復帰時には、引き続きロックされます。",
  "settings.lockTrigger.required":
    "ロックの条件を1つ以上オンにしてください。このシステムでは、Mosaic は画面のロックを検出できません。",
  "settings.clipboard": "クリップボードを消去するまでの時間",
  "settings.clipboard.never": "しない",
  "settings.clipboard.seconds": "{seconds, plural, other {#秒}}",
  "settings.appearance": "外観",
  "settings.theme": "テーマ",
  "settings.theme.system": "システムに合わせる",
  "settings.theme.light": "ライト",
  "settings.theme.dark": "ダーク",
  "settings.language": "言語",
  "settings.language.system": "システムに合わせる",
  "settings.language.machineNote": "AI の支援で翻訳しています。不自然な箇所があれば、お知らせください。",
  "settings.import": "読み込み",
  "settings.import.body":
    "ブラウザや他のパスワードマネージャーからログインを引き継ぎます。Mosaic がエクスポートの手順を案内し、その後のファイル削除もサポートします。",
  "settings.import.open": "パスワードを読み込む",
  "settings.ssh": "SSH エージェント",
  "settings.ssh.enable": "Mosaic の SSH キーを使用",
  "settings.ssh.hint":
    "初期設定ではオフです。エージェントは Mosaic のロックが解除されている間のみ動作し、使用のたびに確認します。",
  "settings.ssh.status": "{count, plural, =0 {提供中のキーなし} other {#個のキーを提供中}}",
  "settings.ssh.endpoint": "SSH_AUTH_SOCK の設定値",
  "settings.ssh.skipped": "提供されていないキー",
  "settings.kit": "緊急キット",
  "settings.kit.save": "緊急キットをもう一度保存",
  "settings.saved": "設定を保存しました。",
  "reauth.title": "マスターパスワードで確認",
  "reauth.body": "この変更は Mosaic の安全性を下げるため、マスターパスワードが必要です。",
  "reauth.bodyKit":
    "シークレットキーをファイルに表示しようとしているため、Mosaic にはマスターパスワードが必要です。",
  "reauth.password": "マスターパスワード",
  "reauth.submit": "確認",
  "settings.intro": "このデバイスでの、Mosaic のロック、シークレットの消去、キーの保護の方法です。",
  "settings.section.autoLock": "自動ロック",
  "settings.section.autoLock.body": "操作しなくても Mosaic が自動でロックされるタイミングです。",
  "settings.section.clipboard": "クリップボード",
  "settings.section.clipboard.body": "コピーしたシークレットは、この時間が経つと消去されます。",
  "settings.appearance.body": "Mosaic のテーマと言語です。",
  "settings.ssh.body": "プログラムが、保管庫の SSH キーでサインインできるようにします。使用のたびに承認します。",
  "settings.kit.body": "緊急キットの新しいコピーを保存します。マスターパスワードが必要です。",
  "settings.quick": "クイックロック解除",
  "settings.quick.body": "マスターパスワードに加えて、より手早く戻る方法です。",
  "settings.quick.windowsHello": "Windows Hello",
  "settings.quick.touchId": "Touch ID",
  "settings.quick.biometric.body":
    "顔、指紋、または PIN でロックを解除します。再起動後は、引き続きマスターパスワードが必要です。",
  "settings.quick.biometric.bodyMac":
    "指紋でロックを解除します。再起動後は、引き続きマスターパスワードが必要です。",
  "settings.quick.expires": "Mosaic は {date, date, medium} に、もう一度マスターパスワードを求めます。",
  "settings.quick.maxAge": "マスターパスワードを求める間隔（最長）",
  "settings.quick.maxAgeDays": "{days, plural, other {#日}}",
  "settings.quick.biometricLinux": "Linux では生体認証によるロック解除は使えません。",
  "settings.quick.biometricUnavailable": "このデバイスでは設定されていません。",
  "settings.quick.keys": "セキュリティキー",
  "settings.quick.keysCount": "{count, plural, =0 {登録なし} other {#個登録済み}}",
  "settings.quick.keysUnsupported": "このシステムでは、セキュリティキーにはまだ対応していません。",
  "settings.quick.keyRow": "セキュリティキー · {tag} · 追加日: {addedAt, date, medium}",
  "settings.quick.addKey": "セキュリティキーを追加",
  "settings.quick.addKeyBody":
    "追加するキーを挿入し、PIN を入力してください。その後、キーが点滅したら触れてください。",
  "settings.quick.manageKeys": "管理",
  "settings.quick.removeKey": "削除",
  "settings.quick.removeKeyNamed": "セキュリティキー {tag} を削除",
  "settings.quick.removeKeyConfirm": "セキュリティキー {tag} を削除しますか？",
  "settings.quick.keyRisk":
    "このキーとその PIN を持つ人は、マスターパスワードなしで保管庫を開けます。",
  "reauth.bodyQuick": "Mosaic に入りやすくなるため、マスターパスワードが必要です。",
  "reauth.bodyKey": "この保管庫にキーを追加するには、マスターパスワードが必要です。",
  "settings.passkeyExport": "パスキーをエクスポート",
  "settings.passkeyExport.body":
    "別のアプリに移すため、保管庫のパスキーを FIDO CXF ファイルに保存します。",
  "settings.passkeyExport.vault": "保管庫",
  "settings.passkeyExport.warning":
    "このファイルには、パスキーが暗号化されずに含まれます。ファイルを持つ人は、あなたとしてサインインできます。読み込んだ後は削除してください。",
  "settings.passkeyExport.none": "この保管庫には、エクスポートできるパスキーがありません。",
  "settings.passkeyExport.button": "パスキーをエクスポート",
  "settings.passkeyExport.reauth":
    "ファイルにはパスキーが暗号化されずに含まれるため、エクスポートにはマスターパスワードが必要です。",
  "settings.passkeyExport.saved":
    "{count, plural, other {#個のパスキーをエクスポートしました。}}読み込んだ後は、ファイルを削除してください。",
  "reauth.bodyMove":
    "セキュリティキーで保護された保管庫から項目を移動するには、マスターパスワードが必要です。",
  "toast.settingsReset":
    "セキュリティ設定が Mosaic の外で変更されたため、安全な初期値にリセットされました。",
  "settings.shortcut.title": "クイック検索のショートカット",
  "settings.shortcut.body": "どのプログラムからでも、この組み合わせを押すと Mosaic の検索が開きます。",
  "settings.shortcut.label": "ショートカット",
  "settings.shortcut.hint":
    "入力欄をクリックしてから、使いたいキーを押してください。Ctrl、Alt、または ⌘（Mac の場合）に、もう1つキーを組み合わせます。",
  "settings.shortcut.save": "ショートカットを保存",
  "settings.shortcut.reset": "初期設定に戻す",
  "settings.shortcut.saved": "ショートカットを保存しました: {shortcut}",
  "settings.shortcut.invalid":
    "Mosaic で使用できないショートカットです。Ctrl、Alt、または ⌘（Mac の場合）に、もう1つキーを組み合わせて押してください。",
  "settings.shortcut.unsupported":
    "お使いのシステムではアプリがグローバルショートカットを設定できないため、ここでは変更できません。",
} satisfies Record<keyof typeof en, string>;
