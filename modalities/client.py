
import json, struct

import asyncio

host = 'localhost'
port = 6142

loop = asyncio.get_event_loop()
writer_queue = loop.create_future()

# Simple framing functions for interop with the rust server
def send_packet(socket, msg):
    data = json.dumps(msg)
    frame = struct.pack(">I", len(data))
    socket.write(frame + data.encode())

async def read_packet(socket):
    len_buf = await socket.read(4)
    msg_len = struct.unpack(">I", len_buf)[0]
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

    # TODO: Add in manual shutdown event?

    return True

async def handle_queries(reader, loop, *args):
    try:
        cont = True
        while cont:
            msg = await read_packet(reader)
            cont = dispatch(msg, loop, *args)

    # Let's just stop the app once the connection is lost for now
    except ConnectionResetError:
        print("Connection to server lost")
        broadcast("quit", loop)

async def handle_requests(writer):
    global writer_queue
    await init_handshake(writer)

    while True:
        msg = await writer_queue
        if msg == "quit": break
        send_packet(writer, msg)
        await writer.drain()

    writer.close()

async def init_handshake(writer):
    send_packet(writer, { 'msg': 'hello' })
    await writer.drain()


# https://stackoverflow.com/questions/49275895/asyncio-multiple-concurrent-servers
async def tcp_echo_client(loop):
    reader, writer = await asyncio.open_connection('127.0.0.1', port, loop=loop)

    reader = asyncio.ensure_future(handle_queries(reader, loop), loop=loop)
    writer = asyncio.ensure_future(handle_requests(writer), loop=loop)
    await asyncio.gather(reader, writer)


loop.run_until_complete(tcp_echo_client(loop))
loop.close()
