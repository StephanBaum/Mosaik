import MosaicKit
import Observation
import SwiftUI

/// The unlocked vault as the tabs see it: the item list, the sync state and the settings. Every call goes
/// through the core's serial queue (`CoreHolder.call`). It exists while the vault is unlocked.
@MainActor
@Observable
final class VaultModel {
    private(set) var items: [ItemSummary]?
    private(set) var overview: SyncOverview?
    private(set) var settings: MobileSettings?
    private(set) var failureCode: String?
    /// The error of "I've checked", shown in Settings where the button is, not on the Vault tab.
    private(set) var acknowledgeFailureCode: String?
    private(set) var isRefreshing = false
    /// Bumped by every change, so a screen that shows an item loads it again.
    private(set) var version = 0

    let holder: CoreHolder
    @ObservationIgnored private unowned let app: AppModel
    @ObservationIgnored private var lastAutoSync: ContinuousClock.Instant?
    @ObservationIgnored private let identityStore: any CredentialIdentityStoring

    init(app: AppModel, holder: CoreHolder, identityStore: any CredentialIdentityStoring = SystemIdentityStore()) {
        self.app = app
        self.holder = holder
        self.identityStore = identityStore
    }

    var clipboardSeconds: Int { Int(settings?.clipboardClearSeconds ?? 30) }

    /// Loads the list, brings the identity store in step with the vault, then syncs (the model is made on every
    /// unlock, so this is the sync after an unlock and after the join).
    func start() async {
        await load()
        await syncIdentities()
        await autoSync()
    }

    /// The automatic sync (after the join, after an unlock, on a return to the foreground). At most one sync runs
    /// at a time (`isRefreshing`), an automatic one waits at least 20 seconds after the last, and none runs unless
    /// the session is unlocked (M10-G4 item 1). `force` skips the rate limit only.
    func autoSync(force: Bool = false) async {
        guard !isRefreshing else { return }
        guard case .unlocked = app.session else { return }
        let now = ContinuousClock.now
        if !force, let last = lastAutoSync, now - last < .seconds(20) { return }
        lastAutoSync = now
        await refresh()
    }

    func load() async {
        do {
            let loaded = try await holder.call { core in
                let filter = ItemFilter(vaultId: nil, trash: .exclude, favoritesOnly: false, kind: nil)
                let list = try core.items(filter: filter)
                return VaultSnapshot(items: list, overview: core.syncOverview(), settings: core.settings())
            }
            items = loaded.items
            overview = loaded.overview
            settings = loaded.settings
            failureCode = nil
        } catch {
            failureCode = ErrorText.code(of: error)
        }
    }

    /// Pull to refresh and "Sync now": sync, load again, then update the identity store.
    func refresh() async {
        guard !isRefreshing else { return }
        isRefreshing = true
        defer { isRefreshing = false }
        do {
            overview = try await holder.call { core in try core.syncNow() }
            failureCode = nil
        } catch {
            let code = ErrorText.code(of: error)
            if code != "syncNotConfigured" {
                failureCode = code
            }
        }
        await load()
        await syncIdentities()
    }

    /// The user checked the sync findings: clears them (an unlocked session), then syncs again.
    func acknowledgeSyncFindings() async {
        guard !isRefreshing else { return }
        isRefreshing = true
        defer { isRefreshing = false }
        acknowledgeFailureCode = nil
        do {
            overview = try await holder.call { core in try core.acknowledgeSyncFindings() }
            overview = try await holder.call { core in try core.syncNow() }
        } catch {
            acknowledgeFailureCode = ErrorText.code(of: error)
        }
        await load()
    }

    /// An item was saved, trashed or marked: reload and tell the system.
    func changed() {
        version += 1
        Task { [self] in
            await load()
            await syncIdentities()
        }
    }

    func search(_ query: String) async throws -> [ItemSummary] {
        try await holder.call { core in try core.search(query: query) }
    }

    func firstVaultId() async throws -> String? {
        try await holder.call { core in try core.vaults().first(where: { $0.available })?.vaultId }
    }

    func syncIdentities() async {
        await IdentityStoreSync.run(holder: holder, store: identityStore)
    }

    /// Applies a settings change. `reauthPassword` is needed to turn the password identity store on.
    func updateSettings(_ update: SettingsUpdate) async throws {
        // A re-authentication runs Argon2id: the same memory preflight as an unlock (CRYPTO_SPEC 6.9.4).
        if update.reauthPassword != nil {
            try await app.preflightMemory()
        }
        let next = try await holder.call { core in try core.updateSettings(update: update) }
        settings = next
        app.idleMinutesChanged(Int(next.idleLockMinutes))
        await syncIdentities()
    }
}

/// What one load brings back.
struct VaultSnapshot: Sendable {
    let items: [ItemSummary]
    let overview: SyncOverview
    let settings: MobileSettings
}
