import Foundation
import UIKit
import UniformTypeIdentifiers

/// The pasteboard rules of ADR-0094: a copied secret stays on this device (no Universal Clipboard) and
/// expires after the user's clipboard timeout.
public enum PasteboardOptions {
    /// `.localOnly` and `.expirationDate`.
    public static func options(expiringAfter seconds: Int, now: Date = Date()) -> [UIPasteboard.OptionsKey: Any] {
        [
            .localOnly: true,
            .expirationDate: now.addingTimeInterval(TimeInterval(seconds)),
        ]
    }

    /// One plain-text item.
    public static func items(for text: String) -> [[String: Any]] {
        [[UTType.utf8PlainText.identifier: text]]
    }
}

@MainActor
public enum PasteboardGuard {
    /// Copies UTF-8 `bytes` and wipes them. The pasteboard takes a `String`, which cannot be wiped (an honest
    /// limit, CRYPTO_SPEC 6.9.5).
    public static func copy(
        bytes: inout Data, expiringAfter seconds: Int, pasteboard: UIPasteboard = .general
    ) {
        let text = String(decoding: bytes, as: UTF8.self)
        bytes.wipe()
        pasteboard.setItems(
            PasteboardOptions.items(for: text),
            options: PasteboardOptions.options(expiringAfter: seconds))
    }

    /// Copies a value of the core, then wipes the copy and the handle.
    public static func copy(_ value: SecretValue, expiringAfter seconds: Int, pasteboard: UIPasteboard = .general) {
        var bytes = value.exposeBytes()
        copy(bytes: &bytes, expiringAfter: seconds, pasteboard: pasteboard)
        value.wipe()
    }
}
