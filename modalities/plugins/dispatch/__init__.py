#!/usr/bin/env python3

import threading

from common import logger
from common.msg import Message
from plugins import Plugin

from wit import Wit

# NOTE: This app is wholely responsible for receiving a 'msg' json object from the server and running it through
    # wit.ai to determine what the message is wanting to perform in the context of the system
# TODO: Improve code quality and naming


class DispatchPlugin(Plugin):
    def __init__(self, config=None):
        self.log = logger.create('dispatch.log')
        self.log.setLevel(logger.logging.INFO)

        self.client = Wit('CM7NKOIYX5BSFGPOPYFAWZDJTZWEVPSR', logger=self.log)
        return

    # I don't think I actually do much here
    # We can't exit this loop properly
    def run(self, queue):
        return True

    def dispatch(self, msg, queue):
        if 'dispatch' in msg:
            msg['dispatch'] = self.client.message(msg['dispatch'])
            self.log.info("MSG <{}>".format(msg['dispatch']))
            self.perform_dispatch(msg, queue)
            return

        if 'stop' in msg and msg['stop']:
            queue.put('quit')
            return

        self.log.info("Received unusable json communication")
        self.log.info("COMM <{}>".format(msg))

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
            # queue.put("quit")

        else:
            self.log.info("Unknown action")
            answer.action('unk')
            answer.message("I have no idea what you meant")

        queue.put(answer)

    def get_hooks(self):
        return [ 'dispatch' ]
