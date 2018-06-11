#!/usr/bin/env python3

import threading

from common import logger
from plugins import Plugin

from wit import Wit

# NOTE: This app is wholely responsible for receiving a 'msg' json object from the server and running it through
    # wit.ai to determine what the message is wanting to perform in the context of the system
# TODO: Add in the capacity for basic routing capabilities (ie. send music request from cli to audio app)
    # I'm not sure where this capacity will reside in the full system architecture though
    # I'm not even sure where **this** app will reside though, so I'm just going to go ahead


class DispatchPlugin(Plugin):
    def __init__(self):
        self.log = logger.create('dispatch.log')
        self.log.setLevel(logger.logging.INFO)

        self.client = Wit('CM7NKOIYX5BSFGPOPYFAWZDJTZWEVPSR', logger=self.log)
        return

    # I don't think I actually do much here
    # We can't exit this loop properly
    def run(self, queue):
        while True:
            continue

    def dispatch(self, msg, queue):
        self.log.info("Received <{}>".format(msg))
        if 'msg' in msg:
            action = self.client.message(msg['msg'])
            self.log.info("MSG <{}>".format(str(action)))

            return self.perform_dispatch(action, queue)

        if 'stop' in msg:
            queue.send('quit')
            return False

        self.log.info("Received unusable json communication")
        self.log.info("COMM <{}>".format(msg))
        return True

    def perform_dispatch(self, msg, queue):
        action = msg['entities']

        if 'intent' in action:
            # TODO: Dispatch on the intents
            self.log.info("INTENTS <{}>".format(action['intent']))

            intent = action['intent'][0]
            answer = { 'stop': ('stop' == intent['value']), 'to_sender': True }

            if intent['value'] == "play_music":
                song = 'Magnet'

                if 'search_query' in action:
                    song = action['search_query'][0]['value']

                self.log.info("PLAYING <{}>".format(song))
                answer['play'] = song
                answer['to_sender'] = False

            else:
                answer['text'] = "I can't do that"

            queue.put(answer)

        elif 'greetings' in action:
            self.log.info("GREETING")
            queue.put({ 'text': 'Hello', 'to_sender': True })

        elif 'thanks' in action:
            self.log.info("GREETING")
            queue.put({ 'text': 'Your welcome', 'to_sender': True })

        elif 'bye' in action:
            self.log.info("GOODBYE")
            queue.put({ 'text': 'Goodbye', 'stop': True, 'to_sender': True })
            return False

        else:
            self.log.info("Unknown action")
            queue.put({ 'text': "I have no idea what you meant", 'to_sender': True })

        return True
