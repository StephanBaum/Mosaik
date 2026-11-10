/**
 * The unlocked app (Main, D08 to D10, D19): the sidebar, the top bar and the main area
 * (vault and trash lists, security, generator, settings, import). The palette and the
 * SSH approval dialog live here too.
 */
import type { MessageKey } from "@mosaic/i18n";
import { useState } from "react";
import { useEvents } from "../../app/EventsProvider.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useLandmarkId } from "../../reveal/context.ts";
import type {
  InsightFinding,
  ItemDetail as Detail,
  SessionState,
  VaultSummary,
} from "../../ipc/contract/index.ts";
import { CommandPalette } from "../CommandPalette.tsx";
import { GeneratorPanel } from "../GeneratorPanel.tsx";
import { ImportFlow } from "../import/ImportFlow.tsx";
import { SecurityPage } from "../rotation/SecurityPage.tsx";
import { useRotationReminder } from "../rotation/useRotationReminder.ts";
import { SettingsScreen } from "../SettingsScreen.tsx";
import { ApprovalDialogs } from "../ApprovalDialogs.tsx";
import { RekeyNotice } from "../settings/security/RekeyRecovery.tsx";
import { RotationResumeBanner } from "../settings/security/RotationResumeBanner.tsx";
import { EmergencyRequestBanner } from "../sharing/EmergencyRequestBanner.tsx";
import { InvitesInbox } from "../sharing/InvitesInbox.tsx";
import { SharingFindings } from "../sharing/SharingFindings.tsx";
import { useVaultSharing } from "../sharing/useVaultSharing.tsx";
import { ListView } from "./ListView.tsx";
import { CreateVaultDialog } from "./CreateVaultDialog.tsx";
import { Sidebar, vaultLabel } from "./Sidebar.tsx";
import { TopBar } from "./TopBar.tsx";
import { useHardwareVaultClosed } from "./useHardwareVaultClosed.ts";
import { useShell, type Shell } from "./useShell.ts";
import {
  useCategoryCounts,
  useFindings,
  useVaults,
  type Category,
} from "./useVaultData.ts";
import { titleOrUntitled } from "./title.ts";

export type View = "vault" | "trash" | "security" | "generator" | "settings" | "import" | "invites";

export type Editor = { mode: "closed" } | { mode: "new" } | { mode: "edit"; detail: Detail };

/** The header's "New item" is the holo primary unless the page has a primary of its own. */
function hasHeaderPrimary(state: {
  editor: Editor;
  view: View;
  itemCount: number | null;
  query: string;
  category: Category;
}): boolean {
  const { editor, view, itemCount, query, category } = state;
  const emptyVault = view === "vault" && itemCount === 0 && query === "" && category === "all";
  return !(editor.mode !== "closed" || view === "import" || view === "generator" || emptyVault);
}

type Translate = (key: MessageKey) => string;

function editorTitleOf(editor: Editor, t: Translate): string | null {
  if (editor.mode === "new") {
    return t("vault.new");
  }
  return editor.mode === "edit" ? (titleOrUntitled(editor.detail.summary.title, t("vault.untitled"))) : null;
}

/** The pages that are not a list: security, generator, settings, import and the invites. */
function Page({ view, shell }: { view: View; shell: Shell }) {
  return (
    <div className="page">
      {view === "security" && <SecurityPage onOpenItem={shell.open} />}
      {view === "generator" && <GeneratorPanel />}
      {view === "settings" && <SettingsScreen />}
      {view === "invites" && (
        <InvitesInbox
          onOpenSettings={() => {
            shell.go("settings");
          }}
        />
      )}
      {view === "import" && (
        <ImportFlow
          embedded
          onClose={() => {
            shell.changed();
            shell.go("vault");
          }}
        />
      )}
    </div>
  );
}

function MainArea({
  shell,
  vaults,
  findings,
}: {
  shell: Shell;
  vaults: readonly VaultSummary[];
  findings: readonly InsightFinding[];
}) {
  const { view, query, category, editor, itemCount } = shell;
  const listing = view === "vault" || view === "trash";
  const mainId = useLandmarkId("main");
  useRotationReminder(() => {
    shell.go("security");
  });
  return (
    <main id={mainId} tabIndex={-1} className="vault__main">
      <div className="vault__notices">
        <EmergencyRequestBanner />
        <SharingFindings />
        <RekeyNotice />
        <RotationResumeBanner />
      </div>
      {listing && (
        <ListView
          vaults={vaults}
          vaultId={shell.vaultId}
          trash={view === "trash"}
          query={query}
          category={category}
          findings={findings}
          selected={shell.selected}
          editor={editor}
          reloadKey={shell.reloadKey}
          empty={
            itemCount === 0 &&
            query.trim() === "" &&
            category === "all" &&
            view === "vault" &&
            editor.mode === "closed"
          }
          onSelect={shell.setSelected}
          onEditor={shell.setEditor}
          onChanged={shell.changed}
          onImport={() => {
            shell.go("import");
          }}
          onLoaded={shell.setItemCount}
        />
      )}
      {!listing && <Page view={view} shell={shell} />}
    </main>
  );
}

function labelOf(vault: VaultSummary | undefined, t: Translate): string {
  return vault === undefined ? t("vault.unnamed") : vaultLabel(vault, t);
}

/** The name the breadcrumb and the sidebar show for the open vault (or all of them). */
function nameOf(vaults: readonly VaultSummary[], vaultId: string | null, t: Translate): string {
  return vaultId === null ? t("vault.allVaults") : labelOf(vaults.find((v) => v.vaultId === vaultId), t);
}

export function VaultLayout({ onLocked }: { onLocked: (session: SessionState) => void }) {
  const events = useEvents();
  const { t } = useI18n();
  const shell = useShell(onLocked);
  const { view, vaultId, category, query, editor, itemCount } = shell;
  const vaults = useVaults(shell.reloadKey);
  const [creating, setCreating] = useState(false);
  const share = useVaultSharing(vaults, vaultId, view === "vault", shell.changed);
  useHardwareVaultClosed(shell);
  const counts = useCategoryCounts(vaultId, shell.reloadKey);
  const findings = useFindings(view === "security");
  const vaultName = nameOf(vaults, vaultId, t);

  return (
    <div className="vault">
      <Sidebar
        vaults={vaults}
        vaultId={vaultId}
        vaultName={vaultName}
        view={view}
        category={category}
        counts={counts}
        findingCount={findings.length}
        onCategory={(next) => {
          shell.setCategory(next);
          shell.go("vault");
        }}
        onVault={(id) => {
          shell.setVaultId(id);
          shell.go("vault");
        }}
        onView={shell.go}
        onLock={shell.lock}
        onNewVault={() => {
          setCreating(true);
        }}
      />
      <TopBar
        vaultName={vaultName}
        view={view}
        category={category}
        editorTitle={editorTitleOf(editor, t)}
        query={query}
        headerPrimary={hasHeaderPrimary({ editor, view, itemCount, query, category })}
        onQuery={shell.search}
        onNew={shell.startNew}
        onPalette={() => {
          events.setPaletteOpen(true);
        }}
        onShare={share.open}
      />
      <MainArea shell={shell} vaults={vaults} findings={findings} />
      {events.paletteOpen && (
        <CommandPalette
          onClose={() => {
            events.setPaletteOpen(false);
          }}
          onOpenItem={shell.open}
          onNavigate={shell.go}
          onNewItem={shell.startNew}
          onLock={shell.lock}
          onShare={share.open}
        />
      )}
      {creating && (
        <CreateVaultDialog
          onCreated={(vault) => {
            shell.changed();
            shell.setVaultId(vault.vaultId);
            shell.go("vault");
          }}
          onClose={() => {
            setCreating(false);
          }}
        />
      )}
      {share.dialog}
      <ApprovalDialogs />
    </div>
  );
}
