//! The handshake on the agent channel (`docs/NATIVE_MESSAGING.md` section 9,
//! CRYPTO_SPEC section 15). The same construction as the browser channel's
//! [`handshake`](crate::handshake), with the `agent/*` labels
//! ([`Channel::Agent`]): a pairing key of one channel fails the other's
//! handshake, because the proofs and the session keys derive from the
//! channel's labels.
//!
//! ```text
//! relay                                        desktop
//! hello {pairingId, nonce_h}            ->
//!                                       <-     challenge {nonce_d, proof_d}   (or refused)
//! finish {proof_h}                      ->
//!                                       <-     AEAD frame 0 (the first event)
//! ```
//!
//! The desktop proves first. Unknown IDs and wrong proofs get the same
//! `refused notPaired`. The proof logic is `PairingKey::*_on` in
//! `mosaic_core`; nothing here derives a key.

use mosaic_core::SecretVec;
use mosaic_core::browser_channel::{
    Channel, ChannelSession, PAIRING_ID_LEN, PairingKey, Role, new_channel_nonce,
};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::agent_preauth::{AgentClientMessage, AgentRefusal, AgentServerMessage};
use crate::error::Error;
use crate::handshake::{next_frame, send, step, write_message};
use crate::hex::Hex;
use crate::protocol::PROTOCOL_VERSION;

/// Reads one pre-auth message from the desktop, within one step timeout.
async fn read_server_message<S: AsyncRead + Unpin>(
    stream: &mut S,
) -> Result<AgentServerMessage, Error> {
    let frame = step(next_frame(stream)).await?;
    AgentServerMessage::parse(&frame)
}

/// Sends `request` and reads the desktop's one answer (`status`,
/// `pairRequest`).
///
/// # Errors
/// [`Error::Timeout`], [`Error::Closed`], framing and parse errors, and
/// [`Error::AgentRefused`] if the desktop answered `refused`.
pub async fn agent_preauth_request<S>(
    stream: &mut S,
    request: &AgentClientMessage,
) -> Result<AgentServerMessage, Error>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    step(write_message(stream, request)).await?;
    match read_server_message(stream).await? {
        AgentServerMessage::Refused { reason, .. } => Err(Error::AgentRefused(reason)),
        other => Ok(other),
    }
}

/// Reads the next pre-auth message after a `pairPending` (the user is
/// deciding), without a step timeout: the caller bounds the wait.
///
/// # Errors
/// [`Error::Closed`], framing and parse errors, [`Error::AgentRefused`].
pub async fn agent_read_server_message_unbounded<S: AsyncRead + Unpin>(
    stream: &mut S,
) -> Result<AgentServerMessage, Error> {
    let frame = next_frame(stream).await?;
    match AgentServerMessage::parse(&frame)? {
        AgentServerMessage::Refused { reason, .. } => Err(Error::AgentRefused(reason)),
        other => Ok(other),
    }
}

/// The relay's side. Returns the session and the plaintext of the desktop's
/// first AEAD frame (the first event; the caller parses it).
///
/// # Errors
/// - [`Error::AgentRefused`]: the desktop sent `refused` (`notPaired`,
///   `locked`, `agentAccessOff`, ...).
/// - [`Error::NotAuthentic`]: the desktop's proof is wrong (a key of the
///   other channel gives this). The relay sends nothing more.
/// - [`Error::Timeout`], [`Error::Closed`], [`Error::Malformed`], framing
///   and channel errors.
pub async fn agent_client_handshake<S>(
    stream: &mut S,
    key: &PairingKey,
    pairing_id: &[u8; PAIRING_ID_LEN],
) -> Result<(ChannelSession, SecretVec), Error>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let nonce_h = new_channel_nonce()?;
    let hello = AgentClientMessage::Hello {
        v: PROTOCOL_VERSION,
        pairing_id: Hex(*pairing_id),
        nonce: Hex(nonce_h),
    };
    step(write_message(stream, &hello)).await?;
    let (nonce_d, proof_d) = match read_server_message(stream).await? {
        AgentServerMessage::Challenge { nonce, proof, .. } => (nonce.0, proof.0),
        AgentServerMessage::Refused { reason, .. } => return Err(Error::AgentRefused(reason)),
        _ => return Err(Error::Malformed),
    };
    key.verify_proof_on(
        Channel::Agent,
        Role::Desktop,
        pairing_id,
        &nonce_h,
        &nonce_d,
        &proof_d,
    )
    .map_err(|_| Error::NotAuthentic)?;
    let proof_h = key.proof_on(Channel::Agent, Role::Host, pairing_id, &nonce_h, &nonce_d)?;
    let finish = AgentClientMessage::Finish {
        v: PROTOCOL_VERSION,
        proof: Hex(proof_h),
    };
    step(write_message(stream, &finish)).await?;
    let mut session = key.session_on(Channel::Agent, Role::Host, pairing_id, &nonce_h, &nonce_d)?;

    let frame = step(next_frame(stream)).await?;
    if frame.first() == Some(&b'{') {
        // Not an AEAD frame: the desktop refused after `finish`.
        return match AgentServerMessage::parse(&frame)? {
            AgentServerMessage::Refused { reason, .. } => Err(Error::AgentRefused(reason)),
            _ => Err(Error::Malformed),
        };
    }
    let first = session.open(&frame)?;
    Ok((session, first))
}

/// Reads the first pre-auth message of a connection on the desktop's side,
/// within one step timeout. A message that is not valid is answered with
/// `refused` (`protocolVersion` or `invalidRequest`) before the error is
/// returned.
///
/// # Errors
/// [`Error::Timeout`], [`Error::Closed`], [`Error::ProtocolVersion`],
/// [`Error::Malformed`] and framing errors.
pub async fn agent_read_client_message<S>(stream: &mut S) -> Result<AgentClientMessage, Error>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let frame = step(next_frame(stream)).await?;
    match AgentClientMessage::parse(&frame) {
        Ok(message) => Ok(message),
        Err(e) => {
            let reason = if e == Error::ProtocolVersion {
                AgentRefusal::ProtocolVersion
            } else {
                AgentRefusal::InvalidRequest
            };
            let _ = write_message(stream, &AgentServerMessage::refused(reason)).await;
            Err(e)
        }
    }
}

/// The desktop's side, after [`agent_read_client_message`] returned a
/// `hello`. `key` is the stored key of `pairing_id`, if there is one;
/// `first` is the plaintext of the first AEAD frame (the first event).
///
/// # Errors
/// - [`Error::AgentRefused`]`(`[`AgentRefusal::NotPaired`]`)`: no such
///   pairing, or the relay's proof is wrong (the same answer in both cases).
/// - [`Error::Timeout`], [`Error::Closed`] (for example when the relay
///   rejected the desktop's proof and hung up), [`Error::Malformed`].
pub async fn agent_server_handshake<S>(
    stream: &mut S,
    pairing_id: &[u8; PAIRING_ID_LEN],
    nonce_h: &[u8; 32],
    key: Option<&PairingKey>,
    first: &[u8],
) -> Result<ChannelSession, Error>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let Some(key) = key else {
        let _ = write_message(
            stream,
            &AgentServerMessage::refused(AgentRefusal::NotPaired),
        )
        .await;
        return Err(Error::AgentRefused(AgentRefusal::NotPaired));
    };
    let nonce_d = new_channel_nonce()?;
    let proof_d = key.proof_on(Channel::Agent, Role::Desktop, pairing_id, nonce_h, &nonce_d)?;
    let challenge = AgentServerMessage::Challenge {
        v: PROTOCOL_VERSION,
        nonce: Hex(nonce_d),
        proof: Hex(proof_d),
    };
    step(write_message(stream, &challenge)).await?;
    let AgentClientMessage::Finish { proof, .. } = agent_read_client_message(stream).await? else {
        let _ = write_message(
            stream,
            &AgentServerMessage::refused(AgentRefusal::InvalidRequest),
        )
        .await;
        return Err(Error::Malformed);
    };
    if key
        .verify_proof_on(
            Channel::Agent,
            Role::Host,
            pairing_id,
            nonce_h,
            &nonce_d,
            &proof.0,
        )
        .is_err()
    {
        let _ = write_message(
            stream,
            &AgentServerMessage::refused(AgentRefusal::NotPaired),
        )
        .await;
        return Err(Error::AgentRefused(AgentRefusal::NotPaired));
    }
    let mut session =
        key.session_on(Channel::Agent, Role::Desktop, pairing_id, nonce_h, &nonce_d)?;
    send(stream, &mut session, first).await?;
    Ok(session)
}

#[cfg(test)]
mod tests;
