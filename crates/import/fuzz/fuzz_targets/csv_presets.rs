//! The CSV presets of LastPass, NordPass, RoboForm, KeePassXC and Dashlane:
//! arbitrary rows under each preset's header never panic, and whatever is
//! accepted makes valid items (`common::check`). The first byte picks the
//! header; the rest is the file body. Seeds: one row per preset from
//! `crates/import/tests/it/{lastpass,nordpass,csv_presets,dashlane}.rs`.
#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;

const HEADERS: [&[u8]; 5] = [
    b"url,username,password,totp,extra,name,grouping,fav\n",
    b"name,url,additional_urls,username,password,note,cardholdername,cardnumber,cvc,pin,expirydate,zipcode,folder,full_name,phone_number,email,address1,address2,city,country,state,type,custom_fields\n",
    b"Name,Url,MatchUrl,Login,Pwd,Note,Folder,RfFieldsV2\n",
    b"Group,Title,Username,Password,URL,Notes,TOTP,Icon,Last Modified,Created\n",
    b"username,username2,username3,title,password,note,url,category,otpSecret\n",
];

fuzz_target!(|data: &[u8]| {
    if let Some((&b, rest)) = data.split_first() {
        let mut input = HEADERS[usize::from(b) % HEADERS.len()].to_vec();
        input.extend_from_slice(rest);
        common::check(mosaic_import::csv::parse(&input, None));
    }
});
