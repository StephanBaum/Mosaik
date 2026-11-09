/**
 * Mock of the M7 fix-round commands: giving up a committed key job and the re-key notices a
 * folder shows (`src-tauri/src/sync/rotation_flow.rs`).
 */
import type { RekeyMarkerView } from "../contract/index.ts";
import type { RotationVariant } from "./rotation.ts";
import { fail, requireUnlocked, type MockHandlers } from "./state.ts";
import { publish } from "./syncData.ts";

function sampleMarker(): RekeyMarkerView {
  return { name: "rekey-00000002.mrk", sha256: "cd".repeat(32), toEpoch: 2 };
}

/** What a variant seeds: a halting notice, an ignored one, or a device too far behind. */
export function seedNotices(variant: RotationVariant): {
  rekeyPending: boolean;
  markers: RekeyMarkerView[];
  ignored: RekeyMarkerView[];
  jumpTooLarge: boolean;
} {
  const pending = variant === "rekeyPending" || variant === "jumpTooLarge";
  return {
    rekeyPending: pending,
    markers: pending ? [sampleMarker()] : [],
    ignored: variant === "ignoredNotices" ? [sampleMarker()] : [],
    jumpTooLarge: variant === "jumpTooLarge",
  };
}

export const markerHandlers: MockHandlers = {
  rotation_abandon_committed: (state) => {
    requireUnlocked(state);
    const { rotation } = state;
    if (rotation.job === null) {
      fail("rotationNone");
    }
    if (rotation.job.phase === "prepared" || rotation.job.phase === "backendStarted" || rotation.job.phase === "backendAccepted") {
      fail("rotationTooFar");
    }
    rotation.job = null;
    rotation.stuck = false;
    return null;
  },

  sync_rekey_markers: (state) => {
    requireUnlocked(state);
    return state.rotation.markers.map((marker) => ({ ...marker }));
  },

  sync_dismiss_rekey_marker: (state, request) => {
    requireUnlocked(state);
    const { rotation, sync } = state;
    const dismissed = rotation.markers.filter((m) => m.sha256 === request.sha256);
    rotation.ignored = [...dismissed, ...rotation.ignored];
    rotation.markers = rotation.markers.filter((m) => m.sha256 !== request.sha256);
    if (rotation.markers.length === 0 && rotation.rekeyPending) {
      rotation.rekeyPending = false;
      sync.status = "upToDate";
      sync.error = null;
    }
    return publish(state);
  },

  sync_ignored_rekey_markers: (state) => {
    requireUnlocked(state);
    return state.rotation.ignored.map((marker) => ({ ...marker }));
  },

  sync_restore_rekey_marker: (state, request) => {
    requireUnlocked(state);
    const { rotation, sync } = state;
    const restored = rotation.ignored.filter((m) => m.sha256 === request.sha256);
    if (restored.length > 0) {
      rotation.ignored = rotation.ignored.filter((m) => m.sha256 !== request.sha256);
      rotation.markers = [...restored, ...rotation.markers];
      rotation.rekeyPending = true;
      sync.status = "error";
      sync.error = "syncAccountRekeyed";
    }
    return publish(state);
  },
};
