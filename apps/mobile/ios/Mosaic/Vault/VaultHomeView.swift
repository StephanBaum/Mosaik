import MosaicKit
import SwiftUI

/// The unlocked screen of T10: the account and a lock button. T11 replaces it with the vault list, the
/// item detail and the settings.
struct VaultHomeView: View {
    let model: AppModel
    let email: String?

    var body: some View {
        MosaicScreen {
            ScreenTitle(text: L10n.vaultTitle())
                .accessibilityIdentifier("vault-title")
            if let email {
                BodyText(text: L10n.unlockAccount(email: email), muted: true)
            }
            Button(L10n.lockNow()) { model.lock() }
                .buttonStyle(SecondaryButtonStyle())
                .accessibilityIdentifier("lock-now")
        }
    }
}
