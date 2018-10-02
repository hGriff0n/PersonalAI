#!/usr/bin/env python3

import logging
import threading

from wit import Wit

from common.msg import Message
from common.plugins import Plugin

# NOTE: This app is wholely responsible for receiving a 'msg' json object from the server and running it through
    # wit.ai to determine what the message is wanting to perform in the context of the system
# TODO: Improve code quality and naming


class DispatchPlugin(Plugin):
    def __init__(self, logger, config=None):
        self.client = Wit('CM7NKOIYX5BSFGPOPYFAWZDJTZWEVPSR', logger=logger)

        self.log = logger
        self.log.setLevel(logging.INFO)
        self.log.info("Finished initialization")

    def run(self, queue):
        return True

    def dispatch(self, msg, queue):
        if 'stop' in msg and msg['stop']:
            return Message.stop()

        if 'dispatch' in msg:
            msg['dispatch'] = self.client.message(msg['dispatch'])
            self.log.info("MSG <{}>".format(msg['dispatch']))
            self.perform_dispatch(msg, queue)

        else:
            self.log.info("Received unusable json communication")
            self.log.info("COMM <{}>".format(msg))

        return ""

    def perform_dispatch(self, msg, queue):
        quest = msg['dispatch']['entities']
        answer = Message(msg['from'])

        if 'intent' in quest:
            # TODO: Dispatch on the intents
            self.log.info("INTENTS <{}>".format(quest['intent']))

            intent = quest['intent'][0]
            answer['stop'] = ('stop' == intent['value'])

            if intent['value'] == "play_music":
                song = 'Magnet'

                if 'search_query' in quest:
                    song = quest['search_query'][0]['value']

                self.log.info("PLAYING <{}>".format(song))

                # NOTE: Here I assume the server is responsible for splitting the messages
                answer['play'] = song
                answer.action('play')
                answer.message("Playing {}".format(song))
                answer.send_to('audio')

            elif intent['value'] == "find":
                answer.action('search')

                # NOTE: I'm assuming this sends the results back where we want them to
                if 'search_query' in quest:
                    answer['query'] = quest['search_query'][0]['value']
                    self.log.info("FINDING <{}>".format(answer['query']))

            else:
                answer.action('unk')
                answer.message("I can't do that")

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

        queue.put(answer)

    def get_hooks(self):
        return [ 'dispatch' ]

# API Documentation:
#   wit: https://github.com/wit-ai/pywit
