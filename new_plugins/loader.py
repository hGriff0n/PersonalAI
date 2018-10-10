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


class MessageEvent(asyncio.Event):
    """
    Custom event
    """
    value = None


class CommChannel:
    """
    Custom communication channel to wrap client behaviors
    """
    def __init__(self, handles):
        self._event_queue = {}
        self._msg_queue = queue.Queue()
        self._handles = handles

    async def wait_for_response(self, msg):
        """
        Send a message to some other plugin and wait for a response message
        """
        self.send(msg)
        self._event_queue[msg.id] = MessageEvent()
        await self._event_queue[msg.id].wait()

        resp = self._event_queue[msg.id].value
        del self._event_queue[msg.id]
        return resp

    def send(self, msg):
        self._msg_queue.put(msg)

    def get_msg(self):
        return self._msg_queue.get()

    @property
    def handles(self):
        return self._handles

    @property
    def events(self):
        return self._event_queue


class _JsonProtocol:
    @staticmethod
    def send_message(msg, socket, log):
        """
        Automaticaly wrap message in correct json protocol
        """
        if isinstance(msg, Message):
            msg = msg.json_packet
        log.info("SENDING <{}>".format(msg))

        data = json.dumps(msg).encode('utf-8')
        frame = struct.pack(">I", len(data))
        socket.sendall(frame + data)

    @staticmethod
    def get_messages(socket, log):
        """
        Generator to automatically parse json protocol
        """
        try:
            while True:
                len_buf = socket.recv(4)
                msg_len = struct.unpack(">I", len_buf)[0]
                buf = socket.recv(msg_len)
                msg = json.loads(buf.decode('utf-8'))

                log.debug("RECEIVED <{}>".format(msg))
                yield Message.from_json(msg)

        except ConnectionResetError:
            log.info("Lost connection to server")


def reader(comm, plugin, socket, loop, log):
    """
    Thread callback to dispatch and handle messages sent to this plugin
    """
    async def _exc_wrapper(msg):
        """
        Wrap asyncio handle to catch and report thrown exceptions
        """
        try:
            await comm.handles[msg.action](plugin, msg, comm)

        except Exception as e:
            log.error("EXCEPTION: " + traceback.format_exc())

            msg.action = 'error'
            msg.args = str(e)
            msg.return_to_sender()
            comm.send(msg)

    # For every message that we receive from the server
    for msg in _JsonProtocol.get_messages(socket, log):
        if Message.is_quit(msg):
            log.debug("Received quit message in reader thread")
            break

        # If we have requested this message in some other handler
        elif msg.id in comm.events:
            comm.events[msg.id].value = msg
            comm.evetns[msg.id].set()

        # Otherwise call the registered plugin handler
        elif msg.action in comm.handles:
            asyncio.run_coroutine_threadsafe(_exc_wrapper(msg), loop=loop)

        else:
            log.debug("Received unexpected message <{}>".format(msg))


def writer(comm, socket, log):
    """
    Thread callback responsible for sending messages out of the plugin
    """
    while True:
        msg = comm.get_msg()
        _JsonProtocol.send_message(msg, socket, log)

        if Message.is_quit(msg):
            log.debug("Received quit message in writer thread")
            break


def handshake(plugin, plugin_handles, comm, log):
    log.info("Performing Initial Plugin Handshake")

    msg = Message(plugin=plugin)
    msg.action = 'handshake'
    msg.args = plugin_handles.keys()
    msg.send_to(role='manager')

    comm.send(msg)


async def run(plugin, comm, log, read_thread, write_thread):
    """
    Run the plugin within the asyncio event loop

    TODO: Should I add in an `asyncio.sleep` call here or should the plugins handle that?
    """
    try:
        while await plugin.run(comm):
            if not write_thread.is_alive() or read_thread.is_alive():
                log.debug("Stopping plugin because communication thread has stopped")
                break

    except:
        log.error("EXCEPTION: " + traceback.format_exc())

    finally:
        msg = Message(plugin=plugin)
        msg.action = Message.STOP
        msg.send_to(role='manager')
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
    sock, num_connection_attempts = socket.socket(), 0

    # Handle connection errors
    while True:
        log.debug("Attempting to connect to {}:{}".format(host, port))
        num_connection_attempts += 1

        try:
            sock.connect((host, port))
            break

        except socket.error as e:
            if num_connection_attempts == loader_args['max_retries']:
                raise e

            log.debug("Connection failed. Sleeping for {} seconds".format(loader_args['retry_delay']))
            time.sleep(loader_args['retry_delay'])

    log.info("Connected to {}:{}".format(host, port))

    # Create the communication threads
    comm = CommChannel(handles)
    loop = asyncio.get_event_loop()
    read_thread = threading.Thread(target=reader, args=(comm, socket, plugin, loop, log))
    read_thread.start()
    write_thread = threading.Thread(target=writer, args=(comm, socket, log))
    write_thread.start()

    # Run the plugin
    handshake(plugin, handles, comm, log)
    loop.run_until_complete(run(plugin, comm, log, read_thread, write_thread))
    log.debug("Quit plugin while {} handles were running".format(len(asyncio.Task.all_tasks())))

    # Clean up everything
    write_thread.join()
    read_thread.join()
    sock.close()
