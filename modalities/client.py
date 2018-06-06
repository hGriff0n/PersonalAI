
import json, socket, struct

host = 'localhost'
port = 6142

# This simply connects to the host and then sends a hello message
client = socket.socket()
client.connect((host, port))

data = json.dumps({ 'msg': 'hello', 'action': 'quit' })
# Add the length framing that the rust server expects
data = struct.pack(">I", len(data)) + data.encode()
client.sendall(data)

# The server then fails here with:
# { code: 10054, kind: ConnectionReset, message: "An existing connection was forcibly closed by the remote host." }
