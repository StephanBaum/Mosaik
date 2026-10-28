/**
 * TS ⇄ Rust contract parity (ADR-0018). `tsc` already checks every Rust sample in
 * contract.fixtures.ts against the TypeScript types; these tests add the directions a type
 * check cannot see: the command list and every string enum must be equal as sets.
 */
import { catalogs, type MessageKey } from "@mosaic/i18n";
import { describe, expect, it } from "vitest";
import { fixtures } from "./contract.fixtures.ts";
import {
  channelCommands,
  commandNames,
  customFieldKinds,
  errorCodes,
  importFormats,
  importWarnings,
  insightActions,
  insightKinds,
  insightSeverities,
  itemKinds,
  languages,
  lockReasons,
  otpSchemes,
  quickUnlockKinds,
  secretFieldKeys,
  sshCloseReasons,
  sshDecisions,
  strengths,
  syncStatuses,
  textFieldKeys,
  themes,
  trashFilters,
  urlMatchModes,
  type ErrorCode,
} from "./contract/index.ts";
import type { EnumFixtures } from "./contract/fixtures.ts";

const sorted = (values: readonly string[]): string[] => [...values].sort();

describe("contract parity with Rust", () => {
  it("lists the same commands", () => {
    expect(sorted([...commandNames, ...channelCommands])).toEqual(sorted(fixtures.commands));
    expect(sorted(Object.keys(fixtures.requests))).toEqual(sorted(commandNames));
  });

  const enums: { [K in keyof EnumFixtures]: readonly string[] } = {
    errorCode: errorCodes,
    quickUnlockKind: quickUnlockKinds,
    itemKind: itemKinds,
    textFieldKey: textFieldKeys,
    secretFieldKey: secretFieldKeys,
    trashFilter: trashFilters,
    urlMatchMode: urlMatchModes,
    customFieldKind: customFieldKinds,
    otpScheme: otpSchemes,
    strength: strengths,
    theme: themes,
    language: languages,
    insightKind: insightKinds,
    insightSeverity: insightSeverities,
    insightAction: insightActions,
    importFormat: importFormats,
    importWarning: importWarnings,
    sshDecision: sshDecisions,
    sshCloseReason: sshCloseReasons,
    lockReason: lockReasons,
    syncStatus: syncStatuses,
  };

  it.each(Object.entries(enums))("%s has the same values", (name, values) => {
    expect(sorted(values)).toEqual(sorted(fixtures.enums[name as keyof EnumFixtures]));
  });

  it("has a sample for every error code", () => {
    expect(sorted(fixtures.errors.map((error) => error.code))).toEqual(sorted(errorCodes));
  });
});

describe("error messages", () => {
  // Compile-time: every code has a message key.
  const key = (code: ErrorCode): MessageKey => `error.${code}`;

  it.each(errorCodes)("error.%s exists in every locale", (code) => {
    expect(catalogs.en[key(code)]).toBeTruthy();
    expect(catalogs.de[key(code)]).toBeTruthy();
  });
});
