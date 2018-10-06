#!/usr/bin/env python3

import argparse
import asyncio
import json
import queue
import socket
import struct
import sys
import threading
import time
import traceback

from common import logger
# from common.msg import Message
# import common.plugins as plugin_system

import plugins as plugin_system
from msg import Message

log = None

class MessageEvent(asyncio.Event):
    value = None

class CommChannel:
    def __init__(self, handles):
        self._event_queue = {}
        self._msg_queue = queue.Queue()
        self._handles = handles

    async def wait_for_response(self, msg):
        self.send(msg)
        self._event_queue[msg.id] = MessageEvent()
        await self._event_queue[msg.id].wait()

        resp = self._event_queue[msg.id].value
        del self._event_queue[msg.id]
        return resp

    def send(self, msg):
        self._msg_queue.put(msg)


def get_messages(socket):
    """
    Generator to gradually yield messages received on the given socket
    Automatically unpacks the handling applied by device-managers
    """
    try:
        while True:
            len_buf = socket.recv(4)
            msg_len = struct.unpack(">I", len_buf)[0]
            buf = socket.recv(msg_len)
            msg = json.loads(buf.decode('utf-8'))
            yield Message(msg=msg)

    except ConnectionResetError:
        log.info("Lost connection to server")


def exception_wrapper(plugin, handle, msg, comm):
    """
    Wrap the handle calling to catch and report exceptions back to the sender
    """
    try:
        async handle(plugin, msg, comm)

    except Exception as e:
        msg.set_action('error')
        msg.set_args(str(e))
        msg.return_to_sender()
        comm.send(msg)


def reader(comm, plugin, socket, loop):
    """
    Thread callback responsible for dispatching messages sent to this plugin
    """
    try:
        for msg in get_messages(socket):
            if Message.is_quit(msg):
                break

            elif msg.id in comm._event_queue:
                comm._event_queue[msg.id].value = msg
                comm._event_queue[msg.id].set()

            elif msg.action in comm._handles:
                # loop.call_soon_threadsafe(exception_wrapper, plugin, comm._handles[msg.action], msg, comm)
                loop.call_soon_threadsafe(comm._handles[msg.action], plugin, msg, comm)

    except:
        log.error("EXCEPTION: " + traceback.format_exc())


def send_message(socket, msg):
    """
    Send a message with the correct network protocol (as expected by device-managers)
    """
    log.info("SENDING <{}>".format(msg))
    data = json.dumps(msg).encode('utf-8')
    frame = struct.pack(">I", len(data))
    socket.sendall(frame + data)


def writer(comm, socket):
    """
    Thread callback responsible for sending messages out of the plugin
    """
    while True:
        msg = comm._msg_queue.get()

        # TODO: Add in some stuff ?

        send_message(socket, msg)
        if Message.is_quit(msg):
            break


def handshake(plugin, plugin_handles, comm):
    log.info("Performing Initial Plugin Handshake")

    msg = Message()
    msg.set_sender(plugin, 'handshake')
    msg.set_action('handshake')
    msg.set_args(list(plugin_handles.keys()))
    msg.set_destination(role='manager', intra_device=True)

    comm.send(msg)


async def run(plugin, comm, read_thread, write_thread):
    try:
        while await plugin.run(comm):
            if not write_thread.is_alive() or read_thread.is_alive():
                log.info("Stopping plugin because communication thread has stopped")
                break

    except:
        log.error("EXCEPTION: " + traceback.format_exc())

    finally:
        msg = Message()
        msg.set_sender(plugin, 'stop')
        msg.set_action('stop')
        msg.set_destination(role='manager', intra_device=True)

        comm.send(msg)


if __name__ == "__main__":
    # Parse the command line for the loader arguments
    parser = argparse.ArgumentParser(description='Load personalai plugin')
    parser.add_argument('plugin', type=str, nargs=1, help='plugin to load')
    parser.add_argument('--plugin-dir', type=str, help='location of plugins')
    parser.add_argument('--host', type=str, help='ip address of the host server')
    parser.add_argument('--port', type=int, help='port that the server is listening on')
    parser.add_argument('--log-dir', type=str, help='location to write log files')
    parser.add_argument('--retry-delay', type=int, help='Num seconds to sleep in between connection retries')
    parser.add_argument('--max-retries', type=int, help='Maximum retry attempts before connection failed')
    [loader_args, plugin_args] = parser.parse_known_args()
    loader_args = vars(loader_args)

    # Load the specified plugin
    name = loader_args['plugin'][0]
    log = logger.create('loader.log', name='__loader__', log_dir=loader_args['log_dir'], fmt="%(asctime)s <%(levelname)s> [{}] %(message)s".format(name))
    log.setLevel(logger.logging.INFO)

    plugin, handles = plugin_system.load(name, log=log, args=plugin_args, plugin_dir=loader_args['plugin_dir'], log_dir=loader_args['log_dir'])
    if plugin is None:
        log.error("Couldn't load plugin {}".format(name))
        exit(1)

    # Launch the networking threads (for communicating with the device manager)
    host, port = loader_args['host'], loader_args['port']
    sock = socket.socket()
    num_connection_attempts = 0

    # Handle connection errors
    while True:
        log.info("Attempting to connect to {}:{}".format(host, port))
        num_connection_attempts += 1

        try:
            sock.connect((host, port))
            break

        except socket.error as e:
            if num_connection_attempts == loader_args['max_retries']:
                raise e

            log.info("Connection failed. Sleeping for {} seconds".format(loader_args['retry_delay']))
            time.sleep(loader_args['retry_delay'])

    log.info("Connected to {}:{}".format(host, port))

    # Create the communication threads
    comm = CommChannel(handles)
    loop = asyncio.get_event_loop()
    read_thread = threading.Thread(target=reader, args=(comm, socket, plugin, loop))
    write_thread = threading.Thread(target=writer, args=(comm, socket))

    # Run the plugin
    handshake(plugin, handles, comm)
    loop.run_until_complete(run(plugin, comm, read_thread, write_thread))

    # Clean up everything
    write_thread.join()
    read_thread.join()
    sock.close()
