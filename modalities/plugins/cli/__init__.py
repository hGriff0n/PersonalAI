#!/usr/bin/env python3

import threading

from common import logger
from common.msg import Message
from plugins import Plugin

# TODO: Change the logger so that I can log to separate files if I want
    # Look at moving the logger into the 'Plugin' class
# TODO: Implement a time-keeping system
    # We probably shouldn't print messages that are too out of date
# TODO: Implement periodic "polling" of the input loop
    # I should be able to "switch" out of input to print messages that are pilling up

class CliPlugin(Plugin):
    def __init__(self):
        self.msgs = []
        self.lock = threading.Lock()

        self.log = logger.create('cli.log')
        self.log.setLevel(logger.logging.INFO)

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
            return False

        msg = Message(None)
        msg.dispatch(query)
        queue.put(msg)
        self.log.info("SENT <{}>".format(msg))

        return True

    def dispatch(self, msg, queue):
        if 'text' in msg:
            with self.lock:
                self.msgs.append(msg['text'])

        self.log.info("RECEIVED <{}>".format(msg))
        return True

    def get_hooks(self):
        return [ 'cli' ]

# Issues with the current framework
    # I can't exactly spawn plugins at runtime (current architecture focused on one plugin per process)
        # Not an issue persay, it works for me but it's not generally how plugins are viewed
        # Honestly, each modality is a plugin for the rust device manager, why do i need the extra level of indirection?
    # Where would logging statements get sent?
