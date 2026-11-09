/** What the rotation variants change in the sync mock (a re-key made on another device). */
import type { MockState } from "./state.ts";
import type { RotationVariant } from "./rotation.ts";

const UNKNOWN_FILE_HASH = "ab".repeat(32);

export function applyRotationToSync(state: MockState, variant: RotationVariant | undefined): void {
  const { sync } = state;
  switch (variant) {
    case "rekeyElsewhere":
      sync.status = "signedOut";
      sync.error = "syncSignedOut";
      sync.passwordChanged = true;
      break;
    case "rekeyPending":
      sync.status = "error";
      sync.error = "syncAccountRekeyed";
      break;
    case "unknownFile":
    case "conflict":
      sync.warnings = [
        ...sync.warnings,
        {
          kind: "unverifiedAccountFile",
          name: variant === "conflict" ? "account-00000001.macct" : "account-00000002.macct",
          sha256: UNKNOWN_FILE_HASH,
          conflict: variant === "conflict",
        },
      ];
      break;
    default:
      break;
  }
}
