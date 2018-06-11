#!/usr/bin/env python3

import json
import plugins
import queue
import socket
import struct
import sys
import threading
import traceback

from common import logger

log = None

# TODO: Add in config support
    # Enable specifying the host, port, and log directory
# TODO: Closing the server should close this as well


def get_messages(socket):
    try:
        while True:
            len_buf = socket.recv(4)
            msg_len = struct.unpack(">I", len_buf)[0]
            buf = socket.recv(msg_len)
            yield json.loads(buf.decode('utf-8'))

    except ConnectionResetError:
        log.info("Lost connection to server")

    finally:
        return

def send_message(socket, msg):
    log.info("SENDING <{}>".format(msg))
    data = json.dumps(msg).encode('utf-8')
    frame = struct.pack(">I", len(data))
    socket.sendall(frame + data)

def reader(plugin, socket, queue):
    try:
        for msg in get_messages(socket):
            log.info("RECEIVED <{}>".format(msg))
            plugin.dispatch(msg, queue)

    except:
        log.error("EXCEPTION: " + traceback.format_exc())

    finally:
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
    log.info("INITATING HANDSHAKE")
    queue.put({ 'msg': 'hello' })

if __name__ == "__main__":
    queue = queue.Queue()
    name = sys.argv[1]

    log = logger.create('loader.{}.log'.format(name))
    log.setLevel(logger.logging.INFO)

    plugin = plugins.load(name, log=log)

    host, port = '127.0.0.1', 6142
    sock = socket.socket()

    log.info("Attempting to connect to {}:{}".format(host, port))
    sock.connect((host, port))
    log.info("Connected to {}:{}".format(host, port))

    read_thread = threading.Thread(target=reader, args=(plugin, sock, queue,))
    write_thread = threading.Thread(target=writer, args=(sock, queue,))

    write_thread.start()
    read_thread.start()

    # TODO: Need an automatic way of stopping the `run` method when the server shuts down
    handshake(plugin, queue)
    log.info("ENTERING {}".format(name))

    try:
        plugin.run(queue)

    except:
        log.error("EXCEPTION: " + traceback.format_exc())
        queue.put("quit")

    write_thread.join()
    read_thread.join()
    sock.close()
