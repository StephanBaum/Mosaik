import XCTest

@testable import MosaicKit

/// The usage descriptions of the camera and Face ID are localized through the `InfoPlist` String Catalog.
/// This reads the table of the built app (the host of the tests), so a build that does not ship it fails here.
final class InfoPlistTests: XCTestCase {
    private let keys = [
        ("NSFaceIDUsageDescription", "mobile.unlock.faceIdReason"),
        ("NSCameraUsageDescription", "mobile.scan.rationale"),
    ]

    func testBuiltBundleShipsTheInfoPlistTableInBothLanguages() throws {
        for language in ["en", "de"] {
            let folder = try XCTUnwrap(
                Bundle.main.path(forResource: language, ofType: "lproj"), "no \(language).lproj in the app")
            let bundle = try XCTUnwrap(Bundle(path: folder))
            for (plistKey, catalogKey) in keys {
                let value = bundle.localizedString(forKey: plistKey, value: "<missing>", table: "InfoPlist")
                XCTAssertNotEqual(value, "<missing>", "\(plistKey) is not in the \(language) InfoPlist table")
                // The same sentence the catalog has for the in-app explanation.
                let catalog = bundle.localizedString(forKey: catalogKey, value: nil, table: nil)
                XCTAssertEqual(value, catalog, "\(plistKey) in \(language)")
            }
        }
    }

    /// Quick unlock runs in the AutoFill extension, so its bundle ships the Face ID text in both languages too.
    func testExtensionBundleShipsTheFaceIdTextInBothLanguages() throws {
        let plugIns = try XCTUnwrap(Bundle.main.builtInPlugInsURL, "the app has no PlugIns folder")
        let appex = plugIns.appendingPathComponent("MosaicAutoFill.appex")
        let bundle = try XCTUnwrap(Bundle(url: appex), "MosaicAutoFill.appex is not embedded in the app")
        XCTAssertFalse(
            (bundle.object(forInfoDictionaryKey: "NSFaceIDUsageDescription") as? String ?? "").isEmpty)
        var seen: [String] = []
        for language in ["en", "de"] {
            let folder = try XCTUnwrap(bundle.path(forResource: language, ofType: "lproj"), "no \(language).lproj")
            let localized = try XCTUnwrap(Bundle(path: folder))
            let value = localized.localizedString(forKey: "NSFaceIDUsageDescription", value: "<missing>", table: "InfoPlist")
            XCTAssertNotEqual(value, "<missing>", "NSFaceIDUsageDescription is not in the extension's \(language) table")
            // The same sentence the catalog has for the in-app explanation.
            let catalog = localized.localizedString(forKey: "mobile.unlock.faceIdReason", value: nil, table: nil)
            XCTAssertEqual(value, catalog, language)
            seen.append(value)
        }
        XCTAssertNotEqual(seen[0], seen[1])
    }

    func testGermanTableIsGerman() throws {
        let german = try XCTUnwrap(Bundle.main.path(forResource: "de", ofType: "lproj"))
        let english = try XCTUnwrap(Bundle.main.path(forResource: "en", ofType: "lproj"))
        let germanBundle = try XCTUnwrap(Bundle(path: german))
        let englishBundle = try XCTUnwrap(Bundle(path: english))
        for (plistKey, _) in keys {
            XCTAssertNotEqual(
                germanBundle.localizedString(forKey: plistKey, value: nil, table: "InfoPlist"),
                englishBundle.localizedString(forKey: plistKey, value: nil, table: "InfoPlist"), plistKey)
        }
    }

    func testInfoDictionaryDeclaresTheCameraUse() {
        let text = Bundle.main.object(forInfoDictionaryKey: "NSCameraUsageDescription") as? String
        XCTAssertFalse((text ?? "").isEmpty)
    }
}
