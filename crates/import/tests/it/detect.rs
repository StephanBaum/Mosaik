//! Content-based detection of the export format.

#![expect(clippy::unwrap_used, clippy::indexing_slicing, reason = "tests")]

use mosaic_import::{Detected, Error, Format, Source, detect, parse};

use crate::support::{fixture, zip_of};

fn is(bytes: &[u8], format: Format, source: Source) {
    assert_eq!(
        detect(bytes),
        Some(Detected { format, source }),
        "{}",
        String::from_utf8_lossy(&bytes[..bytes.len().min(40)])
    );
}

#[test]
fn detects_every_format_from_content() {
    is(&fixture("aes-argon2d.kdbx"), Format::Keepass, Source::KeePass);
    is(
        &zip_of(&[("export.data", &fixture("1pux-export.data.json"))]),
        Format::OnePassword,
        Source::OnePassword,
    );
    is(
        &zip_of(&[("Proton Pass/data.json", b"{}")]),
        Format::ProtonPass,
        Source::ProtonPass,
    );
    is(
        &zip_of(&[("Proton Pass/data.pgp", b"x")]),
        Format::ProtonPass,
        Source::ProtonPass,
    );
    is(
        &zip_of(&[("Dashlane/Credentials.csv", b"username,title\n")]),
        Format::Dashlane,
        Source::Dashlane,
    );
    is(
        b"-----BEGIN PGP MESSAGE-----\n...",
        Format::ProtonPass,
        Source::ProtonPass,
    );
    is(&fixture("bitwarden.json"), Format::Bitwarden, Source::Bitwarden);
    is(
        br#"{"passwordProtected":true,"items":[]}"#,
        Format::Bitwarden,
        Source::Bitwarden,
    );
    is(br#"{"records":[]}"#, Format::Keeper, Source::Keeper);
    is(br#"{"vaults":{}}"#, Format::ProtonPass, Source::ProtonPass);
    is(
        br#"{"version":{"major":1,"minor":0},"accounts":[]}"#,
        Format::Cxf,
        Source::Cxf,
    );
    is(&fixture("enpass.json"), Format::Enpass, Source::Enpass);
    // A byte order mark and leading white space do not hide a JSON file.
    is(b"\xef\xbb\xbf \n{\"records\":[]}", Format::Keeper, Source::Keeper);

    let csv = [
        ("name,url,username,password,note", Source::Chromium),
        (
            "url,username,password,httpRealm,formActionOrigin,guid,timeCreated",
            Source::Firefox,
        ),
        ("Title,URL,Username,Password,Notes,OTPAuth", Source::Safari),
        (
            "folder,favorite,type,name,notes,fields,reprompt,login_uri,login_username,login_password,login_totp",
            Source::BitwardenCsv,
        ),
        ("url,username,password,totp,extra,name,grouping,fav", Source::LastPass),
        (
            "name,url,additional_urls,username,password,note,cardholdername,cardnumber,cvc,pin,expirydate,zipcode,folder,full_name,phone_number,email,address1,address2,city,country,state,type,custom_fields",
            Source::NordPass,
        ),
        ("Name,Url,MatchUrl,Login,Pwd,Note,Folder,RfFieldsV2", Source::RoboForm),
        (
            "Group,Title,Username,Password,URL,Notes,TOTP,Icon,Last Modified,Created",
            Source::KeePassXcCsv,
        ),
        (
            "username,username2,username3,title,password,note,url,category,otpSecret",
            Source::Dashlane,
        ),
        ("Titel,Benutzername,Passwort,Webseite", Source::Csv),
    ];
    for (header, source) in csv {
        is(format!("{header}\nx,y\n").as_bytes(), Format::Csv, source);
    }
    // Keeper's CSV has no header row: it looks like any other CSV.
    is(
        b"Personal,GitHub,alice,pw1,https://github.com,n,,TFC:Keeper,otpauth://totp/x",
        Format::Csv,
        Source::Csv,
    );
}

#[test]
fn unknown_files_are_not_detected() {
    let nul: Vec<u8> = (0..64u8).map(|i| i.wrapping_mul(7)).collect();
    for bytes in [
        &b""[..],
        b"hello\nworld",
        &nul,
        &zip_of(&[("readme.txt", b"hi")]),
        br#"{"x":1}"#,
        b"[1]",
    ] {
        assert_eq!(detect(bytes), None);
    }
}

#[test]
fn utf16_and_latin1_are_not_detected() {
    assert_eq!(detect(b"\xff\xfeN\0a\0m\0e\0"), None);
    let latin1 = b"name,url\nM\xfcller,x";
    assert_eq!(detect(latin1), None);
    assert_eq!(
        parse(Format::Csv, latin1, None).unwrap_err(),
        Error::Malformed
    );
}
