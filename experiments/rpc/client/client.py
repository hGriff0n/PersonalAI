
import json
import socket
import struct

# TODO: Handle errors
# TODO: Wrap in rpc interface
# TODO: Make asynchronous?
    # Apps should be able to "register_rpc" in the host somehow
        # This would allow other apps to call those rpcs and those would be forwarded to the app
        # The server handling code must be separate/asynchronous from the client code

# Connect to server
addr = "127.0.0.1:6142".split(':')
sock = socket.socket()
sock.connect((addr[0], int(addr[1])))

# Construct rpc call
# TODO: Wrap these in a client object (auto-generate?)
msg = {
    "call": "register_app",
    "args": {
        "handles": [
            "tell_story",
            "list_books"
        ]
    },
    # "resp": None,
    "msg_id": "foo",
    "app_id": "me",
    # TODO: Decide on other information to send as metadata
        # How to identify specific locations (such as with speakers?)
}

# Send rpc to server
data = json.dumps(msg).encode('utf-8')
frame = struct.pack('>I', len(data))
sock.sendall(frame + data)
print("Send {} to server.....".format(msg))

# Wait for response
# TODO: Handle errors
len_buf = sock.recv(4)
msg_len = struct.unpack('>I', len_buf)[0]
msg_buf = sock.recv(msg_len)
msg = json.loads(msg_buf.decode('utf-8'))
print("Received {}".format(msg))

print("Keeping the client open to test whether the connection gets closed")
while True:
        pass
