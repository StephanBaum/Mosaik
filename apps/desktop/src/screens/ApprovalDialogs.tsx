/** The approval prompts that can appear over the vault: SSH signing, browser pairing and saving. */
import { BrowserPairingDialog } from "./BrowserPairingDialog.tsx";
import { BrowserSaveDialog } from "./BrowserSaveDialog.tsx";
import { SshApprovalDialog } from "./SshApprovalDialog.tsx";

export function ApprovalDialogs() {
  return (
    <>
      <SshApprovalDialog />
      <BrowserPairingDialog />
      <BrowserSaveDialog />
    </>
  );
}
