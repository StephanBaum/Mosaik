//! Where a provider folder is: what the phone keeps next to the credential
//! (the device-local remote folder location; it holds no secret).

use serde::{Deserialize, Serialize};

use super::credential::Provider;
use crate::transport::TransportError;

/// The longest encoded location.
const MAX_LEN: usize = 4096;

/// A folder on a provider.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FolderLocation {
    /// The provider.
    pub provider: Provider,
    /// The folder as the user chose it: a path below the account's root (a
    /// display path for providers that address by id).
    pub root: String,
    /// `WebDAV`: the canonical origin of the server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
    /// Graph: the drive that holds the folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drive_id: Option<String>,
    /// Graph: the folder's item id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
    /// Dropbox: the namespace path root of a team folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_root: Option<String>,
}

impl FolderLocation {
    /// The JSON text of the location.
    #[must_use]
    pub fn encode(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// The inverse of [`encode`](Self::encode).
    ///
    /// # Errors
    /// [`TransportError::Protocol`] for text over 4096 bytes, malformed JSON
    /// or an unknown field.
    pub fn decode(text: &str) -> Result<Self, TransportError> {
        if text.len() > MAX_LEN {
            return Err(TransportError::Protocol);
        }
        serde_json::from_str(text).map_err(|_| TransportError::Protocol)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn location_json_round_trip_and_unknown_field_refused() {
        let location = FolderLocation {
            provider: Provider::WebDav,
            root: "Mosaic/Family".to_owned(),
            server: Some("https://cloud.example.com".to_owned()),
            drive_id: None,
            item_id: None,
            path_root: None,
        };
        let text = location.encode();
        assert_eq!(FolderLocation::decode(&text).unwrap(), location);
        assert!(text.contains("\"provider\":\"webdav\""));
        assert!(!text.contains("driveId"));
        let graph = FolderLocation {
            provider: Provider::OneDrive,
            root: "/Mosaic".to_owned(),
            server: None,
            drive_id: Some("b!abc".to_owned()),
            item_id: Some("01XYZ".to_owned()),
            path_root: None,
        };
        assert_eq!(FolderLocation::decode(&graph.encode()).unwrap(), graph);
        for bad in [
            r#"{"provider":"webdav","root":"x","surprise":1}"#,
            r#"{"provider":"ftp","root":"x"}"#,
            r#"{"root":"x"}"#,
            "not json",
        ] {
            assert_eq!(
                FolderLocation::decode(bad),
                Err(TransportError::Protocol),
                "{bad}"
            );
        }
        let long = format!(
            r#"{{"provider":"webdav","root":"{}"}}"#,
            "x".repeat(MAX_LEN)
        );
        assert_eq!(FolderLocation::decode(&long), Err(TransportError::Protocol));
    }
}
