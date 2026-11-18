/**
 * Turkish UI strings: sharing. Same keys as English (checked by the type).
 * ICU plural categories of Turkish: one, other (CLDR; `other` is required).
 */
import type { sharing as en } from "../en/sharing.ts";

export const sharing = {
  "sharing.title": "Paylaşım",
  "sharing.body":
    "Kasaları tanıdığınız kişilerle paylaşın. Her şey şifreli kalır: paylaşılan bir kasayı yalnızca davet ettiğiniz kişiler okuyabilir ve Mosaic yalnızca güvenlik kodunu karşılaştırdığınız kişilerle paylaşır.",
  "sharing.notHosted.none":
    "Paylaşım bir Mosaic sunucusu gerektirir. Önce Eşitleme ayarlarında bir Mosaic sunucusuyla eşitlemeyi açın.",
  "sharing.notHosted.folder":
    "Paylaşım bir eşitleme klasörüyle kullanılamaz. Bir klasör bir kasayı kimin okuyabileceğini bilemez, bu yüzden Mosaic klasördeki kasaları yalnızca size özel tutar. Paylaşmak için klasörün bağlantısını kesin ve Eşitleme ayarlarında bir Mosaic sunucusu bağlayın.",
  "sharing.notHosted": "Paylaşım bir Mosaic sunucusu gerektirir. Önce Eşitleme ayarlarında bir tane bağlayın.",
  "sharing.syncSettings": "Eşitleme ayarlarını aç",
  "sharing.ownCode.title": "Güvenlik kodunuz",
  "sharing.ownCode.body":
    "Bu kodu paylaştığınız kişiye yüz yüze veya telefonda okuyun. Sizin için gördüğüyle eşleşiyorsa aranızda kimse yok demektir.",
  "sharing.ownCode.pending": "Güvenlik kodunuz sunucuyla ilk eşitlemeden sonra görünür.",
  "sharing.contacts.title": "Aradığınız kişiler",
  "sharing.contacts.empty": "Henüz kimseyi aramadınız.",
  "sharing.contacts.verified": "Doğrulandı",
  "sharing.contacts.notVerified": "Doğrulanmadı",
  "sharing.contacts.changed": "Yeni kontrol gerekiyor",
  "sharing.contacts.verify": "Kodu karşılaştır…",
  "sharing.contacts.verifyOf": "{email} ile kodu karşılaştır",
  "sharing.contacts.review": "Değişikliği incele…",
  "sharing.contacts.reviewOf": "{email} için değişikliği incele",
  "sharing.share.open": "Paylaş…",
  "sharing.share.title": "{vault} kasasını paylaş",
  "sharing.share.dialog": "Paylaşım: {vault}",
  "sharing.share.roleStep": "{email} için bir rol seçin",
  "sharing.lookup.email": "E-posta adresi",
  "sharing.lookup.find": "Ara",
  "sharing.lookup.searching": "Aranıyor…",
  "sharing.lookup.notFoundHint":
    "Mosaic bir adresin hesabı olup olmadığını söyleyemez. Yukarıdaki kod kişinin gördüğüyle eşleşmiyorsa paylaşmayın.",
  "sharing.code.title": "Güvenlik kodlarını karşılaştırın",
  "sharing.code.compare":
    "Bu kodu {email} ile yüz yüze veya telefonda karşılaştırın. E-posta veya sohbet kullanmayın.",
  "sharing.code.compareNew":
    "Yeni kodu {email} ile yüz yüze veya telefonda karşılaştırın. E-posta veya sohbet kullanmayın.",
  "sharing.code.shown": "{email} için Mosaic’in gösterdiği kod",
  "sharing.code.typed": "Size okudukları kod (isteğe bağlı)",
  "sharing.code.typedMatch": "Bu, yukarıdaki kod.",
  "sharing.code.attest":
    "Bu kodu {email} ile telefonda veya yüz yüze karşılaştırdım ve aynı.",
  "sharing.code.matches": "Kodlar eşleşiyor",
  "sharing.code.mismatch":
    "Kodlar farklı. Paylaşmayın. Sizinle {email} arasında biri olabilir.",
  "sharing.code.mismatchHelp":
    "Adresi kontrol edin ve {email} kişisinden kodu yeniden okumasını isteyin. Hâlâ farklıysa iptal edin ve ona başka bir yolla bildirin.",
  "sharing.code.verified": "{email} doğrulandı.",
  "sharing.code.status.new": "Yeni kişi",
  "sharing.code.status.pinned": "Henüz doğrulanmadı",
  "sharing.code.status.verified": "Doğrulandı",
  "sharing.code.status.changed": "Anahtar değişti",
  "sharing.code.status.conflicting": "Aygıtlar uyuşmuyor",
  "sharing.code.conflicting":
    "Aygıtlarınız {email} için farklı anahtarlar gördü. Paylaşmadan önce kodu onunla yeniden karşılaştırın.",
  "sharing.verify.title": "{email} kişisini doğrula",
  "sharing.role.label": "Rol",
  "sharing.role.owner": "Sahip",
  "sharing.role.manage": "Yönetici",
  "sharing.role.write": "Düzenleyici",
  "sharing.role.read": "Görüntüleyici",
  "sharing.role.desc.manage": "Kişileri davet edebilir ve kaldırabilir, rolleri değiştirebilir.",
  "sharing.role.desc.write": "Öğe ekleyebilir ve değiştirebilir.",
  "sharing.role.desc.read": "Yalnızca okuyabilir.",
  "sharing.role.honesty":
    "Bir öğeyi okuyabilen herkes onu kopyalayabilir. Bir görüntüleyici kasayı değiştiremez ama gördüklerini saklamasını hiçbir şey engellemez ve birini sonradan kaldırmak, zaten indirdiklerini geri almaz.",
  "sharing.invite.send": "Davet gönder",
  "sharing.invite.sent": "Davet {email} adresine gönderildi. Mosaic’i açıp kabul etmesi gerekiyor.",
  "sharing.invite.sentTitle": "Davet gönderildi",
  "sharing.invite.pending": "{email} bekleniyor",
  "sharing.invite.expires": "Bitiş: {when}",
  "sharing.invite.cancel": "Daveti geri çek",
  "sharing.invite.cancelOf": "{email} için daveti geri çek",
  "sharing.invite.cancelled": "Davet geri çekildi.",
  "sharing.inbox.title": "Davetler",
  "sharing.inbox.nav": "Davetler",
  "sharing.inbox.navCount": "Davetler, {count} bekliyor",
  "sharing.inbox.intro":
    "Sizi paylaşılan bir kasaya davet eden kişiler. Mosaic her daveti size göstermeden önce kontrol eder.",
  "sharing.inbox.empty":
    "Şu anda davet yok. Biri sizi paylaşılan bir kasaya davet ettiğinde burada görünür.",
  "sharing.inbox.from": "Gönderen: {email}",
  "sharing.inbox.review": "İncele",
  "sharing.inbox.reviewOf": "{email} tarafından gelen daveti incele",
  "sharing.inbox.refresh": "Yenile",
  "sharing.inbox.preview.title": "{vault} kasasına katılınsın mı?",
  "sharing.inbox.preview.body": "{email} sizi {role} olarak davet ediyor.",
  "sharing.inbox.preview.unnamed": "paylaşılan bir kasa",
  "sharing.inbox.preview.owner": "Kasanın sahibi: {email}.",
  "sharing.inbox.verifySender": "Katılmadan önce {email} kişisinin güvenlik kodunu karşılaştırın.",
  "sharing.inbox.verifyOwner":
    "{sender} sahip değil. Katılmadan önce sahibin güvenlik kodunu da karşılaştırın: kasanın anahtarını değiştirebilen odur.",
  "sharing.inbox.ownerCode": "Sahibin kodu: {email}",
  "sharing.inbox.accept": "Kasaya katıl",
  "sharing.inbox.readyTitle": "Hepsi kontrol edildi. Bu kasaya katılınsın mı?",
  "sharing.inbox.decline": "Reddet",
  "sharing.inbox.block": "Reddet ve göndereni engelle",
  "sharing.inbox.block.title": "Bu gönderen reddedilip engellensin mi?",
  "sharing.inbox.block.body":
    "Mosaic daveti reddeder ve bundan sonra {email} kişisinden gelen tüm davetleri yok sayar. Bunu Mosaic’te geri alamazsınız.",
  "sharing.inbox.unreadable":
    "Bu davet açılamıyor. Hasarlı olabilir veya size yönelik olmayabilir. {email} kişisinden yeniden göndermesini isteyin.",
  "sharing.inbox.joined": "Kasaya katıldınız. Kısa süre içinde kasa listenizde görünür.",
  "sharing.inbox.declined": "Davet reddedildi.",
  "sharing.inbox.outgoing": "Gönderdiğiniz davetler",
  "sharing.inbox.outgoing.empty": "Davetlerinizden hiçbiri yanıt beklemiyor.",
  "sharing.inbox.unknownVault": "bir kasa",
  "sharing.members.title": "Erişimi olan kişiler",
  "sharing.members.you": "Siz",
  "sharing.members.unverified": "Doğrulanmadı",
  "sharing.members.unconfirmed": "Üye listesinde yok",
  "sharing.members.unconfirmed.help":
    "Sunucu bu kişiyi listeliyor ama kasanın kendi üye listesi listelemiyor. Bir sonraki anahtar değişikliği onu dışarıda bırakır. Tanımıyorsanız kaldırın.",
  "sharing.members.dropped": "Artık sunucuda yok",
  "sharing.members.dropped.help":
    "Bu kişi kasanın üye listesinde ama sunucu artık onu listelemiyor. Ayrılmış olabilir. Bunun beklenen bir durum olduğunu kontrol edin.",
  "sharing.members.invited": "Davet edildi",
  "sharing.members.removing": "Kaldırılıyor",
  "sharing.members.verified": "Doğrulandı",
  "sharing.members.verify": "Doğrula…",
  "sharing.members.verifyOf": "{email} kişisini doğrula",
  "sharing.members.remove": "Kaldır",
  "sharing.members.removeOf": "{email} kişisini kaldır",
  "sharing.members.remove.title": "{email} kaldırılsın mı?",
  "sharing.members.remove.confirm":
    "{email} zaten indirdiklerini korur. Mosaic kasa anahtarını şimdi değiştirir ve yeni hiçbir şeyi okuyamaz.",
  "sharing.members.remove.confirmManager":
    "{email} zaten indirdiklerini korur. Sunucu ona artık güncelleme göndermez. Kasa sahibinin aygıtı kasa anahtarını bir sonraki eşitlemede değiştirir ve o zamana kadar {email} kişisini yalnızca sunucu dışarıda tutar.",
  "sharing.members.cancel": "İptal",
  "sharing.members.removed": "Kaldırıldı.",
  "sharing.members.leave": "Kasadan ayrıl",
  "sharing.members.leave.title": "Bu kasadan ayrılınsın mı?",
  "sharing.members.leave.confirm":
    "Bu aygıttaki kopya okunabilir kalır ama güncellenmez. Yeniden katılmak için yeni bir davete ihtiyacınız var.",
  "sharing.members.left": "Kasadan ayrıldınız.",
  "sharing.members.roleOf": "{email} kişisinin rolü",
  "sharing.members.changeRole": "Rolü değiştir",
  "sharing.members.changeRoleOf": "{email} kişisinin rolünü değiştir",
  "sharing.members.invite": "Birini davet et…",
  "sharing.members.empty": "Şimdiye kadar bu kasayı yalnızca siz açabiliyorsunuz.",
  "sharing.members.unknownEmail": "Bilinmeyen adres",
  "sharing.members.managerNote":
    "Kişileri davet edebilir ve kaldırabilirsiniz. Kasa anahtarını yalnızca sahibin aygıtı değiştirir, bu yüzden bir kaldırma sahibin aygıtı eşitlendikten sonra tamamlanır.",
  "sharing.members.readerNote": "Bu kasayı okuyabilirsiniz ama kimin erişimi olduğunu değiştiremezsiniz.",
  "sharing.rotation.running":
    "Bir kaldırma, sahibin aygıtının kasa anahtarını değiştirmesini bekliyor. O eşitlenene kadar kaldırılan kişiyi yalnızca sunucu dışarıda tutar.",
  "sharing.rotation.unprotected":
    "{count, plural, one {# dosya} other {# dosya}} eski korumasını sürdürüyor, çünkü bu aygıtta yok. Kaldırdığınız bir kişi eski anahtarı sakladıysa bunları yine açabilir. Bunları, sahip olan bir aygıttan yeniden ekleyin.",
  "sharing.keyChanged.title": "Güvenlik kodu değişti: {email}",
  "sharing.keyChanged.body":
    "Sunucu artık {email} için farklı bir anahtar gösteriyor. Bu, kişi Mosaic’i yeniden kurduğunda olur ve sunucuya yapılan bir saldırı da böyle görünür. Onunla yüz yüze veya telefonda kodları karşılaştırmadıkça yeni anahtarla hiçbir şey paylaşılmaz.",
  "sharing.keyChanged.old": "Daha önce karşılaştırdığınız kod",
  "sharing.keyChanged.new": "Yeni kod",
  "sharing.keyChanged.accept": "Yeni kod eşleşiyor",
  "sharing.finding.region": "Paylaşım uyarıları",
  "sharing.finding.summary": "{count, plural, one {# paylaşım uyarısı} other {# paylaşım uyarısı}}",
  "sharing.finding.review": "İncele",
  "sharing.finding.collapse": "Daralt",
  "sharing.finding.dismiss": "Kapat",
  "sharing.finding.unknownEmail": "bilinmeyen bir kişi",
  "sharing.finding.verify": "Doğrula…",
  "sharing.finding.remove": "Kaldır",
  "sharing.finding.removeOf": "{email} kişisini {vault} kasasından kaldır",
  "sharing.finding.identityMismatch":
    "Sunucu hesabınız için bu aygıtın ürettiğinden farklı bir paylaşım anahtarı gösteriyor, bu yüzden paylaşım duraklatıldı. Hiçbir şey paylaşmayın ve sunucunuzu ve diğer aygıtlarınızı kontrol edin. Sonra “Kontrol ettim…”i seçin.",
  "sharing.finding.rosterInvalid":
    "{vault} kasasının üye listesi hasarlı veya biri izinsiz değiştirdi. Kimin erişimi olduğunu kontrol edene kadar kişi eklemeyin. Sonra “Kontrol ettim…”i seçin.",
  "sharing.finding.shareUntrusted":
    "{vault} için yeni bir anahtar reddedildi: onu {sender} gönderdi ama bu kasanın anahtarını değiştirmesine izin verilmiyor. Kasa eski anahtarında kalır. Sahibe değiştirip değiştirmediğini sorun. Sonra “Kontrol ettim…”i seçin.",
  "sharing.finding.memberUnverified":
    "{vault} içinde {email} kişisini doğrulamadınız. Mosaic ona yeni anahtar göndermez ve bir sonraki anahtar değişikliği onu dışarıda bırakır. Güvenlik kodunu karşılaştırın veya kaldırın.",
  "sharing.finding.memberDropped":
    "{email}, {vault} kasasının üye listesinde ama sunucu artık onu listelemiyor. Ayrılmış olabilir veya sunucu onu kaldırmış olabilir. Bunun beklenen bir durum olduğunu kontrol edin, değilse kaldırın.",
  "sharing.finding.memberKeyMismatch":
    "Mosaic, {email} için doğruladığınız anahtarla eşleşmeyen bir anahtar aldı, bu yüzden {vault} kasasından kaldırıldı. Yeniden davet etmeden önce güvenlik kodlarını onunla yeniden karşılaştırın.",
  "sharing.finding.vaultHeld":
    "{vault}, diğer aygıtlarınızın kimin sahibi olduğunu onaylamasını bekliyor. Mosaic’i bu kasayı zaten kullanan bir aygıtta açın.",
  "sharing.finding.accessLost":
    "Artık {vault} kasasına erişiminiz yok. Bu aygıttaki kopya okunabilir kalır ama güncellenmez. Sahibinden sizi yeniden davet etmesini isteyin.",
  "sharing.finding.rotationBlocked":
    "Birini kaldırdıktan sonra {vault} kasasının anahtarı değiştirilemedi, bu yüzden o kişi hâlâ eski anahtara sahip. Yeniden eşitleyin. Bu sürerse üye listesini açın ve kimin erişimi olduğunu kontrol edin.",
  "emergency.title": "Acil durum erişimi",
  "emergency.body":
    "Size bir şey olursa kasalarınıza erişim isteyebilecek kişileri belirleyin. Bu hesap kurtarma değildir: seçtiğiniz kasalara yalnızca bir bekleme süresinden sonra salt okunur erişim alırlar ve bu sürede reddedebilirsiniz. Acil Durum Kitinizin yerini tutmaz.",
  "emergency.folderUnsupported":
    "Acil durum erişimi Mosaic sunucusunu gerektirir. Bir eşitleme klasörü bekleme süresini uygulayamaz.",
  "emergency.granted.title": "Erişim isteyebilecek kişiler",
  "emergency.granted.empty": "Kimse kasalarınıza erişim isteyemez.",
  "emergency.granted.add": "Bir kişi ekle…",
  "emergency.granted.scopeAll": "Tüm kasalarınız",
  "emergency.granted.scopeSome": "{count, plural, one {# kasa} other {# kasa}}",
  "emergency.granted.wait": "{days, plural, one {# gün} other {# gün}} bekler",
  "emergency.granted.waitingUntil": "Erişim istedi. Siz reddetmezseniz {when} açılır.",
  "emergency.granted.approved": "İsteği onayladınız.",
  "emergency.granted.released": "Erişim açıldı. Kasaları okuyabilir.",
  "emergency.granted.idle": "İstek yok",
  "emergency.granted.remove": "Erişimi kaldır",
  "emergency.granted.removeOf": "{email} için erişimi kaldır",
  "emergency.granted.remove.title": "{email} için acil durum erişimi kaldırılsın mı?",
  "emergency.granted.remove.body":
    "Artık istek yapamaz. Erişimi zaten açıldıysa Mosaic açtığı kasaların anahtarlarını değiştirir; zaten indirdiklerini korur.",
  "emergency.granted.removed": "Acil durum erişimi kaldırıldı.",
  "emergency.grant.title": "Acil durum erişimine izin ver",
  "emergency.grant.lookup": "Kim isteyebilmeli?",
  "emergency.grant.optionsTitle": "{email} neleri açabilir",
  "emergency.grant.scope": "Neleri açabilir",
  "emergency.grant.scope.all": "Yeniler dahil tüm kasalarım",
  "emergency.grant.scope.selected": "Yalnızca seçtiğim kasalar",
  "emergency.grant.scope.pick": "En az bir kasa seçin.",
  "emergency.grant.wait": "Bekleme süresi",
  "emergency.grant.wait.option": "{days, plural, one {# gün} other {# gün}}",
  "emergency.grant.waitHint":
    "Bir istekten sonra bu süre içinde Mosaic’i en az bir kez açın; böylece onu görüp reddedebilirsiniz. Süre dolunca kişi, size başka soru sorulmadan salt okunur erişim alır.",
  "emergency.grant.serverTrust":
    "Bekleme süresini sunucu tutar. Belirttiğiniz kişi ve sunucuyu işleten kişi birlikte çalışırsa bunu atlayabilirler. Sunucu tek başına kasalarınızı okuyamaz.",
  "emergency.grant.vaultKeysOnly":
    "Bu yalnızca kasalarınızın anahtarlarını kapsar. Hesabınıza, ana parolanıza veya Gizli Anahtarınıza erişim vermez.",
  "emergency.grant.submit": "Acil durum erişimine izin ver",
  "emergency.grant.done": "{email} artık acil durum erişimi isteyebilir.",
  "emergency.grant.needVerify": "Önce {email} kişisinin güvenlik kodunu karşılaştırın.",
  "emergency.received.title": "Sizi belirleyen kişiler",
  "emergency.received.empty": "Kimse sizi acil durum erişimi için belirlemedi.",
  "emergency.received.request": "Erişim iste",
  "emergency.received.requestOf": "{email} kişisinden erişim iste",
  "emergency.received.request.title": "{email} kişisinden erişim istensin mi?",
  "emergency.received.request.body":
    "{email}, Mosaic’i bir sonraki açışında isteğinizi görür. {days, plural, one {# gün} other {# gün}} sonra, reddetmezse kasalarını salt okunur açabilirsiniz.",
  "emergency.received.requested": "İstek gönderildi. {email}, Mosaic’i bir sonraki açışında görür.",
  "emergency.received.requestHint":
    "{email}, isteğinizi Mosaic’te görür. {days, plural, one {# gün} other {# gün}} sonra, reddetmezse kasalarını salt okunur açabilirsiniz.",
  "emergency.received.countdown": "Kasalarını {when} açabilirsiniz.",
  "emergency.received.waitingLive": "İsteğiniz bekliyor.",
  "emergency.received.claim": "Kasalarını aç",
  "emergency.received.claimOf": "Kasalarını aç: {email}",
  "emergency.received.claimed":
    "{count, plural, one {# kasa salt okunur olarak eklendi.} other {# kasa salt okunur olarak eklendi.}}",
  "emergency.received.cooldown": "{email} reddetti. {when} yeniden isteyebilirsiniz.",
  "emergency.received.verifyFirst":
    "Kasalarını açmadan önce {email} kişisinin güvenlik kodunu karşılaştırın. Mosaic yalnızca doğruladığınız bir anahtarı kullanır.",
  "emergency.received.verify": "Kodu karşılaştır…",
  "emergency.received.verifyOf": "{email} ile kodu karşılaştır",
  "emergency.received.approved": "{email} isteğinizi onayladı.",
  "emergency.received.waitDays": "{days, plural, one {# gün} other {# gün}} bekler",
  "emergency.banner.region": "Acil durum erişimi isteği",
  "emergency.banner.body":
    "{email} kasalarınıza acil durum erişimi istedi. Siz reddetmezseniz {when} açılır.",
  "emergency.banner.refuse": "Reddet",
  "emergency.banner.approve": "Şimdi onayla…",
  "emergency.banner.approve.title": "Acil durum erişimi şimdi onaylansın mı?",
  "emergency.banner.approve.body":
    "{email} kasalarınıza hemen salt okunur erişim alır. Onaylamak için ana parolanızı girin.",
  "emergency.banner.approve.confirm": "Onayla",
  "emergency.banner.refused": "İstek reddedildi.",
  "emergency.banner.approved": "Erişim onaylandı.",
} satisfies Record<keyof typeof en, string>;
