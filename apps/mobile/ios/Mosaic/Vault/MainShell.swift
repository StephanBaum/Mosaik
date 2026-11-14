import MosaicKit
import SwiftUI

/// Where a push on a tab goes.
enum VaultRoute: Hashable {
    case detail(vaultId: String, itemId: String)
    case editLogin(vaultId: String?, itemId: String?)
    case editNote(vaultId: String?, itemId: String?)
}

/// The unlocked app: Vault, Search and Settings, as on Android (spec T8, T11).
struct MainShell: View {
    @Environment(\.colorScheme) private var colorScheme
    let app: AppModel
    let vault: VaultModel
    @State private var tab = ShellTab.vault

    private var hasSyncWarnings: Bool { !SyncWarnings.texts(for: vault.overview).isEmpty }

    enum ShellTab: Hashable {
        case vault
        case search
        case settings
    }

    var body: some View {
        TabView(selection: $tab) {
            VaultTab(app: app, vault: vault, onOpenSettings: { tab = .settings })
                .tabItem { Label(L10n.navVault(), systemImage: "lock.rectangle.stack").accessibilityLabel(L10n.navVault()) }
                .tag(ShellTab.vault)
            SearchTab(app: app, vault: vault)
                .tabItem { Label(L10n.navSearch(), systemImage: "magnifyingglass").accessibilityLabel(L10n.navSearch()) }
                .tag(ShellTab.search)
            SettingsTab(app: app, vault: vault)
                .tabItem { Label(L10n.navSettings(), systemImage: "gearshape").accessibilityLabel(L10n.navSettings()) }
                .badge(hasSyncWarnings ? Text("!") : nil)
                .tag(ShellTab.settings)
        }
        .tint(colorScheme.mosaic.accent)
    }
}

/// The screens a route opens. Both list tabs use it.
private struct RouteDestination: View {
    let route: VaultRoute
    let app: AppModel
    let vault: VaultModel
    @Binding var path: [VaultRoute]

    var body: some View {
        switch route {
        case let .detail(vaultId, itemId):
            DetailScreen(
                app: app, vault: vault, vaultId: vaultId, itemId: itemId,
                onEdit: { kind in
                    path.append(
                        kind == .secureNote
                            ? .editNote(vaultId: vaultId, itemId: itemId)
                            : .editLogin(vaultId: vaultId, itemId: itemId))
                },
                onClose: { path.removeAll() })
        case let .editLogin(vaultId, itemId):
            LoginEditorScreen(
                app: app, vault: vault, vaultId: vaultId, itemId: itemId,
                onSaved: { key in saved(key, existing: itemId != nil) })
        case let .editNote(vaultId, itemId):
            NoteEditorScreen(
                vault: vault, vaultId: vaultId, itemId: itemId,
                onSaved: { key in saved(key, existing: itemId != nil) })
        }
    }

    /// A new item goes back to the list; an edited one goes back to its detail.
    private func saved(_ key: ItemKey, existing: Bool) {
        vault.changed()
        if existing {
            path.removeLast()
        } else {
            path = [.detail(vaultId: key.vaultId, itemId: key.itemId)]
        }
    }
}

/// The screens keep their model in `@State`: a pushed screen is rebuilt whenever its parent is, and the model
/// (revealed values, typed text) must survive that.
@MainActor
private struct DetailScreen: View {
    let app: AppModel
    let vault: VaultModel
    let onEdit: (ItemKind) -> Void
    let onClose: () -> Void
    @State private var model: ItemDetailModel

    init(
        app: AppModel, vault: VaultModel, vaultId: String, itemId: String, onEdit: @escaping (ItemKind) -> Void,
        onClose: @escaping () -> Void
    ) {
        self.app = app
        self.vault = vault
        self.onEdit = onEdit
        self.onClose = onClose
        _model = State(initialValue: ItemDetailModel(app: app, vault: vault, vaultId: vaultId, itemId: itemId))
    }

    var body: some View {
        ItemDetailView(app: app, vault: vault, model: model, onEdit: onEdit, onClose: onClose)
    }
}

@MainActor
private struct LoginEditorScreen: View {
    let app: AppModel
    let vault: VaultModel
    let onSaved: (ItemKey) -> Void
    @State private var model: LoginEditorModel

    init(
        app: AppModel, vault: VaultModel, vaultId: String?, itemId: String?, onSaved: @escaping (ItemKey) -> Void
    ) {
        self.app = app
        self.vault = vault
        self.onSaved = onSaved
        _model = State(initialValue: LoginEditorModel(app: app, vault: vault, vaultId: vaultId, itemId: itemId))
    }

    var body: some View {
        LoginEditorView(model: model, app: app, vault: vault, onSaved: onSaved)
    }
}

@MainActor
private struct NoteEditorScreen: View {
    let onSaved: (ItemKey) -> Void
    @State private var model: NoteEditorModel

    init(vault: VaultModel, vaultId: String?, itemId: String?, onSaved: @escaping (ItemKey) -> Void) {
        self.onSaved = onSaved
        _model = State(initialValue: NoteEditorModel(vault: vault, vaultId: vaultId, itemId: itemId))
    }

    var body: some View {
        NoteEditorView(model: model, onSaved: onSaved)
    }
}

private struct VaultTab: View {
    let app: AppModel
    let vault: VaultModel
    let onOpenSettings: () -> Void
    @State private var path: [VaultRoute] = []

    var body: some View {
        NavigationStack(path: $path) {
            VaultListView(
                app: app, vault: vault,
                onOpen: { summary in
                    path.append(.detail(vaultId: summary.vaultId, itemId: summary.itemId))
                },
                onAdd: { login in
                    path.append(login ? .editLogin(vaultId: nil, itemId: nil) : .editNote(vaultId: nil, itemId: nil))
                },
                onOpenSettings: onOpenSettings)
                .navigationDestination(for: VaultRoute.self) { route in
                    RouteDestination(route: route, app: app, vault: vault, path: $path)
                }
        }
    }
}

private struct SearchTab: View {
    let app: AppModel
    let vault: VaultModel
    @State private var path: [VaultRoute] = []

    var body: some View {
        NavigationStack(path: $path) {
            SearchView(
                vault: vault,
                onOpen: { summary in
                    path.append(.detail(vaultId: summary.vaultId, itemId: summary.itemId))
                })
                .navigationDestination(for: VaultRoute.self) { route in
                    RouteDestination(route: route, app: app, vault: vault, path: $path)
                }
        }
    }
}

private struct SettingsTab: View {
    let app: AppModel
    let vault: VaultModel

    var body: some View {
        NavigationStack {
            SettingsView(app: app, vault: vault)
        }
    }
}
