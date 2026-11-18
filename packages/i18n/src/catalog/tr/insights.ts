/**
 * Turkish UI strings: insights. Same keys as English (checked by the type).
 * ICU plural categories of Turkish: one, other (CLDR; `other` is required).
 */
import type { insights as en } from "../en/insights.ts";

export const insights = {
  "insights.title": "Güvenlik",
  "insights.intro":
    "Kontroller bu aygıtta çalışır. Son kontrol: {checkedAt, date, medium}, {checkedAt, time, short}; {count, plural, one {# öğe} other {# öğe}} tarandı.",
  "insights.checkAgain": "Yeniden kontrol et",
  "insights.summary": "Özet",
  "insights.fixFirst": "Önce bunları düzeltin",
  "insights.showAll": "Tüm {count, plural, one {# bulguyu} other {# bulguyu}} göster",
  "insights.empty": "Sorun bulunamadı. Mosaic bu sayfayı her açtığınızda yeniden kontrol eder.",
  "insights.kind.weak": "Zayıf",
  "insights.kind.weak.description": "Tahmin etmesi kolay",
  "insights.kind.reused": "Yeniden kullanılmış",
  "insights.kind.reused.description": "{groups, plural, one {# grup} other {# grup}}",
  "insights.kind.old": "Eski (1 yıldan uzun)",
  "insights.kind.old.description": "1 yıldan uzun süredir değiştirilmedi",
  "insights.kind.missingTotp": "2FA eksik",
  "insights.kind.missingTotp.description": "Site destekliyor ama siz kurmadınız",
  "insights.kind.insecureUrl": "Güvenli olmayan HTTP",
  "insights.kind.insecureUrl.description": "Parolanızı şifrelenmeden gönderir",
  "insights.kind.breached": "Veri ihlalinde",
  "insights.kind.breached.description": "İsteğe bağlı veri ihlali kontrolüyle bulundu",
  "insights.finding.breached":
    "Bu parola bilinen bir veri ihlalinde yer alıyor. Kullandığınız her yerde hemen değiştirin.",
  "insights.finding.weak": "Tahmin etmesi kolay. Oluşturulmuş bir parolayla değiştirin.",
  "insights.finding.reused":
    "Ayrıca {count, plural, one {# başka öğe} other {# başka öğe}} tarafından kullanılıyor. Her siteye ayrı bir parola verin.",
  "insights.finding.old": "1 yıldan uzun süredir değiştirilmedi. Bu hesabı hâlâ kullanıyorsanız değiştirin.",
  "insights.finding.missingTotp":
    "{site} iki adımlı doğrulama kodları sunuyor. Orada açın ve kodu Mosaic’e kaydedin.",
  "insights.finding.insecureUrl":
    "Düz HTTP kullanıyor: {url}. Site destekliyorsa https:// adresine geçin.",
  "insights.severity.high": "Yüksek",
  "insights.severity.medium": "Orta",
  "insights.severity.low": "Düşük",
  "insights.action.changePassword": "Parolayı değiştir",
  "insights.action.setUpTotp": "2FA kur",
  "insights.action.reviewUrl": "İncele",
  "insights.attribution.twoFactor": "2FA verileri: 2FA Directory (2fa.directory), MIT lisansı.",
  "insights.counts": "{count, plural, =0 {Sorun yok} one {# sorun} other {# sorun}}",
  "insights.documentation": "Nasıl açılır: {url}",
  "insights.actionFor": "{action}: {title}",
} satisfies Record<keyof typeof en, string>;
