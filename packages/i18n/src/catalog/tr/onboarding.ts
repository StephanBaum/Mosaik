/**
 * Turkish UI strings: onboarding. Same keys as English (checked by the type).
 * ICU plural categories of Turkish: one, other (CLDR; `other` is required).
 */
import type { onboarding as en } from "../en/onboarding.ts";

export const onboarding = {
  "onboarding.secretKey.title": "Gizli Anahtarınız",
  "onboarding.secretKey.body":
    "Gizli Anahtarınız ve ana parolanız birlikte kasanızın kilidini açar. İkisinden hiçbirini sizin için kurtaramayız.",
  "onboarding.secretKey.verify": "Kaydettiğinizi doğrulamak için Gizli Anahtarınızın son grubunu yazın.",
  "onboarding.secretKey.mistyped":
    "Eşleşmiyor. Acil Durum Kitinizdeki son grubu kontrol edin.",
  "onboarding.step": "Adım {current} / {total}",
  "onboarding.create.title": "Hesabınızı oluşturun",
  "onboarding.create.body":
    "Ana parolanız ve bu aygıtta oluşturduğumuz bir Gizli Anahtar birlikte kasanızın kilidini açar. İkisi de sunucuya asla gönderilmez.",
  "onboarding.create.email": "E-posta",
  "onboarding.create.password": "Ana parola",
  "onboarding.create.passwordHint": "Birbiriyle ilgisiz dört sözcük iyi çalışır. Yalnızca Mosaic için kullanın.",
  "onboarding.create.confirm": "Ana parolayı onaylayın",
  "onboarding.create.mismatch":
    "İki parola eşleşmiyor. Her iki alana da aynı ana parolayı yazın.",
  "onboarding.create.submit": "Hesap oluştur",
  "onboarding.create.working": "Hesabınız oluşturuluyor. Bu birkaç saniye sürer.",
  "onboarding.kit.accountId": "Hesap kimliği",
  "onboarding.kit.secretKey": "Gizli Anahtar",
  "onboarding.kit.lastGroup": "Gizli Anahtarınızın son grubu",
  "onboarding.kit.confirm": "Onayla",
  "kit.title": "Acil Durum Kitinizi kaydedin",
  "kit.body":
    "Kit, Gizli Anahtarınızı içeren bir PDF’tir. Ana parolanızla birlikte kasanızı yeni bir aygıtta açar. Onu bu bilgisayardan uzakta, güvenli bir yerde saklayın.",
  "kit.secretKey": "Gizli Anahtarınız",
  "kit.save": "Acil Durum Kitini kaydet (PDF)",
  "kit.saved": "Kaydedildi. Dosyayı güvenli bir yerde saklayın.",
  "kit.cancelled": "Henüz kaydedilmedi. Devam etmek için kiti kaydedin.",
  "kit.recover.alreadySaved": "Zaten kaydettim",
  "kit.recover.title": "Acil Durum Kitinizi yeniden kaydedin",
  "kit.recover.body":
    "Acil Durum Kitinizi henüz onaylamadınız. Yeniden kaydetmek için ana parolanızı girin, sonra Gizli Anahtarınızın son grubunu onaylayın.",
  "onboarding.import.title": "Parolalarınızı da getirin",
  "onboarding.import.body":
    "Chrome, Edge, Firefox, Safari ve diğer tarayıcılardan veya 1Password, Bitwarden, LastPass, Dashlane, Keeper, NordPass, Proton Pass, RoboForm, KeePass ve Enpass’tan. Birkaç dakika sürer. Bunu daha sonra Ayarlar’dan da yapabilirsiniz.",
  "onboarding.import.start": "Başka bir uygulamadan içe aktar",
  "onboarding.import.skip": "Şimdilik atla",
  "onboarding.done.title": "Hazırsınız",
  "onboarding.done.body": "Kasanız hazır. İstediğiniz zaman Kilitle düğmesiyle kilitleyin.",
  "onboarding.browser.title": "Tarayıcı uzantısını yükleyin",
  "onboarding.browser.body":
    "Girişleri tarayıcınızda doldurun. Uzantı yalnızca ona tıkladığınızda veya klavye kısayolunu kullandığınızda doldurur.",
  "onboarding.browser.steps": "Bir tarayıcı nasıl bağlanır",
  "onboarding.browser.step1": "Mosaic uzantısını tarayıcınıza yükleyin.",
  "onboarding.browser.step2": "Uzantıyı açın ve Bağlan’ı seçin.",
  "onboarding.browser.step3":
    "Kodun eşleştiğini kontrol edin, sonra burada açılan pencerede İzin ver’i seçin.",
  "onboarding.browser.notInStores":
    "Uzantı henüz tarayıcı mağazalarında yok. Daha sonra Ayarlar → Tarayıcılar’dan bağlayabilirsiniz.",
  "onboarding.browser.getFor": "{browser} için edinin",
  "onboarding.browser.connectedList": "Bağlı tarayıcılar",
  "onboarding.browser.waiting": "Uzantı bekleniyor. Henüz bağlı bir tarayıcı yok.",
  "onboarding.browser.connected": "{name} bağlandı",
  "onboarding.browser.done": "Bitti",
  "onboarding.browser.skip": "Şimdilik atla",
  "onboarding.progress": "Kurulum ilerlemesi",
  "onboarding.stepName.create": "Hesap oluştur",
  "onboarding.stepName.kit": "Acil Durum Kiti",
  "onboarding.stepName.verify": "Kitinizi onaylayın",
  "onboarding.stepName.import": "İçe aktar (isteğe bağlı)",
  "onboarding.stepName.browser": "Tarayıcı uzantısı (isteğe bağlı)",
  "kit.preview": "Acil Durum Kiti önizlemesi",
  "kit.continueHint": "Devam etmek için kiti kaydedin.",
  "kit.warning":
    "Bu kit ve ana parolanız olmadan kimse kasanızı kurtaramaz, biz bile.",
  "onboarding.join.link": "Zaten Mosaic kullanıyorum",
  "onboarding.join.title": "Bu aygıtı hesabınıza ekleyin",
  "onboarding.join.body":
    "Ana parolanız ve Acil Durum Kitinizdeki Gizli Anahtar ile oturum açın. Kasalarınız sonra bu aygıta indirilir ve şifreleri çözülür.",
  "onboarding.join.tabs": "Eşitleme verileriniz nerede?",
  "onboarding.join.tab.server": "Mosaic sunucusu",
  "onboarding.join.tab.folder": "Bulut klasörü",
  "onboarding.join.serverUrl": "Sunucu adresi",
  "onboarding.join.email": "Eşitleme hesabı e-postası",
  "onboarding.join.password": "Ana parola",
  "onboarding.join.secretKey": "Gizli Anahtar",
  "onboarding.join.secretKeyHint": "Acil Durum Kitinizden. M1 ile başlar.",
  "onboarding.join.deviceName": "Bu aygıtın adı",
  "onboarding.join.chooseFolder": "Klasör seç…",
  "onboarding.join.folder": "Klasör",
  "onboarding.join.noAccounts":
    "Bu klasörde Mosaic hesabı yok. Diğer aygıtınızın eşitlediği klasörü seçin.",
  "onboarding.join.account": "Hesap",
  "onboarding.join.accountOption": "Hesap {id}",
  "onboarding.join.submit": "Bu aygıtı ekle",
  "onboarding.join.working": "Oturum açılıyor ve kasalarınız indiriliyor…",
  "onboarding.join.twoFactor.title": "Kodunuzu girin",
  "onboarding.join.twoFactor.body":
    "Eşitleme hesabınız iki adımlı doğrulama kullanıyor. Kimlik doğrulayıcı uygulamanızdaki 6 haneli kodu girin.",
  "onboarding.join.twoFactor.code": "6 haneli kod",
  "onboarding.join.twoFactor.submit": "Doğrula",
  "onboarding.join.back": "Bunun yerine yeni bir hesap oluştur",
} satisfies Record<keyof typeof en, string>;
