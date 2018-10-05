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
from common.msg import Message

import plugins as plugin_system

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


# NOTE: These would all be methods of a 'CommChannel' class
# The reader and writer would be put into a thread
# Then we keep the comm channel into the handlers to interact with it
# NOTE: These don't have to be methods
#
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
            yield Message({ k: v for k, v in msg.items() if v is not None })

    except ConnectionResetError:
        log.info("Lost connection to server")


def reader(comm, plugins, socket):
    """
    Thread callback responsible for dispatching messages sent to this plugin
    """
    try:
        for msg in get_messages(socket):
            if msg.action == Message.quit():
                break

            elif msg.id in comm._event_queue:
                comm._event_queue[msg.id].value = msg
                comm._event_queue[msg.id].set()

            elif msg.action in comm._handles:
                comm._handles[msg.action](plugin, msg, comm)

    except:
        log.error("EXCEPTION: " + traceback.format_exc())


def send_message(socket, msg):
    """
    Send a message with the correct network protocol (as expected by device-managers)
    """
    log.info("SENDING <{}>".format(msg))
    if isinstance(msg, Message):
        msg = msg.finalize()
    data = json.dumps(msg).encode('utf-8')
    frame = struct.pack(">I", len(data))
    socket.sendall(frame + data)


def writer(comm, plugin, socket):
    """
    Thread callback responsible for sending messages out of the plugin
    """
    while True:
        msg = comm._msg_queue.get()

        # TODO: Add in some stuff ?

        send_message(socket, msg)


def handshake(plugin_handles, queue):
    log.info("Performing Initial Plugin Handshake")
    queue.put(Message({ 'action': 'handshake', 'hooks': list(plugin_handles.keys()) }))


# TODO: where do I create the plugin instance in the old code?
def run():
