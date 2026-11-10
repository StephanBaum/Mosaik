//! Reading commands: `list`, `search`, `get`, `totp`, `ssh`.

use std::collections::BTreeMap;
use std::io::IsTerminal as _;

use mosaic_core::ItemType;
use mosaic_core::model::OtpKind;
use mosaic_vault::session::TrashFilter;
use serde::Serialize;

use crate::app::App;
use crate::clipboard;
use crate::error::{CliError, EXIT_REVEAL, Result};
use crate::i18n::Msg;
use crate::model::{self, Entry, ItemOut, Value, VaultOut, item_filter, vault_names};
use crate::output::{Output, escape};
use crate::refs;
use crate::spec::Parsed;

fn first_arg(p: &Parsed) -> Result<&str> {
    p.args
        .first()
        .map(String::as_str)
        .ok_or_else(|| CliError::usage(Msg::ErrMissingArgument).with("name", "REF"))
}

fn print_rows(out: &Output, rows: &[ItemOut]) {
    if rows.is_empty() {
        out.say(&out.t(Msg::OutNoItems, &[]));
        return;
    }
    if out.quiet {
        for r in rows {
            out.answer(&r.id);
        }
        return;
    }
    let cell = |s: &str| escape(s);
    let width = |f: &dyn Fn(&ItemOut) -> String| {
        rows.iter().map(|r| f(r).chars().count()).max().unwrap_or(0)
    };
    let w_vault = width(&|r| cell(&r.vault.name));
    let w_type = width(&|r| r.kind.clone());
    let w_title = width(&|r| cell(&r.title));
    for r in rows {
        let line = format!(
            "{}  {:<w_vault$}  {:<w_type$}  {:<w_title$}  {}",
            r.id,
            cell(&r.vault.name),
            r.kind,
            cell(&r.title),
            cell(r.username.as_deref().unwrap_or("")),
        );
        out.say(line.trim_end());
    }
}

fn sorted(mut rows: Vec<ItemOut>) -> Vec<ItemOut> {
    rows.sort_by(|a, b| {
        (a.vault.name.to_lowercase(), a.title.to_lowercase(), &a.id).cmp(&(
            b.vault.name.to_lowercase(),
            b.title.to_lowercase(),
            &b.id,
        ))
    });
    rows
}

/// `mosaic list`.
pub fn list(app: &mut App, p: &Parsed) -> Result<()> {
    let u = app.unlock()?;
    let s = &u.session;
    let vault = p.value("vault").map(|v| refs::vault(s, v)).transpose()?;
    let trash = if p.has("trash") {
        TrashFilter::Only
    } else {
        TrashFilter::Exclude
    };
    let kind = p.value("type").map(ItemType::parse);
    let names = vault_names(s);
    let rows = s
        .items(&item_filter(vault, trash, p.has("favorites")))
        .iter()
        .filter(|r| {
            kind.as_ref()
                .is_none_or(|k| r.item.item_type().as_ref() == Some(k))
        })
        .map(|r| model::row(&names, r))
        .collect();
    let rows = sorted(rows);
    app.out.data(&rows, |o| print_rows(o, &rows));
    Ok(())
}

/// `mosaic search`.
pub fn search(app: &mut App, p: &Parsed) -> Result<()> {
    let u = app.unlock()?;
    let s = &u.session;
    let vault = p.value("vault").map(|v| refs::vault(s, v)).transpose()?;
    let names = vault_names(s);
    let rows = s
        .search(first_arg(p)?)
        .iter()
        .filter(|r| vault.is_none_or(|v| r.vault_id == v))
        .map(|r| model::row(&names, r))
        .collect();
    let rows = sorted(rows);
    app.out.data(&rows, |o| print_rows(o, &rows));
    Ok(())
}

#[derive(Serialize)]
#[serde(untagged)]
enum FieldOut<'a> {
    Value { value: &'a str },
    Concealed { concealed: bool },
}

#[derive(Serialize)]
struct OtpOut<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    concealed: Option<bool>,
    #[serde(rename = "secondsRemaining")]
    seconds_remaining: Option<u32>,
}

#[derive(Serialize)]
struct AttachmentOut {
    name: String,
    size: u64,
    mime: String,
}

#[derive(Serialize)]
struct GetOut<'a> {
    #[serde(flatten)]
    item: ItemOut,
    fields: BTreeMap<String, FieldOut<'a>>,
    otp: Option<OtpOut<'a>>,
    attachments: Vec<AttachmentOut>,
}

fn reveal_required(field: &str) -> CliError {
    CliError::new("revealRequired", EXIT_REVEAL, Msg::ErrRevealRequired).with("field", field)
}

/// `mosaic get`.
#[expect(
    clippy::too_many_lines,
    reason = "one pass over the item: selection, gating, copy, JSON and text"
)]
pub fn get(app: &mut App, p: &Parsed) -> Result<()> {
    let mut u = app.unlock()?;
    let target = refs::resolve(&u.session, first_arg(p)?)?;
    let mut wanted: Vec<String> = p.values("field").into_iter().map(str::to_owned).collect();
    wanted.extend(target.field.clone());
    wanted.dedup();
    let (reveal, copy) = (p.has("reveal"), p.has("copy"));
    let wants_otp = wanted.is_empty() || wanted.iter().any(|w| w == "otp");
    let has_totp = u
        .session
        .item(target.vault, target.item)?
        .otp()
        .is_some_and(|c| c.kind == OtpKind::Totp);
    if wanted.iter().any(|w| w == "otp") && !has_totp {
        return Err(refs::no_such_field("otp"));
    }
    let otp_code = if wants_otp && has_totp {
        Some(
            u.session
                .otp_code(target.vault, target.item, app.now / 1000)?,
        )
    } else {
        None
    };
    let item = u.session.item(target.vault, target.item)?;
    let names = vault_names(&u.session);
    let head = ItemOut {
        id: target.item.to_string(),
        vault: VaultOut {
            id: target.vault.to_string(),
            name: names.get(&target.vault).cloned().unwrap_or_default(),
        },
        kind: item
            .item_type()
            .map_or_else(String::new, |t| t.as_str().to_owned()),
        title: item.title().unwrap_or_default().to_owned(),
        username: item
            .text(mosaic_core::model::TextField::Username)
            .map(str::to_owned),
        urls: item
            .urls()
            .iter()
            .map(|(_, x)| x.url.as_str().to_owned())
            .collect(),
        favorite: item.favorite(),
        trashed: item.trashed_at().is_some(),
    };
    let requested: Vec<&str> = wanted
        .iter()
        .map(String::as_str)
        .filter(|w| *w != "otp")
        .collect();
    let selected: Vec<Entry> = if requested.is_empty() && !wanted.iter().any(|w| w == "otp") {
        model::entries(item)
    } else {
        requested
            .iter()
            .map(|w| model::lookup(item, w).ok_or_else(|| refs::no_such_field(w)))
            .collect::<Result<_>>()?
    };
    let asked_secret = selected
        .iter()
        .find(|e| matches!(e.value, Value::Secret(_)));
    let secret_asked =
        !wanted.is_empty() && (asked_secret.is_some() || wanted.iter().any(|w| w == "otp"));
    if secret_asked && !reveal && !copy {
        let name = asked_secret.map_or("otp", |e| e.key.as_str());
        return Err(reveal_required(name));
    }
    if copy {
        let one = match (selected.as_slice(), otp_code.as_ref(), wanted.len()) {
            ([e], None, 1) => match &e.value {
                Value::Text(t) => t.as_str(),
                Value::Secret(s) => s.expose_secret(),
            },
            ([], Some(code), 1) => code.code.expose_secret(),
            _ => return Err(CliError::usage(Msg::ErrMissingArgument).with("name", "--field")),
        };
        clipboard::copy(&app.out, one, clipboard::clear_after(p)?)?;
    }
    if reveal && !app.out.json && std::io::stdout().is_terminal() {
        app.out
            .warn("revealToTerminal", Msg::WarnRevealToTerminal, &[]);
    }
    let fields: BTreeMap<String, FieldOut<'_>> = selected
        .iter()
        .map(|e| {
            let out = match &e.value {
                Value::Text(t) => FieldOut::Value { value: t },
                Value::Secret(s) if reveal => FieldOut::Value {
                    value: s.expose_secret(),
                },
                Value::Secret(_) => FieldOut::Concealed { concealed: true },
            };
            (e.key.clone(), out)
        })
        .collect();
    let otp = otp_code.as_ref().map(|c| OtpOut {
        code: reveal.then(|| c.code.expose_secret()),
        concealed: (!reveal).then_some(true),
        seconds_remaining: c.seconds_remaining,
    });
    let attachments = item
        .attachments()
        .iter()
        .map(|a| AttachmentOut {
            name: a.file_name.as_str().to_owned(),
            size: a.size,
            mime: a.mime.as_str().to_owned(),
        })
        .collect();
    let doc = GetOut {
        item: head,
        fields,
        otp,
        attachments,
    };
    if copy {
        // The value went to the clipboard; nothing else is printed.
        if app.out.json {
            app.out.data(&doc, |_| {});
        }
        return Ok(());
    }
    app.out.data(&doc, |o| {
        render_get(
            o,
            &doc,
            reveal,
            &selected,
            otp_code.as_ref(),
            !wanted.is_empty(),
        );
    });
    Ok(())
}

fn render_get(
    out: &Output,
    doc: &GetOut<'_>,
    reveal: bool,
    selected: &[Entry],
    otp: Option<&mosaic_vault::session::OtpCode>,
    only_requested: bool,
) {
    let concealed = out.t(Msg::OutConcealed, &[]);
    // One requested field: its value alone, nothing else (scripts read it).
    if only_requested && selected.len() + usize::from(otp.is_some()) == 1 {
        if let Some(entry) = selected.first() {
            match &entry.value {
                Value::Text(t) => out.answer(t),
                Value::Secret(s) => out.write_secret(s.expose_secret(), &entry.key),
            }
        } else if let Some(code) = otp {
            out.write_secret(code.code.expose_secret(), "otp");
        }
        return;
    }
    if only_requested {
        // Several requested fields: key: value lines.
    } else {
        let item = &doc.item;
        out.say(&format!("{}: {}", out.t(Msg::LabelId, &[]), item.id));
        out.say(&format!(
            "{}: {}",
            out.t(Msg::LabelVault, &[]),
            escape(&item.vault.name)
        ));
        out.say(&format!("{}: {}", out.t(Msg::LabelType, &[]), item.kind));
        out.say(&format!(
            "{}: {}",
            out.t(Msg::LabelTitle, &[]),
            escape(&item.title)
        ));
        for url in &item.urls {
            out.say(&format!("url: {}", escape(url)));
        }
    }
    for entry in selected {
        match &entry.value {
            Value::Text(t) => out.say(&format!("{}: {}", entry.key, escape(t))),
            Value::Secret(s) if reveal => {
                out.write_raw(format!("{}: ", entry.key).as_bytes());
                out.write_secret(s.expose_secret(), &entry.key);
            }
            Value::Secret(_) => out.say(&format!("{}: {concealed}", entry.key)),
        }
    }
    if let Some(code) = otp {
        let label = out.t(Msg::LabelOtp, &[]);
        if reveal {
            out.write_raw(format!("{label}: ").as_bytes());
            out.write_secret(code.code.expose_secret(), "otp");
        } else {
            out.say(&format!("{label}: {concealed}"));
        }
    }
    if !only_requested {
        for a in &doc.attachments {
            out.say(&format!(
                "{}: {}",
                out.t(Msg::LabelAttachments, &[]),
                out.t(
                    Msg::OutAttachment,
                    &[
                        ("name", escape(&a.name)),
                        ("size", a.size.to_string()),
                        ("mime", escape(&a.mime)),
                    ],
                )
            ));
        }
    }
}

#[derive(Serialize)]
struct TotpOut<'a> {
    code: &'a str,
    #[serde(rename = "secondsRemaining")]
    seconds_remaining: Option<u32>,
}

/// `mosaic totp`.
pub fn totp(app: &mut App, p: &Parsed) -> Result<()> {
    let mut u = app.unlock()?;
    let target = refs::resolve(&u.session, first_arg(p)?)?;
    let code = u
        .session
        .otp_code(target.vault, target.item, app.now / 1000)?;
    let text = code.code.expose_secret();
    if p.has("copy") {
        clipboard::copy(&app.out, text, clipboard::clear_after(p)?)?;
        return Ok(());
    }
    if app.out.json {
        app.out.data(
            &TotpOut {
                code: text,
                seconds_remaining: code.seconds_remaining,
            },
            |_| {},
        );
    } else {
        app.out.write_secret(text, "code");
    }
    Ok(())
}

#[derive(Serialize)]
struct SshOut {
    id: String,
    vault: VaultOut,
    title: String,
    fingerprint: String,
    #[serde(rename = "publicKey")]
    public_key: String,
}

/// `mosaic ssh`.
pub fn ssh(app: &mut App, p: &Parsed) -> Result<()> {
    let action = p.args.first().map_or("", String::as_str);
    if action == "agent-status" {
        app.out.data(&serde_json::json!({ "agent": false }), |o| {
            o.answer(&o.t(Msg::SshAgentStatus, &[]));
        });
        return Ok(());
    }
    let u = app.unlock()?;
    let names = vault_names(&u.session);
    let ssh_of = |vault, item: &mosaic_core::Item, id: mosaic_core::ItemId| {
        item.ssh_key().map(|k| SshOut {
            id: id.to_string(),
            vault: VaultOut {
                id: mosaic_core::VaultId::to_string(&vault),
                name: names.get(&vault).cloned().unwrap_or_default(),
            },
            title: item.title().unwrap_or_default().to_owned(),
            fingerprint: k.fingerprint.as_str().to_owned(),
            public_key: k.public_key.as_str().to_owned(),
        })
    };
    if action == "list" {
        let mut keys: Vec<SshOut> = u
            .session
            .items(&item_filter(None, TrashFilter::Exclude, false))
            .iter()
            .filter(|r| r.item.item_type() == Some(ItemType::SshKey))
            .filter_map(|r| ssh_of(r.vault_id, r.item, r.item_id))
            .collect();
        keys.sort_by_key(|k| k.title.to_lowercase());
        app.out.data(&keys, |o| {
            if keys.is_empty() {
                o.say(&o.t(Msg::SshNoKeys, &[]));
            }
            for k in &keys {
                o.say(&format!(
                    "{}  {}  {}",
                    k.id,
                    k.fingerprint,
                    escape(&k.title)
                ));
            }
        });
        return Ok(());
    }
    // public-key REF
    let reference = p
        .args
        .get(1)
        .ok_or_else(|| CliError::usage(Msg::ErrMissingArgument).with("name", "REF"))?;
    let target = refs::resolve(&u.session, reference)?;
    let item = u.session.item(target.vault, target.item)?;
    let key = ssh_of(target.vault, item, target.item).ok_or_else(|| refs::no_such_field("ssh"))?;
    app.out.data(&key, |o| o.answer(&key.public_key));
    Ok(())
}
