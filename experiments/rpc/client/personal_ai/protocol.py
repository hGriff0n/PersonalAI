
# standard imports
import abc
import json
import struct
import typing

# third-part imports

# local imports
from personal_ai import rpc

class Protocol(object):
    M = typing.TypeVar('M', bound=rpc.Serializable)

    @abc.abstractmethod
    def make_packet(self, msg: rpc.Message) -> bytes:
        """
        Convert the rpc.Message object into a bytes string for sending through the socket
        """

    @abc.abstractmethod
    def unwrap_packet(self, socket_reader: typing.Callable[[int], bytes]) -> typing.Optional[rpc.Message]:
        """
        Convert the byte string into a rpc.Message

        socket_reader is a function that takes a number of bytes to read from the socket and returns the bytes
        """

    @abc.abstractmethod
    def _serialize(self, msg: 'Protocol.M') -> rpc.SerializedMessage:
        """
        Convert the rpc.Message object into an rpc.SerializedMessage
        """

    @abc.abstractmethod
    def _unserialize(self, msg: rpc.SerializedMessage, msg_class: typing.Type['Protocol.M']) -> typing.Optional['Protocol.M']:
        """
        Convert the rpc.SerializedMessage into the given rpc.Message class
        """


class JsonProtocol(Protocol):
    """
    Helper object to encapsulate handling parsing to/from messages
    """

    def make_packet(self, msg: rpc.Message) -> bytes:
        buf = json.dumps(self._serialize(msg)).encode('utf-8')
        frame = struct.pack('>I', len(buf))
        return frame + buf

    def unwrap_packet(self, socket_reader: typing.Callable[[int], bytes]) -> typing.Optional[rpc.Message]:
        len_buf = socket_reader(4)
        msg_len = struct.unpack('>I', len_buf)[0]

        msg_buf = socket_reader(msg_len).decode('utf-8')
        return self._unserialize(json.loads(msg_buf), rpc.Message)

    def _serialize(self, msg: Protocol.M) -> rpc.SerializedMessage:
        return msg.serialize()

    def _unserialize(self, msg: rpc.SerializedMessage, msg_class: typing.Type[Protocol.M]) -> typing.Optional[Protocol.M]:
        return msg_class.from_dict(msg)
