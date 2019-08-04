
import abc
import json
import typing

import rpc

class Protocol(object):
    M = typing.TypeVar('M', bound=rpc.BaseMessage)

    @abc.abstractmethod
    def encode(self, msg: Protocol.M) -> bytes:
        """
        Convert the rpc.Message object into a byte string for sending through the socket
        """

    @abc.abstractmethod
    def decode(self, msg_buf: bytes, msg_class: typing.Type[Protocol.M]) -> Protocol.M:
        """
        Convert the byte string into a the given rpc.Message class
        """

class JsonProtocol(Protocol):
    """
    Helper object to encapsulate handling parsing to/from messages
    """

    def __init__(self, logger) -> None:
        self._logger = logger

    def encode(self, msg: Protocol.M) -> bytes:
        return json.dumps(msg.to_dict()).encode('utf-8')

    def decode(self, msg_buf: bytes, msg_class: typing.Type[Protocol.M]) -> Protocol.M:
        return msg_class.from_dict(json.loads(msg_buf.decode('utf-8')))
