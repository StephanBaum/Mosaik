/**
 * "Change master password": the screen that was missing since M6. It is the wizard of a password
 * change (review, the passwords, done) over local, hosted and folder accounts.
 */
import { RotationWizard } from "./RotationWizard.tsx";

export function ChangePasswordDialog({ onClose }: { onClose: (focusQuick: boolean) => void }) {
  return <RotationWizard kind="password" onClose={onClose} />;
}
