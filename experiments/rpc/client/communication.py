
import asyncio
import queue
import typing

import rpc
import protocol


class MessageEvent(asyncio.Event):
    """
    Custom event that allows us to introduce an asnc boundary when waiting for a response to come in
    """
    value: typing.Optional[rpc.Message] = None


# NOTE: This class handles communication between the plugin and the reader/writer threads
# To enable asynchronous communication across the network
class CommunicationHandler(object):

    def __init__(self, write_queue: 'queue.Queue[rpc.Message]', logger: typing.Optional[typing.Any] = None) -> None:
        self._logger = logger
        self._write_queue = write_queue
        self._waiting_messages: typing.Dict[str, MessageEvent] = {}

    @property
    def write_queue(self) -> 'queue.Queue[rpc.Message]':
        return self._write_queue

    @property
    def waiting_messages(self) -> typing.Dict[str, MessageEvent]:
        return self._waiting_messages

    # Send the message along the line and get a future response
    def send(self, msg: rpc.Message) -> MessageEvent:
        self._write_queue.put(msg)
        self._waiting_messages[msg.msg_id] = MessageEvent()
        return self._waiting_messages[msg.msg_id]

    # Delete the future response as I don't care about it
    # NOTE: This is for internal cleanup when dealing with no-return rpcs from the server
    # TODO: Do I need code to explicitly drop it if it does come in?
    def drop_message(self, msg: rpc.Message):
        if msg.msg_id in self._waiting_messages:
            del self._waiting_messages[msg.msg_id]

    # Helper method to send a message and immediately wait for it's response
    # This differs from send in that send allows for more freedom in choosing when results are needed
    # TODO: Is the typing okay? Should we foist the error handling for this case onto the client
    async def wait_response(self, msg: rpc.Message) -> typing.Optional[rpc.Message]:
        continuation = self.send(msg)
        await continuation.wait()

        resp = continuation.value
        del self._waiting_messages[msg.msg_id]
        return resp


# NOTE: This class handles communication between the reader/writer threads and the network socket
# To abstract away any protocol/network specific dependencies
class NetworkQueue(object):
    """
    Helper object to manage direct interactions with the rpc socket
    """

    def __init__(self, socket, proto: protocol.Protocol, logger) -> None:
        self._sock = socket
        self._logger = logger
        self._protocol = proto

    def send_message(self, msg: rpc.Message) -> None:
        packet = self._protocol.make_packet(msg)
        self._sock.sendall(packet)

        if self._logger:
            self._logger.info("Sent message {}".format(msg))

    def get_message(self) -> typing.Optional[rpc.Message]:
        def _read(n: int) -> bytes:
            return self._sock.recv(n)

        msg = self._protocol.unwrap_packet(_read)
        if self._logger:
            if msg is not None:
                self._logger.info("Received message {}".format(msg.msg_id))
            else:
                self._logger.error("Failed receiving message!!!")
        return msg
