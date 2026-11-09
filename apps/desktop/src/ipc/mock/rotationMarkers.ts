/**
 * Mock of the M7 fix-round commands: giving up a committed key job and the re-key notices a
 * folder shows (`src-tauri/src/sync/rotation_flow.rs`).
 */
import { fail, requireUnlocked, type MockHandlers } from "./state.ts";
import { publish } from "./syncData.ts";

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
    rotation.markers = rotation.markers.filter((m) => m.sha256 !== request.sha256);
    if (rotation.markers.length === 0 && rotation.rekeyPending) {
      rotation.rekeyPending = false;
      sync.status = "upToDate";
      sync.error = null;
    }
    return publish(state);
  },
};
