
# standard imports
import asyncio
import queue
import socket
import typing
import uuid

# third-part imports

# local imports
import rpc
import protocol


class MessageEvent(asyncio.Event):
    """
    Custom event that allows us to introduce an asnc boundary when waiting for a response to come in
    """
    value: typing.Optional[rpc.Message] = None


class CommunicationHandler(object):
    """
    Handle communication between the plugins and the reader/writer threads
    Enables plugins to send a message and asynchronously "wait" for a response
    """

    def __init__(self, write_queue: 'queue.Queue[rpc.Message]', logger: typing.Optional[typing.Any] = None) -> None:
        self._logger = logger
        self._write_queue = write_queue
        self._waiting_messages: typing.Dict[uuid.UUID, MessageEvent] = {}

    @property
    def write_queue(self) -> 'queue.Queue[rpc.Message]':
        return self._write_queue

    @property
    def waiting_messages(self) -> typing.Dict[uuid.UUID, MessageEvent]:
        return self._waiting_messages

    def send(self, msg: rpc.Message) -> MessageEvent:
        """
        Send the message out to the network
        Returns an awaitable MessageEvent which willl eventually hold the response (if one is returned)
        """
        self._write_queue.put(msg)
        self._waiting_messages[msg.msg_id] = MessageEvent()
        return self._waiting_messages[msg.msg_id]

    def drop_message(self, msg: rpc.Message):
        """
        Delete the response object as we no longer care about the message
        This is mainly for internal cleanup when dealing with no-return rpcs

        TODO: Do I need code to explicitly drop the message if a response does come in?
        """
        if msg.msg_id in self._waiting_messages:
            del self._waiting_messages[msg.msg_id]

    async def wait_response(self, msg: rpc.Message) -> typing.Optional[rpc.Message]:
        """
        Helper method to send a message and immediately wait for its response
        This method demonstrates the behavior for waiting on a specific response
        """
        continuation = self.send(msg)
        await continuation.wait()

        resp = continuation.value
        del self._waiting_messages[msg.msg_id]
        return resp


class NetworkQueue(object):
    """
    Helper object to manage direct interactions with the socket communication

    Abstracts away any dependencies on a specific netowrk or protocol
    """

    SOCKET_TIMEOUT: typing.ClassVar[int] = 5

    def __init__(self, socket, proto: protocol.Protocol, logger) -> None:
        self._sock = socket
        self._logger = logger
        self._protocol = proto

        self._sock.settimeout(NetworkQueue.SOCKET_TIMEOUT)

    def send_message(self, msg: rpc.Message) -> None:
        """
        Push the specific message out to the network
        """
        packet = self._protocol.make_packet(msg)
        self._sock.sendall(packet)

        if self._logger:
            self._logger.info("Sent message {}".format(msg))

    def get_message(self) -> typing.Optional[rpc.Message]:
        """
        Wait for a message to come in from the network
        """
        def _read(n: int) -> bytes:
            return self._sock.recv(n)

        try:
            msg = self._protocol.unwrap_packet(_read)

        # If the socket gets shutdown (on windows), it may produce an error (as it's "not a socket" anymore)
        except (socket.timeout, OSError):
            return None

        if self._logger:
            if msg is not None:
                self._logger.info("Received message {}".format(msg.msg_id))
            else:
                self._logger.error("Failed receiving message!!!")
        return msg
