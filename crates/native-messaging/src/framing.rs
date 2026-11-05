//! Length-prefixed frames (`docs/NATIVE_MESSAGING.md` §3).
//!
//! | Link | Prefix |
//! |---|---|
//! | browser and host (stdio) | `u32`, native byte order |
//! | host and desktop | `u32`, big-endian |
//!
//! A length of 0, one above [`MAX_FRAME_BYTES`] and a stream that ends inside
//! a frame are errors. A stream that ends between frames is a clean close.
//! Read buffers are zeroized on drop; a frame is written with one
//! `write_all` of prefix and payload together.

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use zeroize::Zeroizing;

use crate::error::Error;
use crate::protocol::MAX_FRAME_BYTES;

/// The byte order of a frame's length prefix.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ByteOrder {
    /// The browser's native messaging stdio framing.
    Native,
    /// The host-to-desktop framing.
    Big,
}

impl ByteOrder {
    fn encode(self, len: u32) -> [u8; 4] {
        match self {
            Self::Native => len.to_ne_bytes(),
            Self::Big => len.to_be_bytes(),
        }
    }

    fn decode(self, prefix: [u8; 4]) -> u32 {
        match self {
            Self::Native => u32::from_ne_bytes(prefix),
            Self::Big => u32::from_be_bytes(prefix),
        }
    }
}

/// Validates a frame length from a prefix.
///
/// # Errors
/// [`Error::FrameEmpty`] for 0, [`Error::FrameTooLarge`] above the cap.
pub fn check_len(len: u32) -> Result<usize, Error> {
    let len = usize::try_from(len).map_err(|_| Error::FrameTooLarge)?;
    match len {
        0 => Err(Error::FrameEmpty),
        n if n > MAX_FRAME_BYTES => Err(Error::FrameTooLarge),
        n => Ok(n),
    }
}

/// Reads one frame. `Ok(None)` if the stream ended before the first byte of
/// a prefix.
///
/// # Errors
/// [`Error::FrameEmpty`], [`Error::FrameTooLarge`] (before any payload is
/// read), [`Error::FrameTruncated`] if the stream ends inside the prefix or
/// the payload, [`Error::Io`] otherwise.
pub async fn read_frame<R: AsyncRead + Unpin>(
    reader: &mut R,
    order: ByteOrder,
) -> Result<Option<Zeroizing<Vec<u8>>>, Error> {
    let mut prefix = [0u8; 4];
    let mut filled = 0;
    while filled < prefix.len() {
        let Some(rest) = prefix.get_mut(filled..) else {
            return Err(Error::Malformed);
        };
        let n = reader.read(rest).await?;
        if n == 0 {
            return if filled == 0 {
                Ok(None)
            } else {
                Err(Error::FrameTruncated)
            };
        }
        filled += n;
    }
    let len = check_len(order.decode(prefix))?;
    let mut payload = Zeroizing::new(vec![0u8; len]);
    reader
        .read_exact(payload.as_mut_slice())
        .await
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::UnexpectedEof => Error::FrameTruncated,
            kind => Error::Io(kind),
        })?;
    Ok(Some(payload))
}

/// Writes one frame (prefix and payload in one `write_all`) and flushes.
///
/// # Errors
/// [`Error::FrameEmpty`] or [`Error::FrameTooLarge`] for a payload the
/// peer would refuse; [`Error::Io`] on a write failure.
pub async fn write_frame<W: AsyncWrite + Unpin>(
    writer: &mut W,
    order: ByteOrder,
    payload: &[u8],
) -> Result<(), Error> {
    let len = u32::try_from(payload.len()).map_err(|_| Error::FrameTooLarge)?;
    check_len(len)?;
    let mut buf = Zeroizing::new(Vec::with_capacity(payload.len() + 4));
    buf.extend_from_slice(&order.encode(len));
    buf.extend_from_slice(payload);
    writer.write_all(&buf).await?;
    writer.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::block_on;

    fn read(order: ByteOrder, bytes: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        let mut r = bytes;
        block_on(async { read_frame(&mut r, order).await }).map(|o| o.map(|f| f.to_vec()))
    }

    fn framed(order: ByteOrder, payload: &[u8]) -> Vec<u8> {
        let mut out = order.encode(u32::try_from(payload.len()).unwrap()).to_vec();
        out.extend_from_slice(payload);
        out
    }

    #[test]
    fn round_trip_both_orders() {
        for order in [ByteOrder::Native, ByteOrder::Big] {
            let mut sink = Vec::new();
            block_on(async { write_frame(&mut sink, order, b"hello").await }).unwrap();
            assert_eq!(sink, framed(order, b"hello"));
            assert_eq!(read(order, &sink).unwrap().unwrap(), b"hello");
        }
        assert_eq!(framed(ByteOrder::Big, b"x"), [0, 0, 0, 1, b'x']);
    }

    #[test]
    fn cap_boundaries() {
        let order = ByteOrder::Big;
        let at_cap = vec![7u8; MAX_FRAME_BYTES];
        assert_eq!(
            read(order, &framed(order, &at_cap)).unwrap().unwrap(),
            at_cap
        );
        let over = framed(order, &vec![0u8; MAX_FRAME_BYTES + 1]);
        assert_eq!(read(order, &over), Err(Error::FrameTooLarge));
        assert_eq!(
            read(order, &u32::MAX.to_be_bytes()),
            Err(Error::FrameTooLarge),
            "refused before reading a payload"
        );
        assert_eq!(read(order, &[0, 0, 0, 0]), Err(Error::FrameEmpty));
        let mut sink = Vec::new();
        assert_eq!(
            block_on(async { write_frame(&mut sink, order, &[]).await }),
            Err(Error::FrameEmpty)
        );
        assert_eq!(
            block_on(async { write_frame(&mut sink, order, &over).await }),
            Err(Error::FrameTooLarge)
        );
        assert_eq!(sink.len(), 0);
    }

    #[test]
    fn truncation_and_clean_close() {
        let order = ByteOrder::Native;
        assert_eq!(read(order, &[]), Ok(None), "clean close");
        assert_eq!(
            read(order, &[1, 0]),
            Err(Error::FrameTruncated),
            "in the prefix"
        );
        let mut cut = framed(order, b"hello");
        cut.truncate(cut.len() - 1);
        assert_eq!(
            read(order, &cut),
            Err(Error::FrameTruncated),
            "in the payload"
        );
        let mut two = framed(order, b"a");
        two.extend_from_slice(&framed(order, b"bc"));
        let mut stream = two.as_slice();
        let (first, second, third) = block_on(async {
            (
                read_frame(&mut stream, order).await,
                read_frame(&mut stream, order).await,
                read_frame(&mut stream, order).await,
            )
        });
        assert_eq!(first.unwrap().unwrap().to_vec(), b"a");
        assert_eq!(second.unwrap().unwrap().to_vec(), b"bc");
        assert_eq!(third.unwrap().map(|f| f.to_vec()), None);
    }
}
