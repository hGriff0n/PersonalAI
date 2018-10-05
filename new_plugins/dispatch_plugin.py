#!/usr/bin/env python3

import logging
import threading

from wit import Wit

from common.msg import Message

import plugins

# NOTE: This app is wholely responsible for receiving a 'msg' json object from the server and running it through
    # wit.ai to determine what the message is wanting to perform in the context of the system
# TODO: Improve code quality and naming

class DispatchPlugin(plugins.Plugin):
    def __init__(self, logger, config=None):
        super.__init__(self, logger, config)

        self._log = logger
        self._client = Wit('CM7NKOIYX5BSFGPOPYFAWZDJTZWEVPSR', logger=logger)

        self._register_handle("dispatch", DispatchPlugin.handle_dispatch)

    def run(self, comm):
        return True

    async def handle_dispatch(self, msg, comm):
        query = self._client.message(msg['dispatch'])
        self._log.info("MSG <{}>".format(msg['dispatch']))

        quest = msg['dispatch']['entities']
        if 'intent' in quest:
            self.log.info("INTENT <{}>".format(quest['intent']))

        elif 'greetings' in quest:
            self.log.info("GREETING")
            answer.action('greet')
            answer.message("Hello")

        elif 'thanks' in quest:
            self.log.info("GREETING")
            answer.action('ack')
            answer.message("You're welcome")

        elif 'bye' in quest:
            self.log.info("GOODBYE")
            answer.message("Goodbye")
            answer.action('bye')
            answer['stop'] = True

        else:
            self.log.info("Unknown action")
            answer.action('unk')
            answer.message("I have no idea what you meant")

        self.send(answer)
