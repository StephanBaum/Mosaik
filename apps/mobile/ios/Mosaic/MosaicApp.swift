import MosaicKit
import SwiftUI

@main
struct MosaicApp: App {
    var body: some Scene {
        WindowGroup {
            RootView()
        }
    }
}

/// T6 skeleton: the app name and the version of the Rust core, with the generated theme.
struct RootView: View {
    @Environment(\.colorScheme) private var colorScheme

    var body: some View {
        let colors = colorScheme == .dark ? MosaicTokens.dark : MosaicTokens.light
        ZStack {
            colors.bg.ignoresSafeArea()
            VStack(spacing: MosaicTokens.Space.s4) {
                Text(L10n.appName())
                    .font(.system(size: MosaicTokens.FontSize.xl.size, weight: .semibold))
                    .foregroundStyle(colors.text)
                Text(L10n.settingsVersion(version: coreVersion()))
                    .font(.system(size: MosaicTokens.FontSize.md.size))
                    .foregroundStyle(colors.textMuted)
                    .accessibilityIdentifier("core-version")
            }
            .padding(MosaicTokens.Space.s6)
        }
    }
}
