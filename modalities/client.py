
import asyncio, json, struct

# Simple class to wrap length delimited framing around socket communication
class Socket:
    # NOTE: I have to pass the reader and writer because init can't be async
    def __init__(self, reader, writer):
        self.reader = reader
        self.writer = writer

    async def write(self, msg):
        data = json.dumps(msg).encode('utf-8')
        frame = struct.pack(">I", len(data))
        self.writer.write(frame + data)
        await self.writer.drain()

    async def read(self):
        len_buf = await self.reader.read(4)
        msg_len = struct.unpack(">I", len_buf)[0]
        buf = await self.reader.read(msg_len)
        return json.loads(buf.decode('utf-8'))

    def close(self):
        self.writer.close()

# Client that automatically handles asynchronous networking communication
# Utilizes the 'dispatch' function to handle server requests
# Utilizes the 'broadcast' method to handle write requests
class Client:
    def __init__(self, socket, loop):
        self.conn = socket
        self.loop = loop
        self.queue = loop.create_future()

        self.threads = [
            asyncio.ensure_future(self.handle_queries(), loop=loop),
            asyncio.ensure_future(self.handle_requests(), loop=loop)
        ]

    # NOTE: This doesn't seem to work for the initial handshake message
    def broadcast(self, msg):
        if not self.queue.done():
            self.queue.set_result(msg)
        self.queue = self.loop.create_future()

    async def handle_queries(self):
        try:
            cont = True
            while cont:
                msg = await self.conn.read()
                cont = dispatch(msg, self)

        # Let's just stop the app once the connection is lost for now
        except ConnectionResetError:
            print("Connection to server lost")
            self.broadcast("quit")

    async def _init_handshake(self):
        await self.conn.write({ 'msg': 'hello' })

    async def handle_requests(self):
        await self._init_handshake()

        while True:
            msg = await self.queue
            if msg == "quit": break
            await self.conn.write(msg)

        self.conn.close()

    async def close(self):
        await asyncio.gather(*self.threads)


# NOTE: This is the customization point for how the app controls
# TODO: Figure out how to allow people to customize this
def dispatch(msg, client):
    client.broadcast(msg)

    # TODO: Add in manual shutdown event?

    return True



# https://stackoverflow.com/questions/49275895/asyncio-multiple-concurrent-servers
async def run_client(host, port, loop):
    reader, writer = await asyncio.open_connection(host, port, loop=loop)
    client = Client(Socket(reader, writer), loop)
    await client.close()

def main():
    host = '127.0.0.1'
    port = 6142

    loop = asyncio.get_event_loop()
    loop.run_until_complete(run_client(host, port, loop))
    loop.close()

if __name__ == "__main__":
    main()
