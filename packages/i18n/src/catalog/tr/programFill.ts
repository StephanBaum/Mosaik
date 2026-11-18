/**
 * Turkish UI strings: programFill. Same keys as English (checked by the type).
 * ICU plural categories of Turkish: one, other (CLDR; `other` is required).
 */
import type { programFill as en } from "../en/programFill.ts";

export const programFill = {
  "programFill.chip.label": "Mosaic ile doldur",
  "programFill.chip.locked": "Doldurmak için Mosaic’in kilidini açın",
  "programFill.list.title": "{program} için girişler",
  "programFill.list.none": "{program} için henüz giriş yok",
  "programFill.list.signerChanged": "Bu programın imzalayanı değişti. Yeniden bağlansın mı?",
  "programFill.list.fileChanged": "Bu program bağladığınızdan beri değişti. Yeniden bağlansın mı?",
  "programFill.list.suggestions": "Öneriler",
  "programFill.list.search": "Girişleri ara",
  "programFill.list.link": "Bir giriş bağla…",
  "programFill.list.block": "Bu program için önerme",
  "programFill.link.packaged": "{name} · Mağaza uygulaması · Yayıncı {publisher}",
  "programFill.link.packagedNoPublisher": "{name} · Mağaza uygulaması",
  "programFill.link.signed": "{exe} · {signer} tarafından imzalandı, Windows tarafından denetlendi",
  "programFill.link.unsigned": "{exe} · İmzalı değil · {path}",
  "programFill.link.unsignedNote":
    "Mosaic yalnızca bu tam dosyayı doldurur. Değişirse Mosaic yeniden sorar.",
  "programFill.link.lookalike": "Dikkat: bu ad, daha önce bağladığınız {similar} adına benziyor.",
  "programFill.link.hashChange": "Bağlanan dosya {old}…, bu dosya {new}…",
  "programFill.link.login": "Giriş: {title}",
  "programFill.link.confirm": "Bağla ve oturum aç",
  "programFill.link.confirmFill": "Bağla ve doldur",
  "programFill.link.confirmUnsigned": "Bu imzasız programı bağla",
  "programFill.link.confirmAgain": "Bağlamak için yeniden onaylayın",
  "programFill.notice.elevated":
    "Mosaic yönetici olarak çalışan programları dolduramaz. Bunun yerine parolayı Mosaic’ten kopyalayın.",
  "programFill.notice.noField": "Mosaic burada bir giriş alanı göremiyor.",
  "programFill.notice.aborted":
    "Pencere değiştiği için Mosaic doldurmayı durdurdu. Alanı kontrol edin.",
  "programFill.notice.abortedCleared":
    "Pencere değiştiği için Mosaic doldurmayı durdurdu ve yazdığı her şeyi kaldırdı.",
  "programFill.notice.modifiersHeld":
    "Shift, Ctrl, Alt ve Windows tuşunu bırakın, sonra girişi yeniden seçin.",
  "programFill.notice.notResponding":
    "Bu program zamanında yanıt vermedi. Başka bir şey doldurulmadı.",
  "programFill.notice.close": "Kapat",
  "programFill.settings.title": "Programlarda doldurma",
  "programFill.settings.description":
    "Mosaic, girişlerinizi orada önermek için bu aygıttaki diğer programlarda hangi alanın odakta olduğunu izler. Hiçbir şey saklanmaz veya gönderilmez.",
  "programFill.settings.enable": "Programlarda giriş öner",
  "programFill.settings.submit": "Doldurduktan sonra oturum aç",
  "programFill.settings.hotkey": "Listeyi açan kısayol",
  "programFill.settings.hotkeyNone": "Kısayol yok",
  "programFill.settings.hotkeySave": "Liste kısayolunu kaydet",
  "programFill.settings.blocked": "Mosaic’in giriş önermediği programlar",
  "programFill.settings.blockedNone": "Yok",
  "programFill.settings.unblock": "Yeniden öner",
  "programFill.settings.notWindows": "Şimdilik yalnızca Windows’ta.",
  "programFill.settings.mosaicElevated":
    "Mosaic yönetici olarak çalışıyor, bu yüzden programları doldurmaz. Normal şekilde başlatın.",
  "programFill.settings.hostFailed":
    "Mosaic programları izlemeye başlayamadı. Yeniden denemek için Mosaic’i yeniden başlatın.",
  "programFill.card.title": "Girişleri programlarda da doldurun",
  "programFill.card.turnOn": "Aç",
  "programFill.card.notNow": "Şimdi değil",
  "programFill.item.program": "Program",
  "programFill.item.storeApp": "Mağaza uygulaması",
  "programFill.item.signedBy": "{signer} tarafından imzalandı",
  "programFill.item.unsigned": "İmzalı değil: {path}",
  "programFill.item.submit": "Doldurduktan sonra oturum aç",
  "programFill.item.unlink": "Bağlantıyı kaldır",
} satisfies Record<keyof typeof en, string>;
