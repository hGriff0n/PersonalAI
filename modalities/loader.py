#!/usr/bin/env python3

import json
import plugins
import queue
import socket
import struct
import sys
import threading
import traceback

import argparse

from common import logger
from common.msg import Message

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
            msg = json.loads(buf.decode('utf-8'))
            yield Message({ k: v for k, v in msg.items() if v is not None })

    except ConnectionResetError:
        log.info("Lost connection to server")

    finally:
        return

def send_message(socket, msg):
    log.info("SENDING <{}>".format(msg))
    if isinstance(msg, Message):
        msg = msg.finalize()
    data = json.dumps(msg).encode('utf-8')
    frame = struct.pack(">I", len(data))
    socket.sendall(frame + data)

# NOTE: This function automatically finishes when the server drops the connection
def reader(plugin, socket, queue):
    try:
        for msg in get_messages(socket):
            log.info("RECEIVED <{}>".format(msg))
            plugin.dispatch(msg, queue)

    except:
        log.error("EXCEPTION: " + traceback.format_exc())

    finally:
        queue.put("quit")

# NOTE: This function must decide to stop sending messages to the server
def writer(socket, queue):
    try:
        while True:
            msg = queue.get()
            if msg == 'quit': break
            send_message(socket, msg)

    finally:
        send_message(socket, Message({ 'action': 'quit' }))

# Send the initial handshake information for the server
def handshake(plugin, queue):
    log.info("INITATING HANDSHAKE")
    queue.put(Message({ 'action': 'handshake', 'hooks': plugin.get_hooks() }))


if __name__ == "__main__":
    queue = queue.Queue()

    # Parse the command line for the loader arguments
    parser = argparse.ArgumentParser(description='Load personalai plugin')
    parser.add_argument('plugin', help='plugin to load')
    parser.add_argument('--plugin-dir', type=str, help='location of plugins')
    [loader_args, plugin_args] = parser.parse_known_args()
    loader_args = vars(loader_args)


    # Load the specified plugin
    name = loader_args['plugin'][0]

    log = logger.create('loader.log', name='__loader__')
    log.setLevel(logger.logging.INFO)

    plugin = plugins.load(name, log=log, args=plugin_args, _dir=loader_args['plugin_dir'])


    # Launch the networking threads (for communicating with the device manager)
    host, port = '127.0.0.1', 6142
    sock = socket.socket()

    log.info("Attempting to connect to {}:{}".format(host, port))
    sock.connect((host, port))
    log.info("Connected to {}:{}".format(host, port))

    read_thread = threading.Thread(target=reader, args=(plugin, sock, queue,))
    write_thread = threading.Thread(target=writer, args=(sock, queue,))

    write_thread.start()
    read_thread.start()


    # Run the selected plugin
    handshake(plugin, queue)

    try:
        # NOTE: This function doesn't directly interact with anything outside of this program
        # Therefore, we have to check whether we need to continue running outside of the function
        while plugin.run(queue):
            if not write_thread.is_alive() or not read_thread.is_alive():
                log.info("Stopping because a thread has stopped")
                break

    except:
        log.error("EXCEPTION: " + traceback.format_exc())

    finally:
        queue.put("quit")

    write_thread.join()
    read_thread.join()
    sock.close()
