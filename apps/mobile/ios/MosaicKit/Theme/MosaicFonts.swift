import CoreText
import Foundation
import SwiftUI
import UIKit

/// The bundled fonts (apps/mobile/shared/fonts, SIL OFL 1.1): DM Sans for the interface, IBM Plex Mono for
/// secrets and codes. They are resources of the app, registered for the process at launch.
public enum MosaicFonts {
    /// PostScript names of the default instances. The family names of `MosaicTokens.Fonts` are the design
    /// names; the variable DM Sans file registers its family as "DM Sans 9pt", so the PostScript name is the
    /// reliable handle.
    public static let uiPostScriptName = "DMSans-9ptRegular"
    public static let monoPostScriptName = "IBMPlexMono-Regular"

    /// Registers every `.ttf` in `bundle` for this process. A font that fails is skipped: the text then
    /// falls back to the system font.
    @discardableResult
    public static func register(in bundle: Bundle = .main) -> Int {
        let urls = bundle.urls(forResourcesWithExtension: "ttf", subdirectory: nil) ?? []
        var registered = 0
        for url in urls {
            var error: Unmanaged<CFError>?
            if CTFontManagerRegisterFontsForURL(url as CFURL, .process, &error) {
                registered += 1
            } else {
                error?.release()
            }
        }
        return registered
    }

    public static func uiFont(size: CGFloat, weight: UIFont.Weight = .regular) -> UIFont {
        guard let base = UIFont(name: uiPostScriptName, size: size) else {
            return UIFont.systemFont(ofSize: size, weight: weight)
        }
        let descriptor = base.fontDescriptor.addingAttributes([
            .traits: [UIFontDescriptor.TraitKey.weight: weight.rawValue]
        ])
        return UIFont(descriptor: descriptor, size: size)
    }

    public static func monoFont(size: CGFloat) -> UIFont {
        UIFont(name: monoPostScriptName, size: size) ?? UIFont.monospacedSystemFont(ofSize: size, weight: .regular)
    }
}

extension Font {
    /// A token size in DM Sans, scaled with Dynamic Type.
    public static func mosaic(_ token: MosaicTextToken, weight: Font.Weight = .regular) -> Font {
        Font.custom(MosaicFonts.uiPostScriptName, size: token.size, relativeTo: .body).weight(weight)
    }

    /// A token size in IBM Plex Mono, for secrets and codes.
    public static func mosaicMono(_ token: MosaicTextToken) -> Font {
        Font.custom(MosaicFonts.monoPostScriptName, size: token.size, relativeTo: .body)
    }
}

extension ColorScheme {
    /// The semantic colors of this appearance.
    public var mosaic: MosaicColors {
        self == .dark ? MosaicTokens.dark : MosaicTokens.light
    }
}
