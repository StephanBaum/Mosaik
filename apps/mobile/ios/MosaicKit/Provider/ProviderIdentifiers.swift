import AuthenticationServices
import Foundation

/// Maps what the system gives the extension to what the core takes. No matching happens here: the core turns
/// a `ServiceIdentifier` into an origin (`fill_target_ios`) and decides which logins may fill it.
public enum ProviderIdentifiers {
    /// The domain and URL identifiers, in order. App identifiers carry no site and are dropped.
    public static func map(_ identifiers: [ASCredentialServiceIdentifier]) -> [ServiceIdentifier] {
        var result: [ServiceIdentifier] = []
        for identifier in identifiers {
            switch identifier.type {
            case .domain:
                result.append(ServiceIdentifier(kind: .domain, value: identifier.identifier))
            case .URL:
                result.append(ServiceIdentifier(kind: .url, value: identifier.identifier))
            default:
                continue
            }
        }
        return result
    }
}
