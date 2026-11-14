import Foundation

/// How a one-time code is drawn (6 digits as 3 and 3, 8 as 4 and 4).
public enum OtpFormat {
    public static func group(_ code: String) -> String {
        switch code.count {
        case 6: return String(code.prefix(3)) + " " + String(code.dropFirst(3))
        case 8: return String(code.prefix(4)) + " " + String(code.dropFirst(4))
        default: return code
        }
    }

    /// The share of the period that is left, for the countdown ring: 0 to 1.
    public static func fraction(remaining: Int, period: Int) -> Double {
        guard period > 0 else { return 0 }
        return min(max(Double(remaining) / Double(period), 0), 1)
    }
}

/// Typed websites of the login editor.
public enum UrlNormalizer {
    /// A typed address gets `https://` when it has no scheme.
    public static func normalize(_ typed: String) -> String {
        let trimmed = typed.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.contains("://") ? trimmed : "https://" + trimmed
    }

    /// The text of an app URL (`androidapp://pkg?cert=...`) shown as the package name.
    public static func appName(_ url: String) -> String {
        var name = url
        if let range = name.range(of: "androidapp://") {
            name = String(name[range.upperBound...])
        }
        if let cut = name.firstIndex(of: "?") {
            name = String(name[..<cut])
        }
        return name
    }
}

/// The choices of the settings pickers. The current value is always one of them, so a value set on another
/// device (the desktop allows more) is shown and not silently replaced.
public enum SettingsOptions {
    public static let autoLockSeconds = [0, 30, 60, 300]
    public static let idleMinutes = [1, 2, 5, 10, 15, 30]
    public static let clipboardSeconds = [10, 30, 60, 120, 300]

    public static func withCurrent(_ options: [Int], current: Int) -> [Int] {
        Array(Set(options + [current])).sorted()
    }

    public static func autoLockLabel(seconds: Int) -> String {
        switch seconds {
        case 0: return L10n.settingsAutoLockNow()
        case 30: return L10n.settingsAutoLockS30()
        case 60: return L10n.settingsAutoLockS60()
        case 300: return L10n.settingsAutoLockS300()
        default: return L10n.settingsClipboardOption(seconds: seconds)
        }
    }

    /// A settings change that leaves everything else as it is.
    public static func update(
        autoLock: Int? = nil, idle: Int? = nil, clipboard: Int? = nil, passkeyIdentities: Bool? = nil,
        passwordIdentities: Bool? = nil, reauthPassword: Data? = nil
    ) -> SettingsUpdate {
        SettingsUpdate(
            autoLockBackgroundSeconds: autoLock.map { UInt32(clamping: $0) },
            idleLockMinutes: idle.map { UInt32(clamping: $0) },
            clipboardClearSeconds: clipboard.map { UInt32(clamping: $0) },
            quickUnlockMaxAgeDays: nil,
            passkeyIdentities: passkeyIdentities,
            passwordIdentities: passwordIdentities,
            reauthPassword: reauthPassword)
    }
}

/// The sync state as one line: where the vault lives, the last error, or when it last synced.
public enum SyncLine {
    public static func text(for overview: SyncOverview?, now: Date = Date()) -> String {
        guard let overview else { return "" }
        if !overview.configured { return L10n.syncLocalOnly() }
        if let error = overview.lastError { return ErrorText.message(forCode: error) }
        guard let last = overview.lastSyncedAtMs else { return L10n.syncNever() }
        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .full
        let date = Date(timeIntervalSince1970: TimeInterval(last) / 1000)
        return L10n.syncLast(time: formatter.localizedString(for: date, relativeTo: now))
    }
}

/// The label of a field of the generic detail: a custom field brings its own, a built-in one has a stable key.
public enum FieldLabels {
    public static func label(for field: FieldView) -> String {
        if let custom = field.label { return custom }
        return builtIn(key: field.key) ?? field.key
    }

    public static func builtIn(key: String) -> String? {
        switch key {
        case "notes": return L10n.itemFieldNotes()
        case "login.username": return L10n.itemFieldUsername()
        case "login.password": return L10n.itemFieldPassword()
        case "login.change_password_url": return L10n.itemFieldChangePasswordUrl()
        case "card.cardholder": return L10n.itemFieldCardholderName()
        case "card.brand": return L10n.itemFieldCardBrand()
        case "card.expiry": return L10n.itemFieldCardExpiry()
        case "card.number": return L10n.itemFieldCardNumber()
        case "card.cvv": return L10n.itemFieldCardVerificationCode()
        case "card.pin": return L10n.itemFieldCardPin()
        case "identity.first_name": return L10n.itemFieldFirstName()
        case "identity.middle_name": return L10n.itemFieldMiddleName()
        case "identity.last_name": return L10n.itemFieldLastName()
        case "identity.email": return L10n.itemFieldEmail()
        case "identity.phone": return L10n.itemFieldPhone()
        case "identity.company": return L10n.itemFieldCompany()
        case "identity.address1": return L10n.itemFieldAddressLine1()
        case "identity.address2": return L10n.itemFieldAddressLine2()
        case "identity.city": return L10n.itemFieldCity()
        case "identity.region": return L10n.itemFieldRegion()
        case "identity.postal_code": return L10n.itemFieldPostalCode()
        case "identity.country": return L10n.itemFieldCountry()
        case "identity.birth_date": return L10n.itemFieldBirthDate()
        case "identity.passport_number": return L10n.itemFieldPassportNumber()
        case "identity.license_number": return L10n.itemFieldLicenseNumber()
        case "identity.national_id": return L10n.itemFieldNationalId()
        case "api_key.key_id": return L10n.itemFieldApiKeyId()
        case "api_key.host": return L10n.itemFieldApiKeyHost()
        case "api_key.secret": return L10n.itemFieldApiKeySecret()
        case "wifi.ssid": return L10n.itemFieldWifiSsid()
        case "wifi.security": return L10n.itemFieldWifiSecurity()
        case "wifi.password": return L10n.itemFieldWifiPassword()
        case "ssh.comment": return L10n.itemFieldSshComment()
        case "ssh.passphrase": return L10n.itemFieldSshPassphrase()
        case "ssh.public_key": return L10n.itemFieldSshPublicKey()
        case "ssh.private_key": return L10n.itemFieldSshPrivateKey()
        case "ssh.fingerprint": return L10n.itemFieldSshFingerprint()
        default: return nil
        }
    }

    public static func kind(_ kind: ItemKind) -> String {
        switch kind {
        case .login: return L10n.itemKindLogin()
        case .passkey: return L10n.itemKindPasskey()
        case .totp: return L10n.itemKindTotp()
        case .secureNote: return L10n.itemKindSecureNote()
        case .creditCard: return L10n.itemKindCreditCard()
        case .identity: return L10n.itemKindIdentity()
        case .sshKey: return L10n.itemKindSshKey()
        case .apiKey: return L10n.itemKindApiKey()
        case .wifi: return L10n.itemKindWifi()
        case .document: return L10n.itemKindDocument()
        case .other: return L10n.itemKindUnsupported()
        }
    }
}
