
import json, struct

import asyncio

host = 'localhost'
port = 6142

loop = asyncio.get_event_loop()
writer_queue = loop.create_future()


def send_packet(socket, msg):
    data = json.dumps(msg)
    frame = struct.pack(">I", len(data))
    socket.write(frame + data.encode())

async def read_packet(socket):
    msg_len = struct.unpack(">I", await socket.read(4))[0]
    buf = await socket.read(msg_len)
    return json.loads(buf.decode())

# This doesn't seem to work for the initial startup
    # But does work for communications afterwards (why ??)
def broadcast(msg, loop):
    global writer_queue
    if not writer_queue.done():
        writer_queue.set_result(msg)
    writer_queue = loop.create_future()

def dispatch(msg, loop):
    print(msg)
    broadcast(msg, loop)
    return ()

async def handle_queries(reader, loop, *args):
    while True:
        msg = await read_packet(reader)
        dispatch(msg, loop, *args)

async def handle_requests(writer):
    global writer_queue
    while True:
        msg = await writer_queue
        send_packet(writer, msg)
        await writer.drain()

    writer.close()

def perform_handshake(loop):
    broadcast({ 'msg': 'hello' }, loop)

# I want to be able to send multiple messages to the server without requiring a response
# And to be able to hear multiple messages from the server without giving a response
    # https://stackoverflow.com/questions/49275895/asyncio-multiple-concurrent-servers
async def tcp_echo_client(loop):
    reader, writer = await asyncio.open_connection('127.0.0.1', port, loop=loop)

    reader = asyncio.ensure_future(handle_queries(reader, loop), loop=loop)

    send_packet(writer, { 'msg': 'hello'})
    writer = asyncio.ensure_future(handle_requests(writer), loop=loop)
    # perform_handshake(loop)

    # Shutting down needs to be more graceful
    await asyncio.gather(reader, writer)


loop.run_until_complete(tcp_echo_client(loop))
loop.close()
