#!/usr/bin/env python3

import json
import plugins
import queue
import socket
import struct
import sys
import threading

# log = logger.create('cli.log')
# log.setLevel(logger.logging.INFO)

# TODO: Add in logging (in a correct package)
# TODO: Need to add in a console lock to interleave input/output
    # This lock should be "breakable" after a little while of non-use
# TODO: Closing the server should close this app


def get_messages(socket):
    try:
        while True:
            len_buf = socket.recv(4)
            msg_len = struct.unpack(">I", len_buf)[0]
            buf = socket.recv(msg_len)
            yield json.loads(buf.decode('utf-8'))

    except ConnectionResetError:
        print("Connection to server lost")

    finally:
        return

def send_message(socket, msg):
    data = json.dumps(msg).encode('utf-8')
    frame = struct.pack(">I", len(data))
    socket.sendall(frame + data)

def reader(plugin, socket, queue):
    for msg in get_messages(socket):
        plugin.dispatch(msg, queue)
    queue.put("quit")

def writer(socket, queue):
    try:
        while True:
            msg = queue.get()
            if msg == "quit": break
            send_message(socket, msg)
    finally:
        send_message(socket, { 'action': 'quit' })

# Send the initial handshake information for the server
def handshake(plugin, queue):
    queue.put({ 'msg': 'hello' })

if __name__ == "__main__":
    queue = queue.Queue()
    name = sys.argv[1]
    plugin = plugins.load("cli")

    sock = socket.socket()
    sock.connect(('127.0.0.1', 6142))

    read_thread = threading.Thread(target=reader, args=(plugin, sock, queue,))
    write_thread = threading.Thread(target=writer, args=(sock, queue,))

    write_thread.start()
    read_thread.start()

    # TODO: Need an automatic way of stopping the `run` method when the server shuts down
    handshake(plugin, queue)
    plugin.run(queue)

    write_thread.join()
    read_thread.join()
    sock.close()

# API Documentation:
#   Pyro4: https://pythonhosted.org/Pyro4/
