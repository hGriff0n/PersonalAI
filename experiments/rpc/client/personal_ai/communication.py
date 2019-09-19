
# standard imports
import asyncio
import queue
import socket
import typing
import uuid

# third-part imports

# local imports
from personal_ai import rpc
from personal_ai import protocol
from personal_ai.logger import Logger


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

    def __init__(self, write_queue: 'queue.Queue[rpc.Message]', log: Logger) -> None:
        self._log = log
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
        self._log.debug("Placed msg id={} on waiting queue. Listening for response".format(msg.msg_id))
        return self._waiting_messages[msg.msg_id]

    def drop_message(self, msg: rpc.Message):
        """
        Delete the response object as we no longer care about the message
        This is mainly for internal cleanup when dealing with no-return rpcs

        TODO: Do I need code to explicitly drop the message if a response does come in?
        """
        if msg.msg_id in self._waiting_messages:
            self._log.debug("Stop listening for message id={}".format(msg.msg_id))
            del self._waiting_messages[msg.msg_id]
        else:
            self._log.warning("Attempt to drop message that no one is listening on (id={})".format(msg.msg_id))

    async def wait_response(self, msg: rpc.Message) -> typing.Optional[rpc.Message]:
        """
        Helper method to send a message and immediately wait for its response
        This method demonstrates the behavior for waiting on a specific response
        """
        continuation = self.send(msg)
        await continuation.wait()

        self._log.debug("Detected response on message id={}. Resuming".format(msg.msg_id))
        resp = continuation.value
        del self._waiting_messages[msg.msg_id]
        return resp


class NetworkQueue(object):
    """
    Helper object to manage direct interactions with the socket communication

    Abstracts away any dependencies on a specific netowrk or protocol
    """

    SOCKET_TIMEOUT: typing.ClassVar[int] = 5

    def __init__(self, socket, proto: protocol.Protocol, log) -> None:
        self._sock = socket
        self._log = log
        self._protocol = proto

        self._sock.settimeout(NetworkQueue.SOCKET_TIMEOUT)

    @property
    def logger(self) -> Logger:
        return self._log

    def send_message(self, msg: rpc.Message) -> None:
        """
        Push the specific message out to the network
        """
        packet = self._protocol.make_packet(msg)
        self._sock.sendall(packet)
        self._log.debug("Sent message id={}".format(msg.msg_id))

    def get_message(self) -> typing.Optional[rpc.Message]:
        """
        Wait for a message to come in from the network
        """
        def _read(n: int) -> bytes:
            return self._sock.recv(n)

        try:
            msg = self._protocol.unwrap_packet(_read)
            if msg is None:
                self._log.warning("Failed receiving message")
            else:
                self._log.debug("Received message id={}".format(msg.msg_id))

        # If the socket gets shutdown (on windows), it may produce an error (as it's "not a socket" anymore)
        except (socket.timeout, OSError):
            self._log.warning("Socket timeout/shutdown. Returning None")
            return None

        return msg
