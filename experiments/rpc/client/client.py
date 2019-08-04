
import abc
import asyncio
import json
import queue
import socket
import struct
import typing

import rpc
import protocol

# TODO: Handle errors
# TODO: Make asynchronous?
    # Apps should be able to "register_rpc" in the host somehow
        # This would allow other apps to call those rpcs and those would be forwarded to the app
        # The server handling code must be separate/asynchronous from the client code


# NOTE: This class handles communication between the reader/writer threads and the network socket
# To abstract away any protocol/network specific dependencies
class ConnectionHandler(object):
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


# TODO: Type `dispatcher` - run_coroutine_threadsafe has an expected type that `Callable` wasn't matching
# TODO: Type `loop` - it's an asyncio Event Loop
def reader(conn: ConnectionHandler, comm: CommunicationHandler, loop, dispatcher, logger: typing.Optional[typing.Any] = None) -> None:
    """
    Thread callback to handle messages as they are received by the plugin
    """
    try:
        while True:
            msg = conn.get_message()

            # This error is already handled
            if msg is None:
                continue

            if msg.msg_id in comm.waiting_messages:
                if logger is not None:
                    logger.info("Received response to message id={}".format(msg.msg_id))
                comm.waiting_messages[msg.msg_id].value = msg
                loop.call_soon_threadsafe(comm.waiting_messages[msg.msg_id].set)

            else:
                if logger is not None:
                    logger.info("Handling message id={} through plugin handle".format(msg.msg_id))
                asyncio.run_coroutine_threadsafe(dispatcher(msg), loop=loop)

    except ConnectionResetError as e:
        if logger is not None:
            logger.error("Lost connection to server: {}".format(e))

    except Exception as e:
        if logger is not None:
            logger.error("Exception while waiting for messages: {}".format(e))


def writer(conn: ConnectionHandler, write_queue: 'queue.Queue[rpc.Message]') -> None:
    """
    Thread callback responsible for sending messages out of the plugin

    This enables us to avoid waiting on the write queue as it not an "async boundary"
    """
    while True:
        msg = write_queue.get()
        conn.send_message(msg)


# TODO: Move the communication into this entry function
async def run() -> None:
    pass


# Construct protocol object
proto = protocol.JsonProtocol(None)

# Connect to server
addr = "127.0.0.1:6142".split(':')
sock = socket.socket()
sock.connect((addr[0], int(addr[1])))
conn = ConnectionHandler(sock, proto, None)

# Construct rpc call
rpc_msg_dict = {
    "call": "register_app",
    "args": {
        "handles": [
            "tell_story",
            "list_books"
        ]
    },
    "msg_id": "foo",
}
rpc_message = rpc.Message.from_dict(rpc_msg_dict)

# Send rpc to server
if rpc_message is None:
    print("Failed to parse {} into an rpc.Message", rpc_msg_dict)
    exit(1)

conn.send_message(rpc_message)
print("Send {} to server.....".format(rpc_message.to_dict()))

# Wait for response
msg = conn.get_message()
if msg is not None:
    print("Received {}".format(msg.to_dict()))
