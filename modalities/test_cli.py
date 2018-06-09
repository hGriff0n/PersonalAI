#!/usr/bin/env python3

import asyncio
import threading
import queue

from client import Socket

# Immediate development work
# Develop this to use the client package to communicate with the rust server (as a plugin)
# Pick a development direction to follow
    # Produce the audio and dispatch apps the same way
# Get back to dispatch_daemon level working system

# log = logger.create('cli.log')
# log.setLevel(logger.logging.INFO)

def dispatch(msg, queue):
    print(msg)
    return True

# TODO: Work on `client.py` package to provie a usable interface
# TODO: Need to add in a console lock to interleave input/output
    # This lock should be "breakable" after a little while of non-use
# TODO: Closing the server should close this app
    # I think I'm actually going to send an explicit "quit" command though
    # Would need to have a wait to stop the cli loop though
# TODO: Add in logging (in a correct package)
async def network_communication(queue, loop, plugin):
    host, port = '127.0.0.1', 6142
    reader, writer = await asyncio.open_connection(host, port, loop=loop)
    socket = Socket(reader, writer)

    async def handle_requests(socket, queue):
        await socket.write({ 'msg': 'hello' })

        while True:
            msg = queue.get()
            if msg == "quit": break
            await socket.write(msg)

        socket.close()

    # TODO: This function isn't being run (apparently unless the server was stopped)
    async def handle_queries(socket, queue):
        try:
            print("hello")
            while True:
                msg = await socket.read()
                if not dispatch(msg, queue): break

        except ConnectionResetError:
            print("Connection to server lost")
        finally:
            queue.put("quit")

    asyncio.gather(*[
        asyncio.ensure_future(handle_requests(socket, queue)),
        asyncio.ensure_future(handle_queries(socket, queue))
    ])

def run_network(queue, plugin):
    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    loop.run_until_complete(network_communication(queue, loop, plugin))
    loop.close()

import plugins

if __name__ == "__main__":
    queue = queue.Queue()

    cli = plugins.load("cli")
    if cli is not None:
        t = threading.Thread(target=run_network, args=(queue, cli, ))
        t.start()
        cli.run(queue)
        # t.join()
    print("done")

# API Documentation:
#   Pyro4: https://pythonhosted.org/Pyro4/
