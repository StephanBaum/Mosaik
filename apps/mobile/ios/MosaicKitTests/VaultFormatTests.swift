import XCTest

@testable import MosaicKit

final class VaultFormatTests: XCTestCase {
    func testOtpCodeGroupingAndRingFraction() {
        XCTAssertEqual(OtpFormat.group("123456"), "123 456")
        XCTAssertEqual(OtpFormat.group("12345678"), "1234 5678")
        XCTAssertEqual(OtpFormat.group("12345"), "12345")
        XCTAssertEqual(OtpFormat.fraction(remaining: 15, period: 30), 0.5, accuracy: 0.0001)
        XCTAssertEqual(OtpFormat.fraction(remaining: 40, period: 30), 1, accuracy: 0.0001)
        XCTAssertEqual(OtpFormat.fraction(remaining: -3, period: 30), 0, accuracy: 0.0001)
        XCTAssertEqual(OtpFormat.fraction(remaining: 5, period: 0), 0, accuracy: 0.0001)
    }

    func testTypedWebsitesGetHttps() {
        XCTAssertEqual(UrlNormalizer.normalize(" example.com "), "https://example.com")
        XCTAssertEqual(UrlNormalizer.normalize("http://example.com"), "http://example.com")
        XCTAssertEqual(UrlNormalizer.appName("androidapp://org.example.app?cert=abcd"), "org.example.app")
        XCTAssertEqual(UrlNormalizer.appName("org.example.app"), "org.example.app")
    }

    func testSettingsOptionsKeepTheCurrentValue() {
        XCTAssertEqual(SettingsOptions.withCurrent([10, 30, 60], current: 45), [10, 30, 45, 60])
        XCTAssertEqual(SettingsOptions.withCurrent([10, 30, 60], current: 30), [10, 30, 60])
        let update = SettingsOptions.update(clipboard: 60)
        XCTAssertEqual(update.clipboardClearSeconds, 60)
        XCTAssertNil(update.autoLockBackgroundSeconds)
        XCTAssertNil(update.passwordIdentities)
        XCTAssertNil(update.reauthPassword)
        XCTAssertEqual(SettingsOptions.update(autoLock: -5).autoLockBackgroundSeconds, 0)
    }

    func testSyncLineSaysWhereTheVaultLives() {
        func overview(configured: Bool, error: String? = nil, last: UInt64? = nil) -> SyncOverview {
            SyncOverview(
                configured: configured, signedIn: configured, serverUrl: nil, pendingUploads: 0,
                lastSyncedAtMs: last, lastError: error, halted: false, warnings: [])
        }
        XCTAssertEqual(SyncLine.text(for: nil), "")
        XCTAssertEqual(SyncLine.text(for: overview(configured: false)), L10n.syncLocalOnly())
        XCTAssertEqual(SyncLine.text(for: overview(configured: true)), L10n.syncNever())
        XCTAssertEqual(
            SyncLine.text(for: overview(configured: true, error: "syncServerUnreachable")),
            L10n.errorSyncServerUnreachable())
        let now = Date()
        let last = UInt64(now.timeIntervalSince1970 * 1000) - 120_000
        let text = SyncLine.text(for: overview(configured: true, last: last), now: now)
        XCTAssertFalse(text.isEmpty)
        XCTAssertNotEqual(text, L10n.syncNever())
    }

    func testFieldLabelsKnowBuiltInKeysAndFallBackToTheKey() {
        for key in ["notes", "login.username", "login.password", "card.number", "identity.email", "wifi.ssid"] {
            XCTAssertNotNil(FieldLabels.builtIn(key: key), key)
        }
        XCTAssertNil(FieldLabels.builtIn(key: "custom.unknown"))
        let custom = FieldView(
            key: "custom.1", labelKey: nil, label: "Recovery email", concealed: false, value: "a@b.c", copyable: true)
        XCTAssertEqual(FieldLabels.label(for: custom), "Recovery email")
        let plain = FieldView(
            key: "custom.2", labelKey: nil, label: nil, concealed: false, value: nil, copyable: false)
        XCTAssertEqual(FieldLabels.label(for: plain), "custom.2")
        let kinds: [ItemKind] = [
            .login, .passkey, .totp, .secureNote, .creditCard, .identity, .sshKey, .apiKey, .wifi, .document, .other,
        ]
        for kind in kinds {
            XCTAssertFalse(FieldLabels.kind(kind).isEmpty)
        }
    }

    func testReplaceTakingMovesTheBytesAndWipesTheSource() {
        let buffer = SecretBuffer()
        buffer.set(from: "old")
        var incoming = Data("M1-secret".utf8)
        buffer.replace(taking: &incoming)
        XCTAssertTrue(incoming.isEmpty)
        var copy = buffer.copyOut()
        XCTAssertEqual(String(decoding: copy, as: UTF8.self), "M1-secret")
        copy.wipe()
        buffer.wipe()
        XCTAssertTrue(buffer.isEmpty)
    }
}
