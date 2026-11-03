import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { UserText } from "./UserText.tsx";

describe("UserText", () => {
  it("wraps typed text in its own bidirectional isolate", () => {
    const { container } = render(
      <p>
        Key: <UserText>{"abc‮def"}</UserText> (ssh)
      </p>,
    );
    const isolate = container.querySelector("bdi.user-text");
    expect(isolate).not.toBeNull();
    expect(isolate?.textContent).toBe("abc‮def");
    // The surrounding text is not inside the isolate.
    expect(container.querySelector("p")?.textContent).toBe("Key: abc‮def (ssh)");
  });
});
