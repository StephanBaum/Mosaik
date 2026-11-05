//! The handshake and the AEAD frames on the host-to-desktop link
//! (`docs/NATIVE_MESSAGING.md` §4, CRYPTO_SPEC §15).
//!
//! ```text
//! host                                         desktop
//! hello {pairingId, nonce_h}            ->
//!                                       <-     challenge {nonce_d, proof_d}   (or refused)
//! finish {proof_h}                      ->
//!                                       <-     AEAD frame 0: {"event":"state",...}
//! ```
//!
//! The desktop proves first, so a host never reveals its proof to a program
//! that does not know `K_pair`. Each step has a timeout of
//! [`HANDSHAKE_STEP_TIMEOUT_SECS`](crate::protocol::HANDSHAKE_STEP_TIMEOUT_SECS).
//! [`client_handshake`] is the host's side; [`server_handshake`] is what the
//! desktop runs after it read a `hello`.

use core::time::Duration;

use mosaic_core::browser_channel::{
    ChannelSession, PAIRING_ID_LEN, PairingKey, Role, new_channel_nonce,
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::timeout;

use crate::error::Error;
use crate::framing::{ByteOrder, read_frame, write_frame};
use crate::hex::Hex;
use crate::preauth::{ClientMessage, ServerMessage};
use crate::protocol::{
    DesktopMessage, ErrorCode, Event, HANDSHAKE_STEP_TIMEOUT_SECS, LockState, PROTOCOL_VERSION,
    to_zeroizing,
};

/// The per-step timeout.
pub const STEP_TIMEOUT: Duration = Duration::from_secs(HANDSHAKE_STEP_TIMEOUT_SECS);

/// Runs `fut` for at most [`STEP_TIMEOUT`].
pub(crate) async fn step<T>(fut: impl Future<Output = Result<T, Error>>) -> Result<T, Error> {
    timeout(STEP_TIMEOUT, fut)
        .await
        .map_err(|_| Error::Timeout)?
}

/// Writes a pre-auth message as one big-endian frame.
///
/// # Errors
/// [`Error::Io`] on a write failure.
pub async fn write_message<S, M>(stream: &mut S, message: &M) -> Result<(), Error>
where
    S: AsyncWrite + Unpin,
    M: serde::Serialize,
{
    write_frame(stream, ByteOrder::Big, &to_zeroizing(message)?).await
}

/// Reads a frame; a closed connection is [`Error::Closed`].
pub(crate) async fn next_frame<S: AsyncRead + Unpin>(
    stream: &mut S,
) -> Result<zeroize::Zeroizing<Vec<u8>>, Error> {
    read_frame(stream, ByteOrder::Big)
        .await?
        .ok_or(Error::Closed)
}

/// Reads one pre-auth message from the desktop, within one step timeout.
///
/// # Errors
/// [`Error::Timeout`], [`Error::Closed`], framing errors and parse errors.
pub async fn read_server_message<S: AsyncRead + Unpin>(
    stream: &mut S,
) -> Result<ServerMessage, Error> {
    let frame = step(next_frame(stream)).await?;
    ServerMessage::parse(&frame)
}

/// Sends `request` and reads the desktop's one answer (`status`, `focus`).
///
/// # Errors
/// As [`read_server_message`], plus [`Error::Refused`] if the desktop
/// answered `refused`.
pub async fn preauth_request<S>(
    stream: &mut S,
    request: &ClientMessage,
) -> Result<ServerMessage, Error>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    step(write_message(stream, request)).await?;
    match read_server_message(stream).await? {
        ServerMessage::Refused { reason, .. } => Err(Error::Refused(reason)),
        other => Ok(other),
    }
}

/// The host's side. Returns the session and the desktop's lock state (its
/// first AEAD frame).
///
/// # Errors
/// - [`Error::Refused`]: the desktop sent `refused` (`notPaired`,
///   `protocolVersion`, ...).
/// - [`Error::NotAuthentic`]: the desktop's proof is wrong. The host sends
///   nothing more.
/// - [`Error::Timeout`], [`Error::Closed`], [`Error::Malformed`], framing
///   and channel errors.
pub async fn client_handshake<S>(
    stream: &mut S,
    key: &PairingKey,
    pairing_id: &[u8; PAIRING_ID_LEN],
) -> Result<(ChannelSession, LockState), Error>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let nonce_h = new_channel_nonce()?;
    let hello = ClientMessage::Hello {
        v: PROTOCOL_VERSION,
        pairing_id: Hex(*pairing_id),
        nonce: Hex(nonce_h),
    };
    step(write_message(stream, &hello)).await?;
    let (nonce_d, proof_d) = match read_server_message(stream).await? {
        ServerMessage::Challenge { nonce, proof, .. } => (nonce.0, proof.0),
        ServerMessage::Refused { reason, .. } => return Err(Error::Refused(reason)),
        _ => return Err(Error::Malformed),
    };
    key.verify_proof(Role::Desktop, pairing_id, &nonce_h, &nonce_d, &proof_d)
        .map_err(|_| Error::NotAuthentic)?;
    let proof_h = key.proof(Role::Host, pairing_id, &nonce_h, &nonce_d)?;
    let finish = ClientMessage::Finish {
        v: PROTOCOL_VERSION,
        proof: Hex(proof_h),
    };
    step(write_message(stream, &finish)).await?;
    let mut session = key.session(Role::Host, pairing_id, &nonce_h, &nonce_d)?;

    let frame = step(next_frame(stream)).await?;
    if frame.first() == Some(&b'{') {
        // Not an AEAD frame: the desktop refused after `finish`.
        return match ServerMessage::parse(&frame)? {
            ServerMessage::Refused { reason, .. } => Err(Error::Refused(reason)),
            _ => Err(Error::Malformed),
        };
    }
    let plaintext = session.open(&frame)?;
    match DesktopMessage::parse(plaintext.expose_secret())? {
        DesktopMessage::Event(Event::State { state, .. }) => Ok((session, state)),
        _ => Err(Error::Malformed),
    }
}

/// Reads the first pre-auth message of a connection on the desktop's side,
/// within one step timeout. A message that is not valid is answered with
/// `refused` (`protocolVersion` or `invalidRequest`) before the error is
/// returned.
///
/// # Errors
/// [`Error::Timeout`], [`Error::Closed`], [`Error::ProtocolVersion`],
/// [`Error::Malformed`] and framing errors.
pub async fn read_client_message<S>(stream: &mut S) -> Result<ClientMessage, Error>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let frame = step(next_frame(stream)).await?;
    match ClientMessage::parse(&frame) {
        Ok(message) => Ok(message),
        Err(e) => {
            let reason = if e == Error::ProtocolVersion {
                ErrorCode::ProtocolVersion
            } else {
                ErrorCode::InvalidRequest
            };
            let _ = write_message(stream, &ServerMessage::refused(reason)).await;
            Err(e)
        }
    }
}

/// The desktop's side, after [`read_client_message`] returned a
/// [`ClientMessage::Hello`]. `key` is the stored key of `pairing_id`, if
/// there is one; `state` is the lock state to report in the first frame.
///
/// # Errors
/// - [`Error::Refused`]`(`[`ErrorCode::NotPaired`]`)`: no such pairing, or
///   the host's proof is wrong (the desktop sent `refused notPaired`, the
///   same answer in both cases).
/// - [`Error::Timeout`], [`Error::Closed`] (for example when the host
///   rejected the desktop's proof and hung up), [`Error::Malformed`].
pub async fn server_handshake<S>(
    stream: &mut S,
    pairing_id: &[u8; PAIRING_ID_LEN],
    nonce_h: &[u8; 32],
    key: Option<&PairingKey>,
    state: LockState,
) -> Result<ChannelSession, Error>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let Some(key) = key else {
        let _ = write_message(stream, &ServerMessage::refused(ErrorCode::NotPaired)).await;
        return Err(Error::Refused(ErrorCode::NotPaired));
    };
    let nonce_d = new_channel_nonce()?;
    let proof_d = key.proof(Role::Desktop, pairing_id, nonce_h, &nonce_d)?;
    let challenge = ServerMessage::Challenge {
        v: PROTOCOL_VERSION,
        nonce: Hex(nonce_d),
        proof: Hex(proof_d),
    };
    step(write_message(stream, &challenge)).await?;
    let ClientMessage::Finish { proof, .. } = read_client_message(stream).await? else {
        let _ = write_message(stream, &ServerMessage::refused(ErrorCode::InvalidRequest)).await;
        return Err(Error::Malformed);
    };
    let proof_h = proof.0;
    if key
        .verify_proof(Role::Host, pairing_id, nonce_h, &nonce_d, &proof_h)
        .is_err()
    {
        let _ = write_message(stream, &ServerMessage::refused(ErrorCode::NotPaired)).await;
        return Err(Error::Refused(ErrorCode::NotPaired));
    }
    let mut session = key.session(Role::Desktop, pairing_id, nonce_h, &nonce_d)?;
    let first = to_zeroizing(&Event::state(state))?;
    send(stream, &mut session, &first).await?;
    Ok(session)
}

/// Seals `plaintext` and writes it as one frame.
///
/// # Errors
/// [`Error::Channel`] if sealing fails, [`Error::FrameTooLarge`] if the frame
/// would exceed the cap, [`Error::Io`] on a write failure.
pub async fn send<S: AsyncWrite + Unpin>(
    stream: &mut S,
    session: &mut ChannelSession,
    plaintext: &[u8],
) -> Result<(), Error> {
    let frame = session.seal(plaintext)?;
    write_frame(stream, ByteOrder::Big, &frame).await
}

#[cfg(test)]
mod tests;
