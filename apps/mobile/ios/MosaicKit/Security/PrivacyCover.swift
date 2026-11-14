import Observation
import SwiftUI

/// Whether the privacy cover is up (spec 3.13). The app covers its window as soon as the scene resigns
/// active, so the app-switcher snapshot and a screen share show no vault data, and uncovers it when the
/// scene is active again.
@MainActor
@Observable
public final class PrivacyCoverModel {
    public private(set) var isCovered: Bool

    public init(isCovered: Bool = false) {
        self.isCovered = isCovered
    }

    public func sceneWillResignActive() {
        isCovered = true
    }

    public func sceneDidBecomeActive() {
        isCovered = false
    }
}

/// The cover: the background color and the app name, in the theme of the system appearance.
public struct PrivacyCoverView: View {
    @Environment(\.colorScheme) private var colorScheme

    public init() {}

    public var body: some View {
        let colors = colorScheme == .dark ? MosaicTokens.dark : MosaicTokens.light
        ZStack {
            colors.bg.ignoresSafeArea()
            Text(L10n.appName())
                .font(Font.mosaic(MosaicTokens.FontSize.xl, weight: .semibold))
                .foregroundStyle(colors.text)
                .accessibilityHidden(true)
        }
        .accessibilityIdentifier("privacy-cover")
    }
}

extension View {
    /// Draws the privacy cover over this view while `model.isCovered`.
    public func privacyCover(_ model: PrivacyCoverModel) -> some View {
        overlay {
            if model.isCovered {
                PrivacyCoverView()
            }
        }
    }
}
