import AuthenticationServices
import MosaicKit

/// T6 stub of the Credential Provider extension: it provides nothing yet and answers every request
/// with "user interaction required" (the real provider arrives in T12).
final class CredentialProviderViewController: ASCredentialProviderViewController {
    override func viewDidLoad() {
        super.viewDidLoad()
        // Links the shared framework (and with it the Rust core) into the extension.
        _ = coreVersion()
    }

    override func prepareCredentialList(for serviceIdentifiers: [ASCredentialServiceIdentifier]) {
        extensionContext.cancelRequest(withError: ASExtensionError(.userCanceled))
    }

    override func provideCredentialWithoutUserInteraction(for credentialRequest: any ASCredentialRequest) {
        extensionContext.cancelRequest(withError: ASExtensionError(.userInteractionRequired))
    }
}
