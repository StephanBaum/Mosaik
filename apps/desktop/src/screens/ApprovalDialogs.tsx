/**
 * The approval prompts that can appear over the vault: SSH signing, browser pairing, saving a
 * login and passkeys.
 */
import { BrowserPairingDialog } from "./BrowserPairingDialog.tsx";
import { BrowserPasskeyDialog } from "./BrowserPasskeyDialog.tsx";
import { BrowserSaveDialog } from "./BrowserSaveDialog.tsx";
import { SshApprovalDialog } from "./SshApprovalDialog.tsx";

export function ApprovalDialogs() {
  return (
    <>
      <SshApprovalDialog />
      <BrowserPairingDialog />
      <BrowserSaveDialog />
      <BrowserPasskeyDialog />
    </>
  );
}
