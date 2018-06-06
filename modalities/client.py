
import json, socket, struct

host = 'localhost'
port = 6142

def add_frame(data):
    return struct.pack(">I", len(data))

def send(socket, msg):
    data = json.dumps(msg)
    frame = add_frame(data)
    socket.sendall(frame + data.encode())

def get_length_framed_message(socket):
    msg_len = struct.unpack(">I", socket.recv(4))[0]
    msg_buf = socket.recv(msg_len).decode('utf-8')
    return json.loads(msg_buf)

# This simply connects to the host and then sends a hello message
client = socket.socket()
client.connect((host, port))

send(client, { 'msg': 'hello', 'action': 'quit' })
msg = get_length_framed_message(client)
print(msg)

# The server then fails here with:
# { code: 10054, kind: ConnectionReset, message: "An existing connection was forcibly closed by the remote host." }
