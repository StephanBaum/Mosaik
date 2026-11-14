import MosaicKit
import SwiftUI

/// The onboarding screens (spec 3.14). One primary action per screen.
struct OnboardingFlow: View {
    @Bindable var model: OnboardingModel
    let capture: CaptureObserver
    let sceneActive: Bool

    var body: some View {
        switch model.step {
        case .welcome:
            WelcomeStep(model: model)
        case .details:
            if model.scanning, let scan = model.scanModel {
                SetupScanScreen(model: scan, sceneActive: sceneActive, onBack: { model.stopScanning() })
            } else {
                DetailsStep(model: model, capture: capture)
            }
        case .password:
            PasswordStep(model: model)
        case .secondFactor:
            SecondFactorStep(model: model)
        case .securityKey:
            SecurityKeyStep(model: model)
        case .quickPassword:
            QuickPasswordStep(model: model)
        case .autofill:
            AutofillStep(model: model)
        case .done:
            DoneStep(model: model)
        }
    }
}

private struct WelcomeStep: View {
    @Environment(\.colorScheme) private var colorScheme
    let model: OnboardingModel

    var body: some View {
        MosaicScreen {
            Spacer(minLength: MosaicTokens.Space.s8)
            ScreenTitle(text: L10n.onboardingWelcomeTitle())
                .accessibilityIdentifier("onboarding-welcome")
            BodyText(text: L10n.onboardingWelcomeBody())
            Button(L10n.onboardingStart()) { model.start() }
                .buttonStyle(PrimaryButtonStyle())
                .accessibilityIdentifier("onboarding-start")
            BodyText(text: L10n.onboardingPreAudit(), muted: true)
            Text(L10n.settingsVersion(version: coreVersion()))
                .font(Font.mosaic(MosaicTokens.FontSize.xs))
                .foregroundStyle(colorScheme.mosaic.textMuted)
                .accessibilityIdentifier("core-version")
        }
    }
}

private struct DetailsStep: View {
    @Bindable var model: OnboardingModel
    let capture: CaptureObserver

    var body: some View {
        MosaicScreen {
            ScreenTitle(text: L10n.onboardingDetailsTitle())
            Button(L10n.scanSetupAction()) { model.startScanning() }
                .buttonStyle(SecondaryButtonStyle())
                .disabled(model.isWorking)
                .accessibilityIdentifier("onboarding-scan")
            HintText(text: model.keyFromCode ? L10n.onboardingDetailsScanned() : L10n.onboardingDetailsScanOr())
            PlainField(
                label: L10n.onboardingServer(), placeholder: L10n.onboardingServerPlaceholder(),
                text: $model.server, identifier: "onboarding-server", keyboard: .URL, contentType: .URL)
            PlainField(
                label: L10n.onboardingEmail(), placeholder: "", text: $model.email,
                identifier: "onboarding-email", keyboard: .emailAddress, contentType: .emailAddress)
            if model.keyFromCode {
                VStack(alignment: .leading, spacing: MosaicTokens.Space.s3) {
                    FieldLabel(text: L10n.onboardingSecretKey())
                    BodyText(text: L10n.onboardingDetailsKeyFromCode())
                        .accessibilityIdentifier("onboarding-key-from-code")
                    Button(L10n.commonRemove()) { model.discardScannedKey() }
                        .buttonStyle(QuietButtonStyle())
                        .accessibilityIdentifier("onboarding-key-remove")
                }
            } else {
            VStack(alignment: .leading, spacing: MosaicTokens.Space.s3) {
                SecretField(
                    label: L10n.onboardingSecretKey(), buffer: model.secretKey,
                    identifier: "onboarding-secret-key", clearToken: model.secretKeyClearToken,
                    reveal: model.showSecretKey && capture.revealAllowed,
                    onChange: { empty in model.secretKeyFilled = !empty })
                if let note = capture.hiddenNote {
                    BodyText(text: note, muted: true)
                } else {
                    Toggle(isOn: $model.showSecretKey) {
                        Text(L10n.itemSecretRevealAction(field: L10n.onboardingSecretKey()))
                            .font(Font.mosaic(MosaicTokens.FontSize.sm))
                    }
                    .accessibilityIdentifier("onboarding-secret-key-reveal")
                }
                BodyText(text: L10n.onboardingSecretKeyHint(), muted: true)
            }
            }
            PlainField(
                label: L10n.onboardingDeviceName(), placeholder: "", text: $model.deviceName,
                identifier: "onboarding-device-name")
            if let message = model.errorMessage { ErrorLine(text: message) }
            if model.isWorking { BodyText(text: L10n.onboardingWorking(), muted: true) }
            Button(L10n.commonContinue()) { Task { await model.submitDetails() } }
                .buttonStyle(PrimaryButtonStyle())
                .disabled(!model.canSubmitDetails)
                .accessibilityIdentifier("onboarding-details-continue")
            Button(L10n.commonBack()) { model.back() }
                .buttonStyle(SecondaryButtonStyle())
        }
    }
}

private struct PasswordStep: View {
    @Bindable var model: OnboardingModel

    var body: some View {
        MosaicScreen {
            ScreenTitle(text: L10n.onboardingPasswordTitle())
            BodyText(text: L10n.onboardingPasswordHint())
            SecretField(
                label: L10n.unlockPasswordLabel(), buffer: model.password, identifier: "onboarding-password",
                clearToken: model.passwordClearToken,
                onChange: { empty in model.passwordFilled = !empty },
                onSubmit: { Task { await model.signIn() } })
            if model.canUseQuickUnlock {
                OptionRow(
                    text: L10n.unlockQuickOptionFaceId(), isOn: $model.enrollQuickUnlock,
                    identifier: "onboarding-quick-unlock")
            }
            if let message = model.errorMessage { ErrorLine(text: message) }
            if model.isWorking { BodyText(text: L10n.onboardingWorking(), muted: true) }
            Button(L10n.onboardingSignIn()) { Task { await model.signIn() } }
                .buttonStyle(PrimaryButtonStyle())
                .disabled(!model.canSignIn)
                .accessibilityIdentifier("onboarding-sign-in")
            Button(L10n.commonBack()) { model.back() }
                .buttonStyle(SecondaryButtonStyle())
        }
    }
}

private struct SecondFactorStep: View {
    @Bindable var model: OnboardingModel

    var body: some View {
        MosaicScreen {
            ScreenTitle(text: L10n.onboardingSecondFactorTitle())
            if model.secondFactor?.totp == true {
                PlainField(
                    label: L10n.onboardingTotpLabel(), placeholder: "", text: $model.totp,
                    identifier: "onboarding-totp", keyboard: .numberPad, contentType: .oneTimeCode)
            }
            if let message = model.errorMessage { ErrorLine(text: message) }
            if model.isWorking { BodyText(text: L10n.onboardingWorking(), muted: true) }
            if model.secondFactor?.totp == true {
                Button(L10n.onboardingVerify()) { Task { await model.submitTotp() } }
                    .buttonStyle(PrimaryButtonStyle())
                    .disabled(model.totp.isEmpty || model.isWorking)
                    .accessibilityIdentifier("onboarding-totp-verify")
            }
            if model.secondFactor?.webauthn == true {
                if model.secondFactor?.totp == true {
                    Button(L10n.onboardingSecurityKey()) { Task { await model.startSecurityKey() } }
                        .buttonStyle(SecondaryButtonStyle())
                        .disabled(model.isWorking)
                        .accessibilityIdentifier("onboarding-security-key")
                } else {
                    Button(L10n.onboardingSecurityKey()) { Task { await model.startSecurityKey() } }
                        .buttonStyle(PrimaryButtonStyle())
                        .disabled(model.isWorking)
                        .accessibilityIdentifier("onboarding-security-key")
                }
            }
            Button(L10n.commonBack()) { model.back() }
                .buttonStyle(SecondaryButtonStyle())
        }
    }
}

private struct SecurityKeyStep: View {
    let model: OnboardingModel

    var body: some View {
        MosaicScreen {
            ScreenTitle(text: L10n.onboardingSecurityKey())
            BodyText(text: L10n.onboardingSecurityKeyWaiting(code: model.handoffCode ?? ""))
            ProgressView()
                .accessibilityLabel(L10n.onboardingWorking())
            if let message = model.errorMessage { ErrorLine(text: message) }
            Button(L10n.commonCancel()) { model.back() }
                .buttonStyle(SecondaryButtonStyle())
        }
    }
}

private struct QuickPasswordStep: View {
    @Bindable var model: OnboardingModel

    var body: some View {
        MosaicScreen {
            ScreenTitle(text: L10n.onboardingQuickTitle())
            BodyText(text: L10n.settingsQuickPassword())
            SecretField(
                label: L10n.unlockPasswordLabel(), buffer: model.password, identifier: "onboarding-quick-password",
                clearToken: model.passwordClearToken,
                onChange: { empty in model.passwordFilled = !empty },
                onSubmit: { Task { await model.enrollQuickFromPrompt() } })
            if let message = model.errorMessage { ErrorLine(text: message) }
            if model.isWorking { BodyText(text: L10n.onboardingWorking(), muted: true) }
            Button(L10n.settingsQuickEnable()) { Task { await model.enrollQuickFromPrompt() } }
                .buttonStyle(PrimaryButtonStyle())
                .disabled(!model.passwordFilled || model.isWorking)
                .accessibilityIdentifier("onboarding-quick-enable")
            Button(L10n.onboardingNotNow()) { model.skipQuickPrompt() }
                .buttonStyle(SecondaryButtonStyle())
                .disabled(model.isWorking)
                .accessibilityIdentifier("onboarding-quick-skip")
        }
    }
}

private struct AutofillStep: View {
    let model: OnboardingModel

    var body: some View {
        MosaicScreen {
            ScreenTitle(text: L10n.onboardingAutofillTitle())
            BodyText(text: L10n.onboardingAutofillBody())
            if model.autofillSettingsAvailable {
                Button(L10n.onboardingOpenSettings()) { model.openAutofillSettings() }
                    .buttonStyle(PrimaryButtonStyle())
                    .accessibilityIdentifier("onboarding-open-autofill-settings")
            } else {
                BodyText(text: L10n.onboardingAutofillSteps())
            }
            if model.autofillSettingsAvailable {
                Button(L10n.onboardingNotNow()) { model.goToDone() }
                    .buttonStyle(SecondaryButtonStyle())
                    .accessibilityIdentifier("onboarding-autofill-continue")
            } else {
                Button(L10n.commonContinue()) { model.goToDone() }
                    .buttonStyle(PrimaryButtonStyle())
                    .accessibilityIdentifier("onboarding-autofill-continue")
            }
        }
    }
}

private struct DoneStep: View {
    let model: OnboardingModel

    var body: some View {
        MosaicScreen {
            ScreenTitle(text: L10n.onboardingDoneTitle())
            BodyText(text: L10n.onboardingDoneBody())
            Button(L10n.commonDone()) { model.finish() }
                .buttonStyle(PrimaryButtonStyle())
                .accessibilityIdentifier("onboarding-done")
        }
    }
}
