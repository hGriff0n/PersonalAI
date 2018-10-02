#!/usr/bin/env python3

import logging
import threading

from common.msg import Message
from common.plugins import Plugin

# TODO: Look at moving the logger into the 'Plugin' class
# TODO: Implement a time-keeping system
    # We probably shouldn't print messages that are too out of date
# TODO: Implement periodic "polling" of the input loop
    # I should be able to "switch" out of input to print messages that are pilling up

class CliPlugin(Plugin):
    def __init__(self, logger, config=None):
        self.msgs = []
        self.lock = threading.Lock()

        self.log = logger
        self.log.setLevel(logging.INFO)
        self.log.info("Finished initialization")

    def _print_all(self):
        if len(self.msgs) != 0:
            for msg in self.msgs:
                print(msg)

            self.msgs = []

    def run(self, queue):
        with self.lock:
            self._print_all()

        query = input("> ")
        if query == "":
            self._print_all()
            return True

        if query == "quit":
            self.log.info("STOPPING")
            queue.put(Message({ 'action': 'quit', 'routing': 'broadcast' }))
            return False

        msg = Message('cli')
        msg.dispatch(query)
        queue.put(msg)
        self.log.info("SENT <{}>".format(msg))

        return True

    def dispatch(self, msg, queue):
        if 'text' in msg:
            self.log.debug("Received text action")
            with self.lock:
                self.msgs.append(msg['text'])

        elif 'find' in msg:
            self.log.debug("Received search results")
            with self.lock:
                self.msgs.append(msg)

        self.log.info("RECEIVED <{}>".format(msg))
        return ""

    def get_hooks(self):
        return [ 'cli' ]

# API Documentation:
