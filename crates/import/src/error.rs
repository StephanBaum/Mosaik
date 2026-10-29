//! Errors of the importers.
//!
//! No variant carries a value from the export file: no field, title, file
//! name, password or byte offset into a secret. They name only what kind of
//! problem the file has, so they can be logged and shown to the user.

/// Why an export file could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The file is larger than [`crate::MAX_FILE_LEN`], or a part of it
    /// (a decompressed payload, an archive member) is larger than its limit.
    #[error("file too large")]
    TooLarge,

    /// The file is not a well-formed export of the chosen format.
    #[error("not a valid export of this format")]
    Malformed,

    /// The file is a version of the format this build does not read (for
    /// example KDBX 3.1).
    #[error("unsupported format version")]
    UnsupportedVersion,

    /// The file uses a cipher, key derivation or compression this build does
    /// not read (for example Twofish in a KDBX file).
    #[error("unsupported cipher, key derivation or compression")]
    Unsupported,

    /// The export is encrypted with a password this importer does not
    /// accept (a password-protected Bitwarden export). Export it again
    /// without encryption.
    #[error("encrypted export")]
    Encrypted,

    /// The file needs a password (KDBX) and none was given.
    #[error("password required")]
    PasswordRequired,

    /// The password does not open the file, or the file was modified. KDBX
    /// cannot tell the two apart.
    #[error("wrong password or damaged file")]
    WrongCredentials,

    /// The file asks for more work or memory than the importer allows: too
    /// many entries, nesting too deep, or key-derivation parameters above
    /// the limits in [`crate::kdbx`].
    #[error("file exceeds an import limit")]
    LimitExceeded,

    /// Memory for the file could not be reserved.
    #[error("allocation failed")]
    Allocation,
}

/// `Result` with [`Error`].
pub type Result<T> = core::result::Result<T, Error>;
