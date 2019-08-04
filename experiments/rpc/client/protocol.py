
import abc
import json
import struct
import typing

import rpc

class Protocol(object):
    M = typing.TypeVar('M', bound=rpc.BaseMessage)

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
    def encode(self, msg: Protocol.M) -> rpc.UntypedMessage:
        """
        Convert the rpc.Message object into an rpc.UntypedMessage
        """

    @abc.abstractmethod
    def decode(self, msg: rpc.UntypedMessage, msg_class: typing.Type[Protocol.M]) -> typing.Optional[Protocol.M]:
        """
        Convert the rpc.UntypedMessage into the given rpc.Message class
        """


class JsonProtocol(Protocol):
    """
    Helper object to encapsulate handling parsing to/from messages
    """

    def __init__(self, logger) -> None:
        self._logger = logger

    def make_packet(self, msg: rpc.Message) -> bytes:
        buf = json.dumps(self.encode(msg)).encode('utf-8')
        frame = struct.pack('>I', len(buf))
        return frame + buf

    def unwrap_packet(self, socket_reader: typing.Callable[[int], bytes]) -> typing.Optional[rpc.Message]:
        len_buf = socket_reader(4)
        msg_len = struct.unpack('>I', len_buf)[0]

        msg_buf = socket_reader(msg_len).decode('utf-8')
        return self.decode(json.loads(msg_buf), rpc.Message)

    def encode(self, msg: Protocol.M) -> rpc.UntypedMessage:
        return msg.to_dict()

    def decode(self, msg: rpc.UntypedMessage, msg_class: typing.Type[Protocol.M]) -> typing.Optional[Protocol.M]:
        decoded_msg = msg_class.from_dict(msg)
        if decoded_msg is None and self._logger is not None:
            self._logger.warning("Failed to decode message {} to rpc.Message type `{}`", msg, msg_class)
        return decoded_msg
