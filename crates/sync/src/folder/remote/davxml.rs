//! The `WebDAV` multistatus answer to a `PROPFIND` (RFC 4918), read strictly:
//! namespaces are resolved, a document type declaration is refused, the depth
//! and the number of entries are bounded, and every `href` has to name one
//! valid component directly below the collection that was asked about. What
//! does not is dropped, so a server cannot put a name outside the folder into
//! a listing.

use std::ffi::OsString;
use std::time::SystemTime;

use quick_xml::events::Event;
use quick_xml::name::{Namespace, ResolveResult};
use quick_xml::reader::NsReader;

use super::oauth::percent_decode;
use super::time::parse_http_date;
use crate::folder::store::{Entry, EntryKind, valid_component};
use crate::transport::TransportError;

/// The deepest element nesting that is read.
const MAX_DEPTH: usize = 16;
/// The most `response` elements of one answer.
const MAX_RESPONSES: usize = 4096;
/// The longest text of one element that is kept.
const MAX_TEXT: usize = 8 * 1024;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Elem {
    Response,
    Href,
    Propstat,
    Status,
    Etag,
    Length,
    Modified,
    ResourceType,
    Collection,
    Other,
}

fn elem(local: &str, dav: bool) -> Elem {
    if !dav {
        return Elem::Other;
    }
    match local {
        "response" => Elem::Response,
        "href" => Elem::Href,
        "propstat" => Elem::Propstat,
        "status" => Elem::Status,
        "getetag" => Elem::Etag,
        "getcontentlength" => Elem::Length,
        "getlastmodified" => Elem::Modified,
        "resourcetype" => Elem::ResourceType,
        "collection" => Elem::Collection,
        _ => Elem::Other,
    }
}

/// What one `propstat` says.
#[derive(Default)]
struct Props {
    ok: bool,
    is_dir: bool,
    len: Option<u64>,
    modified: Option<SystemTime>,
    etag: Option<String>,
}

/// One `response` element.
#[derive(Default)]
struct Raw {
    /// The decoded path of the `href`; empty if it could not be decoded.
    path: String,
    is_dir: bool,
    len: Option<u64>,
    modified: Option<SystemTime>,
    etag: Option<String>,
}

/// The path of an `href`: the scheme and authority of an absolute one are
/// dropped, the rest is percent-decoded.
fn href_path(href: &str) -> String {
    let path = href
        .strip_prefix("https://")
        .or_else(|| href.strip_prefix("http://"))
        .map_or(href, |rest| {
            rest.find('/').map_or("", |i| rest.get(i..).unwrap_or(""))
        });
    let path = path.split(['?', '#']).next().unwrap_or_default();
    percent_decode(path, false).unwrap_or_default()
}

#[derive(Default)]
struct Parser {
    out: Vec<Raw>,
    resp: Option<Raw>,
    props: Option<Props>,
    capture: Option<Elem>,
    text: String,
    in_resource_type: bool,
}

impl Parser {
    fn start(&mut self, e: Elem) {
        match e {
            Elem::Response => self.resp = Some(Raw::default()),
            Elem::Propstat => self.props = Some(Props::default()),
            Elem::ResourceType => self.in_resource_type = true,
            Elem::Collection if self.in_resource_type => {
                if let Some(p) = self.props.as_mut() {
                    p.is_dir = true;
                }
            }
            Elem::Href | Elem::Status | Elem::Etag | Elem::Length | Elem::Modified => {
                self.capture = Some(e);
                self.text.clear();
            }
            _ => {}
        }
    }

    fn end(&mut self, e: Elem) -> Result<(), TransportError> {
        match e {
            Elem::Response => {
                if let Some(r) = self.resp.take()
                    && !r.path.is_empty()
                {
                    if self.out.len() >= MAX_RESPONSES {
                        return Err(TransportError::Protocol);
                    }
                    self.out.push(r);
                }
            }
            Elem::Propstat => {
                if let (Some(p), Some(r)) = (self.props.take(), self.resp.as_mut())
                    && p.ok
                {
                    r.is_dir |= p.is_dir;
                    r.len = r.len.or(p.len);
                    r.modified = r.modified.or(p.modified);
                    r.etag = r.etag.take().or(p.etag);
                }
            }
            Elem::ResourceType => self.in_resource_type = false,
            Elem::Href | Elem::Status | Elem::Etag | Elem::Length | Elem::Modified
                if self.capture == Some(e) =>
            {
                self.capture = None;
                let text = self.text.trim();
                match e {
                    Elem::Href => {
                        if let Some(r) = self.resp.as_mut() {
                            r.path = href_path(text);
                        }
                    }
                    Elem::Status => {
                        if let Some(p) = self.props.as_mut() {
                            p.ok = text
                                .split_whitespace()
                                .nth(1)
                                .is_some_and(|code| code.starts_with('2') && code.len() == 3);
                        }
                    }
                    _ => {
                        if let Some(p) = self.props.as_mut() {
                            match e {
                                Elem::Etag => p.etag = Some(text.to_owned()),
                                Elem::Length => p.len = text.parse().ok(),
                                _ => p.modified = parse_http_date(text),
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn push_text(&mut self, text: &str) -> Result<(), TransportError> {
        if self.capture.is_some() {
            if self.text.len() + text.len() > MAX_TEXT {
                return Err(TransportError::Protocol);
            }
            self.text.push_str(text);
        }
        Ok(())
    }
}

/// The text of a predefined or numeric entity reference.
fn entity(name: &str, numeric: Option<char>) -> Option<char> {
    numeric.or(match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => None,
    })
}

fn parse_responses(body: &[u8]) -> Result<Vec<Raw>, TransportError> {
    let mut reader = NsReader::from_reader(body);
    let mut parser = Parser::default();
    let mut buf = Vec::new();
    let mut depth = 0usize;
    loop {
        let (ns, event) = reader
            .read_resolved_event_into(&mut buf)
            .map_err(|_| TransportError::Protocol)?;
        let dav = matches!(ns, ResolveResult::Bound(Namespace("DAV:")));
        match event {
            Event::Start(e) => {
                depth += 1;
                if depth > MAX_DEPTH {
                    return Err(TransportError::Protocol);
                }
                parser.start(elem(e.local_name().as_ref(), dav));
            }
            Event::Empty(e) => {
                let kind = elem(e.local_name().as_ref(), dav);
                parser.start(kind);
                parser.end(kind)?;
            }
            Event::End(e) => {
                depth = depth.saturating_sub(1);
                parser.end(elem(e.local_name().as_ref(), dav))?;
            }
            Event::Text(t) => parser.push_text(&t.xml10_content())?,
            Event::CData(c) => {
                parser.push_text(&c.into_inner())?;
            }
            Event::GeneralRef(r) => {
                let numeric = r.resolve_char_ref().map_err(|_| TransportError::Protocol)?;
                let c = entity(&r, numeric).ok_or(TransportError::Protocol)?;
                parser.push_text(c.encode_utf8(&mut [0u8; 4]))?;
            }
            Event::DocType(_) => return Err(TransportError::Protocol),
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(parser.out)
}

fn entry_of(raw: &Raw, name: &str) -> Entry {
    Entry {
        name: OsString::from(name),
        kind: if raw.is_dir {
            EntryKind::Dir
        } else {
            EntryKind::File
        },
        len: if raw.is_dir { 0 } else { raw.len.unwrap_or(0) },
        modified: raw.modified,
        version: raw.etag.as_deref().map(|e| e.as_bytes().into()),
    }
}

/// The entries of a `Depth: 1` answer about `collection_href` (a decoded path,
/// with or without a trailing slash). The collection itself is dropped, and so
/// is every entry whose `href` is not `collection_href` plus exactly one valid
/// component.
///
/// # Errors
/// [`TransportError::Protocol`] for malformed XML, a document type
/// declaration, nesting over 16 levels, more than 4096 responses or an
/// entity that is not predefined.
pub fn parse_multistatus(body: &[u8], collection_href: &str) -> Result<Vec<Entry>, TransportError> {
    let prefix = format!("{}/", collection_href.trim_end_matches('/'));
    let mut entries = Vec::new();
    for raw in parse_responses(body)? {
        let Some(rest) = raw.path.strip_prefix(&prefix) else {
            continue;
        };
        let name = rest.strip_suffix('/').unwrap_or(rest);
        if valid_component(name) {
            entries.push(entry_of(&raw, name));
        }
    }
    Ok(entries)
}

/// The entry of a `Depth: 0` answer about `target_href` (a decoded path).
///
/// # Errors
/// [`TransportError::Protocol`] as for [`parse_multistatus`], and when the
/// answer is about another path.
pub fn parse_stat(body: &[u8], target_href: &str) -> Result<Entry, TransportError> {
    let target = target_href.trim_end_matches('/');
    let name = target.rsplit('/').next().unwrap_or_default();
    parse_responses(body)?
        .iter()
        .find(|r| r.path.trim_end_matches('/') == target)
        .map(|raw| entry_of(raw, name))
        .ok_or(TransportError::Protocol)
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLLECTION: &str = "/remote.php/dav/files/alice/Mosaic";

    fn answer(responses: &str) -> Vec<u8> {
        format!(
            "<?xml version=\"1.0\"?><d:multistatus xmlns:d=\"DAV:\" xmlns:oc=\"http://owncloud.org/ns\">{responses}</d:multistatus>"
        )
        .into_bytes()
    }

    fn file(href: &str, len: u64) -> String {
        format!(
            "<d:response><d:href>{href}</d:href><d:propstat><d:prop>\
             <d:getlastmodified>Sun, 15 Nov 2026 10:20:30 GMT</d:getlastmodified>\
             <d:getcontentlength>{len}</d:getcontentlength><d:resourcetype/>\
             <d:getetag>&quot;abc{len}&quot;</d:getetag></d:prop>\
             <d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>"
        )
    }

    fn dir(href: &str) -> String {
        format!(
            "<d:response><d:href>{href}</d:href><d:propstat><d:prop>\
             <d:resourcetype><d:collection/></d:resourcetype>\
             <d:getetag>\"d1\"</d:getetag></d:prop>\
             <d:status>HTTP/1.1 200 OK</d:status></d:propstat>\
             <d:propstat><d:prop><d:getcontentlength/></d:prop>\
             <d:status>HTTP/1.1 404 Not Found</d:status></d:propstat></d:response>"
        )
    }

    #[test]
    fn parses_files_dirs_and_drops_the_collection() {
        let body = answer(&format!(
            "{}{}{}{}",
            dir(&format!("{COLLECTION}/")),
            file(&format!("{COLLECTION}/a%20b.mseg"), 12),
            dir(&format!("{COLLECTION}/sub/")),
            file(&format!("https://cloud.example.com{COLLECTION}/c.mck"), 3),
        ));
        let list = parse_multistatus(&body, COLLECTION).unwrap();
        let names: Vec<_> = list.iter().map(|e| e.name.to_str().unwrap()).collect();
        assert_eq!(names, ["a b.mseg", "sub", "c.mck"]);
        assert_eq!(list[0].kind, EntryKind::File);
        assert_eq!(list[0].len, 12);
        assert_eq!(list[0].modified_ms(), 1_794_738_030_000);
        assert_eq!(list[0].version.as_deref(), Some(&b"\"abc12\""[..]));
        assert_eq!(list[1].kind, EntryKind::Dir);
        assert_eq!(list[1].len, 0);
        assert_eq!(list[2].len, 3);
    }

    #[test]
    fn entries_outside_the_collection_are_dropped() {
        let body = answer(&format!(
            "{}{}{}{}{}{}{}",
            file(&format!("{COLLECTION}/ok.mseg"), 1),
            file("/remote.php/dav/files/alice/Other/x.mseg", 1),
            file(&format!("{COLLECTION}/a%2Fb.mseg"), 1),
            file(&format!("{COLLECTION}/deep/er.mseg"), 1),
            file(&format!("{COLLECTION}/%2E%2E"), 1),
            file(&format!("{COLLECTION}/sub%3Aname"), 1),
            file(&format!("{COLLECTION}x/y.mseg"), 1),
        ));
        let list = parse_multistatus(&body, &format!("{COLLECTION}/")).unwrap();
        let names: Vec<_> = list.iter().map(|e| e.name.to_str().unwrap()).collect();
        assert_eq!(names, ["ok.mseg"]);
    }

    #[test]
    fn stat_reads_the_one_entry() {
        let body = answer(&file(&format!("{COLLECTION}/f.mseg"), 9));
        let e = parse_stat(&body, &format!("{COLLECTION}/f.mseg")).unwrap();
        assert_eq!((e.name.to_str(), e.len), (Some("f.mseg"), 9));
        let d = parse_stat(&answer(&dir(&format!("{COLLECTION}/"))), COLLECTION).unwrap();
        assert!(d.is_dir());
        assert_eq!(
            parse_stat(&body, "/elsewhere/f.mseg").err(),
            Some(TransportError::Protocol)
        );
    }

    #[test]
    fn entities_resolve_and_hostile_xml_is_refused() {
        let body = answer(&file(&format!("{COLLECTION}/a&amp;b&#x63;"), 1));
        let list = parse_multistatus(&body, COLLECTION).unwrap();
        assert_eq!(list[0].name.to_str(), Some("a&bc"));
        let proto = Some(TransportError::Protocol);
        // An undefined entity, a document type declaration, a broken document.
        let unknown = answer(&file(&format!("{COLLECTION}/a&xxe;b"), 1));
        assert_eq!(parse_multistatus(&unknown, COLLECTION).err(), proto);
        let doctype = b"<?xml version=\"1.0\"?><!DOCTYPE x [<!ENTITY a \"b\">]><d:multistatus xmlns:d=\"DAV:\"/>";
        assert_eq!(parse_multistatus(doctype, COLLECTION).err(), proto);
        assert_eq!(
            parse_multistatus(b"<d:multistatus", COLLECTION).err(),
            proto
        );
        // Nesting over 16 levels.
        let deep = format!("{}{}", "<a>".repeat(17), "</a>".repeat(17));
        assert_eq!(parse_multistatus(deep.as_bytes(), COLLECTION).err(), proto);
        // More than 4096 responses.
        let many = answer(
            &(0..=MAX_RESPONSES)
                .map(|i| file(&format!("{COLLECTION}/{i}.mseg"), 1))
                .collect::<String>(),
        );
        assert_eq!(parse_multistatus(&many, COLLECTION).err(), proto);
        // An element in another namespace is not a DAV element.
        let foreign = b"<x:multistatus xmlns:x=\"urn:x\" xmlns:d=\"urn:other\"><d:response><d:href>/remote.php/dav/files/alice/Mosaic/a</d:href></d:response></x:multistatus>";
        assert!(parse_multistatus(foreign, COLLECTION).unwrap().is_empty());
    }
}
