/**
 * The approval prompts that can appear over the vault: SSH signing, browser pairing, saving a
 * login, passkeys, password changes, and the agent endpoint's pairing and access requests.
 */
import { AgentAccessDialog } from "./AgentAccessDialog.tsx";
import { AgentPairingDialog } from "./AgentPairingDialog.tsx";
import { BrowserPairingDialog } from "./BrowserPairingDialog.tsx";
import { BrowserPasskeyDialog } from "./BrowserPasskeyDialog.tsx";
import { BrowserRotationDialog } from "./BrowserRotationDialog.tsx";
import { BrowserSaveDialog } from "./BrowserSaveDialog.tsx";
import { SshApprovalDialog } from "./SshApprovalDialog.tsx";

export function ApprovalDialogs() {
  return (
    <>
      <SshApprovalDialog />
      <AgentPairingDialog />
      <AgentAccessDialog />
      <BrowserPairingDialog />
      <BrowserSaveDialog />
      <BrowserPasskeyDialog />
      <BrowserRotationDialog />
    </>
  );
}
