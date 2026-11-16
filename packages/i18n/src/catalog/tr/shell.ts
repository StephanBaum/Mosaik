/**
 * Turkish UI strings: shell. Same keys as English (checked by the type).
 * ICU plural categories of Turkish: one, other (CLDR; `other` is required).
 */
import type { shell as en } from "../en/shell.ts";

export const shell = {
  "shell.emergencyKit.title": "Mosaic Acil Durum Kiti",
  "shell.emergencyKit.intro":
    "Bu sayfayı kilitli bir çekmece gibi güvenli bir yerde saklayın. Gizli Anahtarınız ve ana parolanız birlikte kasanızın kilidini yeni bir aygıtta açar. Hiç kimse, Mosaic dahil, bunlardan hiçbirini sizin için kurtaramaz.",
  "shell.emergencyKit.email": "Hesap e-postası",
  "shell.emergencyKit.accountId": "Hesap kimliği",
  "shell.emergencyKit.signInUrl": "Oturum açma adresi",
  "shell.emergencyKit.signInUrlLocal": "Yalnızca bu aygıt (henüz eşitleme sunucusu kurulmadı)",
  "shell.emergencyKit.secretKey": "Gizli Anahtar",
  "shell.emergencyKit.masterPassword": "Ana parola",
  "shell.emergencyKit.masterPasswordHint":
    "Satıra elle yalnızca bu sayfa güvenli bir yerde kalacaksa yazın. Aksi halde aklınızda tutun.",
  "shell.emergencyKit.footer":
    "Mosaic denetim öncesi bir yazılımdır: henüz bağımsız bir güvenlik denetçisi tarafından incelenmedi.",
  "shell.shortcut.conflict":
    "{shortcut} kısayolunu başka bir uygulama zaten kullanıyor. O uygulamayı kapatın veya kısayolunu değiştirin, sonra hızlı aramayı kullanmak için Mosaic’i yeniden başlatın.",
  "keys.ctrl": "Ctrl",
  "keys.shift": "Shift",
  "keys.alt": "Alt",
  "keys.space": "Boşluk",
  "keys.esc": "Esc",
  "keys.enter": "Enter",
} satisfies Record<keyof typeof en, string>;
