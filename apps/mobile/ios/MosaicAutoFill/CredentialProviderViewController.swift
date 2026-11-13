import AuthenticationServices
import MosaicKit
import SwiftUI
import UIKit

/// The Credential Provider extension (spec 3.11 and 3.12, iOS).
///
/// - It never fills without the user: `provideCredentialWithoutUserInteraction` always answers "user
///   interaction required", because every request is a new process with a locked vault.
/// - Every request opens the store, unlocks with Face ID or Touch ID (the user verification of a passkey
///   ceremony) and closes the store before the system hears the result (`ProviderModel.finish`).
/// - The core decides which logins may fill which site. This class reads the system's request objects into
///   plain values and hands the result back.
final class CredentialProviderViewController: ASCredentialProviderViewController {
    private var model: ProviderModel?
    private var host: UIViewController?

    override func viewDidLoad() {
        super.viewDidLoad()
        MosaicFonts.register()
    }

    // MARK: Lists

    override func prepareCredentialList(for serviceIdentifiers: [ASCredentialServiceIdentifier]) {
        start(.passwordList(identifiers: ProviderIdentifiers.map(serviceIdentifiers)))
    }

    override func prepareCredentialList(
        for serviceIdentifiers: [ASCredentialServiceIdentifier], requestParameters: ASPasskeyCredentialRequestParameters
    ) {
        start(
            .passkeyList(
                identifiers: ProviderIdentifiers.map(serviceIdentifiers),
                rpId: requestParameters.relyingPartyIdentifier, clientDataHash: requestParameters.clientDataHash,
                allowed: requestParameters.allowedCredentials))
    }

    @available(iOS 18.0, *)
    override func prepareOneTimeCodeCredentialList(for serviceIdentifiers: [ASCredentialServiceIdentifier]) {
        start(.oneTimeCodeList(identifiers: ProviderIdentifiers.map(serviceIdentifiers)))
    }

    // MARK: Single credentials

    /// Always refused: the vault is locked in every new process, and the unlock needs the user.
    override func provideCredentialWithoutUserInteraction(for credentialRequest: any ASCredentialRequest) {
        extensionContext.cancelRequest(withError: ASExtensionError(.userInteractionRequired))
    }

    override func prepareInterfaceToProvideCredential(for credentialRequest: any ASCredentialRequest) {
        if let passkey = credentialRequest as? ASPasskeyCredentialRequest,
            let identity = Self.identity(passkey.credentialIdentity, as: ASPasskeyCredentialIdentity.self)
        {
            start(
                .passkeyAssert(
                    rpId: identity.relyingPartyIdentifier, credentialId: identity.credentialID,
                    clientDataHash: passkey.clientDataHash))
        } else if let password = credentialRequest as? ASPasswordCredentialRequest,
            let identity = Self.identity(password.credentialIdentity, as: ASPasswordCredentialIdentity.self)
        {
            start(
                .passwordProvide(
                    identifiers: ProviderIdentifiers.map([identity.serviceIdentifier]),
                    record: IdentityStoreSync.parseRecordIdentifier(identity.recordIdentifier)))
        } else {
            extensionContext.cancelRequest(withError: ASExtensionError(.failed))
        }
    }

    override func prepareInterface(forPasskeyRegistration registrationRequest: any ASCredentialRequest) {
        guard let request = registrationRequest as? ASPasskeyCredentialRequest,
            let identity = Self.identity(request.credentialIdentity, as: ASPasskeyCredentialIdentity.self)
        else {
            extensionContext.cancelRequest(withError: ASExtensionError(.failed))
            return
        }
        start(
            .passkeyRegister(
                PasskeyRegistrationRequest(
                    rpId: identity.relyingPartyIdentifier, userName: identity.userName,
                    userHandle: identity.userHandle, clientDataHash: request.clientDataHash,
                    algorithms: request.supportedAlgorithms.map { Int64($0.rawValue) })))
    }

    // MARK: Configuration

    override func prepareInterfaceForExtensionConfiguration() {
        start(.configuration)
    }

    // MARK: Request and result

    /// The identity of a request as its concrete type. A function with an existential parameter, so the cast
    /// is the same whether the system types the property as the protocol or as the class.
    private static func identity<T: ASCredentialIdentity>(_ identity: any ASCredentialIdentity, as type: T.Type) -> T? {
        identity as? T
    }

    private func start(_ request: ProviderRequest) {
        model?.cancelSilently()
        let model = ProviderModel(
            request: request, makeSession: { try ProviderSession() },
            onFinish: { [weak self] outcome in self?.complete(outcome) })
        self.model = model
        install(ProviderRootView(model: model))
        Task { await model.start() }
    }

    private func install(_ root: ProviderRootView) {
        if let old = host {
            old.willMove(toParent: nil)
            old.view.removeFromSuperview()
            old.removeFromParent()
        }
        let hosting = UIHostingController(rootView: root)
        addChild(hosting)
        hosting.view.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(hosting.view)
        NSLayoutConstraint.activate([
            hosting.view.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            hosting.view.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            hosting.view.topAnchor.constraint(equalTo: view.topAnchor),
            hosting.view.bottomAnchor.constraint(equalTo: view.bottomAnchor),
        ])
        hosting.didMove(toParent: self)
        host = hosting
    }

    /// The store is already closed (`ProviderModel.finish`); this only tells the system.
    private func complete(_ outcome: ProviderOutcome) {
        switch outcome {
        case let .password(password):
            extensionContext.completeRequest(
                withSelectedCredential: ASPasswordCredential(user: password.user, password: password.password),
                completionHandler: nil)
        case let .oneTimeCode(code):
            if #available(iOS 18.0, *) {
                extensionContext.completeOneTimeCodeRequest(
                    using: ASOneTimeCodeCredential(code: code), completionHandler: nil)
            } else {
                extensionContext.cancelRequest(withError: ASExtensionError(.failed))
            }
        case let .assertion(assertion, rpId, clientDataHash):
            let credential = ASPasskeyAssertionCredential(
                userHandle: assertion.userHandle, relyingParty: rpId, signature: assertion.signature,
                clientDataHash: clientDataHash, authenticatorData: assertion.authenticatorData,
                credentialID: assertion.credentialId)
            extensionContext.completeAssertionRequest(using: credential, completionHandler: nil)
        case let .registration(registration, rpId, clientDataHash):
            let credential = ASPasskeyRegistrationCredential(
                relyingParty: rpId, clientDataHash: clientDataHash, credentialID: registration.credentialId,
                attestationObject: registration.attestationObject)
            extensionContext.completeRegistrationRequest(using: credential, completionHandler: nil)
        case .configurationDone:
            extensionContext.completeExtensionConfigurationRequest()
        case .cancelled:
            extensionContext.cancelRequest(withError: ASExtensionError(.userCanceled))
        case .credentialNotFound:
            extensionContext.cancelRequest(withError: ASExtensionError(.credentialIdentityNotFound))
        case .failed:
            extensionContext.cancelRequest(withError: ASExtensionError(.failed))
        }
    }
}
