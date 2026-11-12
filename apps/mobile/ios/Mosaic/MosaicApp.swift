import MosaicKit
import SwiftUI

@main
struct MosaicApp: App {
    @State private var model = AppModel()
    @Environment(\.scenePhase) private var scenePhase

    var body: some Scene {
        WindowGroup {
            RootView(model: model)
                .onChange(of: scenePhase, initial: true) { _, phase in
                    model.scenePhaseChanged(phase)
                }
        }
    }
}

/// Chooses the screen from the state of the core: onboarding, unlock or the unlocked vault. The privacy
/// cover goes over everything while the scene is not active (spec 3.13).
struct RootView: View {
    @Environment(\.colorScheme) private var colorScheme
    let model: AppModel

    var body: some View {
        ZStack {
            colorScheme.mosaic.bg.ignoresSafeArea()
            content
        }
        .background(CaptureProbe(observer: model.capture))
        .simultaneousGesture(DragGesture(minimumDistance: 0).onChanged { _ in MainActor.assumeIsolated { model.userActivity() } })
        .privacyCover(model.privacy)
    }

    @ViewBuilder
    private var content: some View {
        switch model.phase {
        case .starting:
            ProgressView()
        case let .failed(message):
            MosaicScreen {
                ScreenTitle(text: L10n.appName())
                ErrorLine(text: message)
            }
        case .ready:
            ready
        }
    }

    @ViewBuilder
    private var ready: some View {
        if let onboarding = model.onboarding {
            OnboardingFlow(model: onboarding, capture: model.capture)
        } else {
            switch model.session {
            case let .locked(email, secretKeyStored, availability):
                if let unlock = model.unlock {
                    UnlockView(
                        model: unlock, email: email, secretKeyStored: secretKeyStored,
                        availability: availability)
                }
            case let .unlocked(_, email, _):
                VaultHomeView(model: model, email: email)
            case .storeBusy:
                MosaicScreen {
                    ScreenTitle(text: L10n.appName())
                    BodyText(text: L10n.unlockStoreBusy())
                    Button(L10n.commonTryAgain()) { Task { await model.reopenStore() } }
                        .buttonStyle(PrimaryButtonStyle())
                }
            case .noAccount, .storeClosed, .busy:
                ProgressView()
            }
        }
    }
}
