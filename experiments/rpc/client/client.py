
import abc
import json
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
