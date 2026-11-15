/**
 * The list at a login field in another program (ADR-0164): the logins linked to the program (one
 * click or Enter fills), or, when none is, a search and suggestions to link one. Nothing here
 * ever shows a password; a login is its title, user name and vault.
 */
import { useEffect, useId, useRef, useState, type KeyboardEvent } from "react";
import { Button } from "../components/Button.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import type { IpcError, PickerLogin, PickerState } from "../ipc/contract/index.ts";
import { LinkStepView } from "./LinkStepView.tsx";
import { NoticeView } from "./NoticeView.tsx";
import type { PickerIpc } from "./ipc.ts";

function fold(text: string): string {
  return text.toLocaleLowerCase();
}

function LoginOption({
  login,
  id,
  selected,
  onPick,
}: {
  login: PickerLogin;
  id: string;
  selected: boolean;
  onPick: () => void;
}) {
  return (
    <li
      id={id}
      role="option"
      aria-selected={selected}
      className={selected ? "picker__option picker__option--active" : "picker__option"}
      onClick={onPick}
    >
      <span className="picker__option-title">{login.title}</span>
      {login.username === null ? null : <span className="picker__option-sub">{login.username}</span>}
      {login.vaultName === null ? null : <span className="picker__option-sub">{login.vaultName}</span>}
    </li>
  );
}

/** What the list can ask of Rust; each answer's error goes to `setError`. */
function useListActions(state: PickerState, ipc: PickerIpc, setError: (error: IpcError | null) => void) {
  const run = async (
    command: "picker_dismiss" | "picker_block",
  ): Promise<void> => {
    const result = await ipc.call(command, { generation: state.generation });
    setError(result.ok ? null : result.error);
  };
  const withLogin = async (command: "picker_choose" | "picker_link_begin", login: PickerLogin): Promise<void> => {
    const result = await ipc.call(command, {
      generation: state.generation,
      vaultId: login.vaultId,
      itemId: login.itemId,
    });
    setError(result.ok ? null : result.error);
  };
  return {
    dismiss: () => void run("picker_dismiss"),
    block: () => void run("picker_block"),
    choose: (login: PickerLogin) => void withLogin("picker_choose", login),
    beginLink: (login: PickerLogin) => void withLogin("picker_link_begin", login),
  };
}

type Actions = ReturnType<typeof useListActions>;

function linkAgainKey(reason: PickerState["linkAgain"]) {
  if (reason === "signerChanged") {
    return "programFill.list.signerChanged";
  }
  return reason === "fileChanged" ? "programFill.list.fileChanged" : null;
}

/** No login is linked (or the user chose to link one): search, suggestions, and a click starts the link step. */
function LinkSearch({ state, ipc, actions, setError }: ListParts) {
  const { t } = useI18n();
  const [query, setQuery] = useState("");
  const [found, setFound] = useState<PickerLogin[]>([]);
  const searchNo = useRef(0);
  const program = state.program?.name ?? "";
  const key = linkAgainKey(state.linkAgain);
  const empty = query.trim() === "";

  const search = async (text: string): Promise<void> => {
    setQuery(text);
    searchNo.current += 1;
    const mine = searchNo.current;
    const result = await ipc.call("picker_search", { generation: state.generation, query: text });
    if (mine !== searchNo.current) {
      return;
    }
    if (result.ok) {
      setFound(result.value);
      setError(null);
    } else {
      setError(result.error);
    }
  };

  return (
    <>
      {state.matches.length === 0 ? <p className="picker__text">{t("programFill.list.none", { program })}</p> : null}
      {key === null ? null : <p className="picker__note">{t(key)}</p>}
      <input
        className="picker__search"
        type="text"
        autoComplete="off"
        aria-label={t("programFill.list.search")}
        placeholder={t("programFill.list.search")}
        value={query}
        maxLength={128}
        onChange={(event) => void search(event.target.value)}
      />
      {empty && state.suggestions.length > 0 ? (
        <h2 className="picker__heading">{t("programFill.list.suggestions")}</h2>
      ) : null}
      <ul className="picker__options" aria-label={t("programFill.list.link")}>
        {(empty ? state.suggestions : found).map((login) => (
          <LoginOption
            key={`${login.vaultId}/${login.itemId}`}
            login={login}
            id={`link-${login.itemId}`}
            selected={false}
            onPick={() => {
              actions.beginLink(login);
            }}
          />
        ))}
      </ul>
    </>
  );
}

/** The keys of the list: arrows move, Enter picks, typing filters. */
function listKeys(
  event: KeyboardEvent<HTMLElement>,
  parts: {
    visible: readonly PickerLogin[];
    active: number;
    filterInput: HTMLInputElement | null;
    setActive: (update: (index: number) => number) => void;
    setFilter: (update: (text: string) => string) => void;
    choose: (login: PickerLogin) => void;
  },
): void {
  const last = Math.max(parts.visible.length - 1, 0);
  if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    event.preventDefault();
    const step = event.key === "ArrowDown" ? 1 : -1;
    parts.setActive((index) => Math.min(Math.max(index + step, 0), last));
    return;
  }
  const login = parts.visible[parts.active];
  if (event.key === "Enter" && login !== undefined) {
    event.preventDefault();
    parts.choose(login);
    return;
  }
  const typing = event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey;
  if (typing && event.target !== parts.filterInput) {
    // Typing anywhere in the list filters it.
    event.preventDefault();
    parts.setFilter((text) => text + event.key);
    parts.setActive(() => 0);
    parts.filterInput?.focus();
  }
}

/** The logins linked to the program: a listbox with a filter box. */
function MatchList({ state, actions }: ListParts) {
  const { t } = useI18n();
  const base = useId();
  const [filter, setFilter] = useState("");
  const [active, setActive] = useState(state.preselected ?? 0);
  const filterInput = useRef<HTMLInputElement>(null);
  const listbox = useRef<HTMLUListElement>(null);
  const program = state.program?.name ?? "";

  // The list takes the keys at once: Enter picks the preselected login.
  useEffect(() => {
    listbox.current?.focus();
  }, []);

  const needle = fold(filter.trim());
  const visible = state.matches.filter(
    (login) =>
      needle === "" || fold(login.title).includes(needle) || fold(login.username ?? "").includes(needle),
  );
  const activeIndex = Math.min(active, Math.max(visible.length - 1, 0));
  const optionId = (index: number) => `${base}-option-${String(index)}`;

  return (
    <div
      className="picker__matches"
      onKeyDown={(event) => {
        listKeys(event, {
          visible,
          active: activeIndex,
          filterInput: filterInput.current,
          setActive,
          setFilter,
          choose: actions.choose,
        });
      }}
    >
      <ul
        ref={listbox}
        className="picker__options"
        role="listbox"
        tabIndex={0}
        aria-label={t("programFill.list.title", { program })}
        aria-activedescendant={visible.length === 0 ? undefined : optionId(activeIndex)}
      >
        {visible.map((login, index) => (
          <LoginOption
            key={`${login.vaultId}/${login.itemId}`}
            login={login}
            id={optionId(index)}
            selected={index === activeIndex}
            onPick={() => {
              actions.choose(login);
            }}
          />
        ))}
      </ul>
      <input
        ref={filterInput}
        className="picker__search"
        type="text"
        autoComplete="off"
        aria-label={t("programFill.list.search")}
        placeholder={t("programFill.list.search")}
        value={filter}
        maxLength={128}
        onChange={(event) => {
          setFilter(event.target.value);
          setActive(0);
        }}
      />
    </div>
  );
}

interface ListParts {
  state: PickerState;
  ipc: PickerIpc;
  actions: Actions;
  setError: (error: IpcError | null) => void;
}

function LoginList({ state, ipc }: { state: PickerState; ipc: PickerIpc }) {
  const { t, errorMessage } = useI18n();
  const [linking, setLinking] = useState(state.matches.length === 0);
  const [error, setError] = useState<IpcError | null>(null);
  const actions = useListActions(state, ipc, setError);
  const program = state.program?.name ?? "";
  const parts = { state, ipc, actions, setError };

  return (
    <section
      className="picker picker--list"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          actions.dismiss();
        }
      }}
    >
      <h1 className="picker__title">{t("programFill.list.title", { program })}</h1>
      {linking ? <LinkSearch {...parts} /> : <MatchList {...parts} />}
      <ErrorAlert error={error} message={error === null ? undefined : errorMessage(error)} />
      <div className="picker__actions">
        {linking ? null : (
          <Button
            size="sm"
            variant="ghost"
            onClick={() => {
              setLinking(true);
            }}
          >
            {t("programFill.list.link")}
          </Button>
        )}
        <Button size="sm" variant="ghost" onClick={actions.block}>
          {t("programFill.list.block")}
        </Button>
      </div>
    </section>
  );
}

/** What the list window shows for its state: a notice, the link step, or the logins. */
export function PickerList({ state, ipc }: { state: PickerState; ipc: PickerIpc }) {
  if (state.mode === "notice") {
    return <NoticeView state={state} ipc={ipc} />;
  }
  if (state.link !== null) {
    return <LinkStepView state={state} step={state.link} ipc={ipc} />;
  }
  return <LoginList key={state.generation} state={state} ipc={ipc} />;
}
