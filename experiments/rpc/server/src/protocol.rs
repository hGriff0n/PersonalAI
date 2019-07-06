
// macro imports

// standard imports

// third-party imports
use serde::{Serialize, Deserialize};
use tokio::prelude::*;

// local imports


//
// Exported functions
//

// TODO: Write this in docs.rs style?
// Helper function to setup the connection for reading/writing
//  1) Split the connection into read/write ends
//  2) Wrap the connection in a codec to handle packet framing
//  3) Wrap the connection in a protocol so interaction is in the message layer
pub(crate) fn frame_with_protocol<P, Conn, Codec>(conn: Conn, codec_producer: &Fn() -> Codec)
    -> (P::Reader, P::Writer)
    where Conn: AsyncRead + AsyncWrite,
          Codec: tokio::codec::Decoder + tokio::codec::Encoder,
          P: RpcProtocol<Conn, Codec>
{
    let (reader, writer) = conn.split();
    (P::make_reader(reader, codec_producer), P::make_writer(writer, codec_producer))
}


//
// Trait definitions
//

// Helper trait to wrap the code to frame a tcp communication in a generic fashion
// TODO: Improve dependency injection?
// TODO: The type parameters are a bit unfortunate
pub trait RpcProtocol<Conn, Codec>
    where Conn: AsyncRead + AsyncWrite,
          Codec: tokio::codec::Decoder + tokio::codec::Encoder,
{
    type Reader;
    type Writer;

    fn make_reader(read_conn: tokio::io::ReadHalf<Conn>, codec_producer: &Fn() -> Codec) -> Self::Reader;
    fn make_writer(write_conn: tokio::io::WriteHalf<Conn>, codec_producer: &Fn() -> Codec) -> Self::Writer;
}

// Helper trait to wrap serialization of messages
pub trait RpcSerializer: Clone+Copy {
    type Message;

    // TODO: Convert these to static methods
    fn from_value<T>(msg: Self::Message) -> Result<T, std::io::Error>
        where for<'d> T: Deserialize<'d>;
    fn to_value<T>(msg: T) -> Result<Self::Message, std::io::Error>
        where T: Serialize;
}

//
// Trait Implementations
//

// Protocol to handle and serialize json messages
#[derive(Clone, Copy)]
pub struct JsonProtocol;
impl RpcSerializer for JsonProtocol {
    type Message = serde_json::Value;

    fn from_value<T>(msg: Self::Message) -> Result<T, std::io::Error>
        where for<'d> T: Deserialize<'d>
    {
        serde_json::from_value(msg)
            .map_err(|_err| std::io::Error::new(std::io::ErrorKind::InvalidInput, "failed to deserialize message"))
    }

    fn to_value<T: Serialize>(msg: T) -> Result<Self::Message, std::io::Error> {
        serde_json::to_value(msg)
            .map_err(|_err| std::io::Error::new(std::io::ErrorKind::InvalidData, "failed to serialize message"))
    }
}
impl<Conn, Codec> RpcProtocol<Conn, Codec> for JsonProtocol
    where
          // Check that generics conform to the required type scheme of RpcProtocol
          Conn: AsyncRead + AsyncWrite,
          Codec: tokio::codec::Decoder + tokio::codec::Encoder,

          // Check that the codec also conforms to the type requires of JsonWriter
          bytes::BytesMut: std::convert::From<<Codec as tokio::codec::Decoder>::Item>,
          <Codec as tokio::codec::Decoder>::Error: std::convert::From<serde_json::Error>,
          tokio::codec::FramedWrite<tokio::io::WriteHalf<Conn>, Codec>: Sink<SinkItem = bytes::Bytes>,
          <tokio::codec::FramedWrite<tokio::io::WriteHalf<Conn>, Codec> as futures::Sink>::SinkError: std::convert::From<serde_json::Error>,
{
    type Reader = tokio_serde_json::ReadJson<
        tokio::codec::FramedRead<tokio::io::ReadHalf<Conn>, Codec>,
        serde_json::Value>;
    type Writer = tokio_serde_json::WriteJson<
        tokio::codec::FramedWrite<tokio::io::WriteHalf<Conn>, Codec>,
        serde_json::Value>;

    fn make_reader(read_conn: tokio::io::ReadHalf<Conn>, codec_producer: &Fn() -> Codec) -> Self::Reader {
        tokio_serde_json::ReadJson::new(tokio::codec::FramedRead::new(read_conn, codec_producer()))
    }

    fn make_writer(write_conn: tokio::io::WriteHalf<Conn>, codec_producer: &Fn() -> Codec) -> Self::Writer {
        tokio_serde_json::WriteJson::new(tokio::codec::FramedWrite::new(write_conn, codec_producer()))
    }
}
