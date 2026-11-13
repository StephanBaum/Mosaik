import Foundation

/// The rules for a tap on a QuickType suggestion (G2 M3, ADR-0092 Amendment). iOS does not tell the provider
/// which page the user is on for such a selection: the request carries only the identity's own domain, so the
/// core's origin match would compare the identity with itself. The conservative answer is that the request
/// never fills by itself: it shows the saved site and fills only after the user confirms it, and after the
/// biometric unlock of the same request.
public enum ProviderFillPolicy {
    /// The login a password request may fill without a further tap. It is the identity's login only after the
    /// user confirmed the saved site, and only when the core still lists it as a match for that site.
    public static func loginToFill(record: ItemKey?, matches: [ItemKey], confirmed: Bool) -> ItemKey? {
        guard confirmed, let record, matches.contains(record) else { return nil }
        return record
    }

    /// The saved site to name in "Fill the login for ...?": the value of a `.domain` identifier. A `.url`
    /// identifier gives nil (the screen says "this site"): Swift never parses a URL, because Foundation and
    /// the core's WHATWG parser disagree and the screen could name another site than the core fills (G3i-3).
    public static func savedSite(of identifiers: [ServiceIdentifier]) -> String? {
        guard let first = identifiers.first else { return nil }
        switch first.kind {
        case .url:
            return nil
        default:
            return first.value
        }
    }
}
