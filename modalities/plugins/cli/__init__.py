#!/usr/bin/env python3

import threading

from common import logger
from plugins import Plugin

# TODO: Hook up with dispatch and audio system
    # Move them into separate "plugins"
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
        while True:
            with self.lock:
                self._print_all()

            query = input("> ")
            if query == "quit":
                self.log.info("STOPPING")
                break

            # msg = self.Message({ 'msg': query })
            msg = { 'msg': query }
            queue.put(msg)
            self.log.info("SENT <{}>".format(msg))

        queue.put("quit")

    def dispatch(self, msg, queue):
        if 'text' in msg:
            with self.lock:
                self.msgs.append(msg['text'])
            self.log.info("RECEIVED <{}>".format(msg['text']))

        return True

# Issues with the current framework
    # I can't exactly spawn plugins at runtime (current architecture focused on one plugin per process)
        # Not an issue persay, it works for me but it's not generally how plugins are viewed
        # Honestly, each modality is a plugin for the rust device manager, why do i need the extra level of indirection?
    # Where would logging statements get sent?
