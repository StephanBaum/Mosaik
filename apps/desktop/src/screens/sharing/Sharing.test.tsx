import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../../App.tsx";
import { createMockIpc, type MockOptions } from "../../ipc/mock.ts";
import { KIM, LEE, SAM } from "../../ipc/mock/sharingData.ts";

type User = ReturnType<typeof userEvent.setup>;

async function open(options: MockOptions, language = "en") {
  const user = userEvent.setup();
  const ipc = createMockIpc({ start: "unlocked", sync: { variant: "hosted" }, ...options });
  render(<App ipc={ipc} languages={[language]} />);
  await screen.findByText("GitHub");
  return user;
}

async function openWork(user: User) {
  await user.click(screen.getByRole("button", { name: /^Work/ }));
  await user.click(await screen.findByRole("button", { name: "Share…" }));
  return screen.findByRole("dialog", { name: /Sharing: / });
}

describe("sharing a vault", () => {
  it("looks a person up, requires the code comparison, then sends the invite", async () => {
    const user = await open({ sharing: "empty" });
    const members = await openWork(user);
    expect(await within(members).findByText("Only you can open this vault so far.")).toBeTruthy();

    await user.click(within(members).getByRole("button", { name: "Invite someone…" }));
    const share = await screen.findByRole("dialog", { name: /^Share / });
    await user.type(within(share).getByLabelText("Their email address"), LEE.email);
    await user.click(within(share).getByRole("button", { name: "Look up" }));

    // The code is shown side by side with a field for the one they read out.
    expect(await within(share).findByText(/Code Mosaic shows for/)).toBeTruthy();
    expect(within(share).getByText("9DQ6K")).toBeTruthy();
    await user.type(within(share).getByLabelText("Code they read to you (optional)"), "9DQ6K 3MV7X 1WHT4 B8ZN2 C5RF0J");
    expect(within(share).getByText("This is the code above.")).toBeTruthy();
    await user.click(within(share).getByRole("button", { name: "It matches" }));

    // Verified: the role, with the honesty line, then Send.
    expect(await within(share).findByText(/Anyone who can read an item can copy it/)).toBeTruthy();
    await user.click(within(share).getByRole("radio", { name: /Editor/ }));
    await user.click(within(share).getByRole("button", { name: "Send invite" }));
    expect(await within(share).findByText(/Invite sent to/)).toBeTruthy();
    await user.click(within(share).getByRole("button", { name: "Done" }));

    expect(await within(members).findByText("Invited")).toBeTruthy();
  });

  it("refuses to continue while the typed code differs", async () => {
    const user = await open({ sharing: "empty" });
    const members = await openWork(user);
    await user.click(within(members).getByRole("button", { name: "Invite someone…" }));
    const share = await screen.findByRole("dialog", { name: /^Share / });
    await user.type(within(share).getByLabelText("Their email address"), KIM.email);
    await user.click(within(share).getByRole("button", { name: "Look up" }));
    await user.type(
      await within(share).findByLabelText("Code they read to you (optional)"),
      "AAAAA AAAAA AAAAA AAAAA AAAAAA",
    );
    expect(within(share).getByText(/The codes differ\. Don.t share/)).toBeTruthy();
    expect(within(share).getByRole<HTMLButtonElement>("button", { name: "It matches" }).disabled).toBe(true);
  });

  it("shows the old and the new code after a key change and verifies the new one", async () => {
    const user = await open({ sharing: "keyChanged" });
    const members = await openWork(user);
    await user.click(within(members).getByRole("button", { name: "Invite someone…" }));
    const share = await screen.findByRole("dialog", { name: /^Share / });
    await user.type(within(share).getByLabelText("Their email address"), "sam@example.com");
    await user.click(within(share).getByRole("button", { name: "Look up" }));
    expect(await within(share).findByText(/safety code changed/)).toBeTruthy();
    expect(within(share).getByText("Code you compared before")).toBeTruthy();
    expect(within(share).getByText("New code")).toBeTruthy();
    await user.click(within(share).getByRole("button", { name: "Compare and accept" }));
    // Sam is a member already: the invite is refused after the verification.
    expect(await within(share).findByText(/Anyone who can read an item can copy it/)).toBeTruthy();
    await user.click(within(share).getByRole("button", { name: "Send invite" }));
    expect(await within(share).findByText(/member of the vault already/)).toBeTruthy();
  });

  it("lists members with their roles, flags the ones nobody verified and removes one", async () => {
    const user = await open({ sharing: "rotation" });
    const members = await openWork(user);
    expect(await within(members).findByText(sampleOwnerEmail)).toBeTruthy();
    expect(within(members).getByText("Being removed")).toBeTruthy();
    expect(within(members).getByText(/waiting for the owner's device to change the vault key/)).toBeTruthy();
    expect(within(members).getByText(/2 files keep the old protection/)).toBeTruthy();
    expect(within(members).getAllByText("Not verified").length).toBeGreaterThan(0);

    await user.click(within(members).getByRole("button", { name: `Remove ${SAM.email}` }));
    await waitFor(() => {
      expect(within(members).queryByText(SAM.email)).toBeNull();
    });
  });

  it("tells a manager that only the owner's device changes the key", async () => {
    const user = await open({ sharing: "manager" });
    const members = await openWork(user);
    expect(await within(members).findByText(/Only the owner's device changes the vault key/)).toBeTruthy();
    expect(within(members).getByRole("button", { name: "Leave vault" })).toBeTruthy();
  });

  it("says why sharing is off with a sync folder", async () => {
    const user = await open({ sharing: "basic", sync: { variant: "folder" } });
    const members = await openWork(user);
    expect(within(members).getByText(/Sharing isn't available with a sync folder/)).toBeTruthy();
    expect(within(members).queryByRole("button", { name: "Invite someone…" })).toBeNull();
  });
});

const sampleOwnerEmail = "alex@example.com";

describe("the invites inbox", () => {
  it("shows the count, reviews an invite and joins after comparing the sender's code", async () => {
    const user = await open({ sharing: "invites" });
    await user.click(await screen.findByRole("button", { name: /Invites, 3 waiting/ }));
    const list = await screen.findByRole("list", { name: "Invites" });
    expect(within(list).getAllByRole("listitem")).toHaveLength(3);

    await user.click(within(list).getByRole("button", { name: `Review: ${SAM.email}` }));
    const dialog = await screen.findByRole("dialog", { name: /Join .*Family/ });
    expect(await within(dialog).findByText(/invites you as Editor/)).toBeTruthy();
    // Joining is not offered before the code was compared.
    expect(within(dialog).queryByRole("button", { name: "Join vault" })).toBeNull();
    await user.click(within(dialog).getByRole("button", { name: "It matches" }));
    await user.click(await within(dialog).findByRole("button", { name: "Join vault" }));

    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: /Join / })).toBeNull();
    });
    expect(await screen.findByRole("button", { name: /Invites, 2 waiting/ })).toBeTruthy();
    // The new vault shows up in the sidebar.
    expect(await screen.findByRole("button", { name: /Family/ })).toBeTruthy();
  });

  it("asks for the owner's code too when a manager invites to somebody else's vault", async () => {
    const user = await open({ sharing: "invites" });
    await user.click(await screen.findByRole("button", { name: /Invites, 3 waiting/ }));
    await user.click(await screen.findByRole("button", { name: `Review: ${KIM.email}` }));
    const dialog = await screen.findByRole("dialog", { name: /Join .*Holiday plans/ });
    expect(await within(dialog).findByText(/isn.t the owner/)).toBeTruthy();
    expect(within(dialog).getByText("Code for the owner, ⁨sam@example.com⁩")).toBeTruthy();
    expect(within(dialog).queryByRole("button", { name: "Join vault" })).toBeNull();
    await user.click(within(dialog).getByRole("button", { name: "It matches" }));
    await user.click(await within(dialog).findByRole("button", { name: "Join vault" }));
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: /Join / })).toBeNull();
    });
  });

  it("says an invite that cannot be opened is unreadable, and lets it be declined", async () => {
    const user = await open({ sharing: "invites" });
    await user.click(await screen.findByRole("button", { name: /Invites, 3 waiting/ }));
    await user.click(await screen.findByRole("button", { name: `Review: ${LEE.email}` }));
    const dialog = await screen.findByRole("dialog");
    expect(await within(dialog).findByText(/This invite can.t be opened/)).toBeTruthy();
    await user.click(within(dialog).getByRole("button", { name: "Decline" }));
    await waitFor(() => {
      expect(screen.queryByRole("button", { name: `Review: ${LEE.email}` })).toBeNull();
    });
  });
});

describe("emergency access", () => {
  it("shows a banner to the grantor and refuses the request", async () => {
    const user = await open({ sharing: "request" });
    const banner = await screen.findByRole("region", { name: "Emergency access request" });
    expect(within(banner).getByText(/asked for emergency access to your vaults/)).toBeTruthy();
    await user.click(within(banner).getByRole("button", { name: "Refuse" }));
    await waitFor(() => {
      expect(screen.queryByRole("region", { name: "Emergency access request" })).toBeNull();
    });
  });

  it("approves early with the master password", async () => {
    const user = await open({ sharing: "request", sync: { variant: "hostedNoTotp" } });
    const banner = await screen.findByRole("region", { name: "Emergency access request" });
    await user.click(within(banner).getByRole("button", { name: "Approve now…" }));
    const dialog = await screen.findByRole("dialog", { name: "Approve emergency access now?" });
    await user.type(within(dialog).getByLabelText("Master password"), "mosaic");
    await user.click(within(dialog).getByRole("button", { name: "Approve" }));
    await waitFor(() => {
      expect(screen.queryByRole("region", { name: "Emergency access request" })).toBeNull();
    });
  });

  it("lists who can ask and who named you, with the wait and what is open", async () => {
    const user = await open({ sharing: "emergency" });
    await user.click(screen.getByRole("button", { name: "Settings" }));
    const heading = await screen.findByRole("heading", { name: "Emergency access" });
    const section = heading.closest("section") as HTMLElement;
    expect(await within(section).findByText("People who can request access")).toBeTruthy();
    expect(within(section).getByText("All your vaults")).toBeTruthy();
    expect(within(section).getByText("Waits 30 days")).toBeTruthy();
    expect(within(section).getByText("People who named you")).toBeTruthy();
    expect(within(section).getByRole("button", { name: /Request access/ })).toBeTruthy();
    expect(within(section).getAllByRole("button", { name: /Open their vaults/ }).length).toBeGreaterThan(0);
  });

  it("explains that a sync folder cannot do emergency access", async () => {
    const user = await open({ sharing: "basic", sync: { variant: "folder" } });
    await user.click(screen.getByRole("button", { name: "Settings" }));
    expect(await screen.findByText(/Emergency access needs the Mosaic server/)).toBeTruthy();
  });
});

describe("sharing findings", () => {
  it("shows each finding with what to do, and acknowledges the ones that point at the server", async () => {
    const user = await open({ sharing: "findings" });
    const group = await screen.findByRole("group", { name: "Sharing warnings" });
    expect(within(group).getByText(/shows a different sharing key for your account/)).toBeTruthy();
    expect(within(group).getByText(/You haven.t verified .*kim@example.com.* in .*Work/)).toBeTruthy();
    expect(within(group).getByText(/list them any more/)).toBeTruthy();
    expect(within(group).getByText(/You no longer have access to/)).toBeTruthy();
    expect(within(group).getByRole("button", { name: "Verify…" })).toBeTruthy();
    await user.click(within(group).getByRole("button", { name: "I've checked…" }));
    await waitFor(() => {
      expect(screen.queryByRole("group", { name: "Sharing warnings" })).toBeNull();
    });
  });

  it("is in German, too", async () => {
    await open({ sharing: "findings" }, "de");
    const group = await screen.findByRole("group", { name: "Warnungen zum Teilen" });
    expect(within(group).getByText(/Du hast keinen Zugriff mehr auf/)).toBeTruthy();
  });
});

describe("settings: sharing", () => {
  it("shows the own safety code and the contacts, and verifies one", async () => {
    const user = await open({ sharing: "basic" });
    await user.click(screen.getByRole("button", { name: "Settings" }));
    const heading = await screen.findByRole("heading", { name: "Sharing" });
    const section = heading.closest("section") as HTMLElement;
    expect(await within(section).findByText("Your safety code")).toBeTruthy();
    expect(within(section).getByText("7KQ4X")).toBeTruthy();
    expect(within(section).getByText("Verified")).toBeTruthy();
    expect(within(section).getByText("Not verified")).toBeTruthy();
    await user.click(within(section).getByRole("button", { name: `Compare code… ${KIM.email}` }));
    const dialog = await screen.findByRole("dialog");
    await user.click(await within(dialog).findByRole("button", { name: "It matches" }));
    await waitFor(() => {
      expect(within(section).queryByText("Not verified")).toBeNull();
    });
  });
});
