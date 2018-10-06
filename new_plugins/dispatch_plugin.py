#!/usr/bin/env python3

import logging
import threading

from wit import Wit

# from common.msg import Message
# from common import plugins

import plugins
from msg import Message

# NOTE: This app is wholely responsible for receiving a 'msg' json object from the server and running it through
    # wit.ai to determine what the message is wanting to perform in the context of the system
# TODO: Improve code quality and naming

class DispatchPlugin(plugins.Plugin):
    def __init__(self, logger, config=None):
        super.__init__(self, logger, config)

        self._log = logger
        self._client = Wit('CM7NKOIYX5BSFGPOPYFAWZDJTZWEVPSR', logger=logger)

        self._register_handle('dispatch', DispatchPlugin.handle_dispatch)

    async def run(self, comm):
        return True

    async def handle_dispatch(self, msg, comm):
        dispatch = msg.args[0]
        self._log.info("MSG <{}>".format(dispatch))

        query = self._client.message(dispatch)
        quest = query['entities']

        # TODO: We want to split this out into handlers?
        if 'intent' in quest:
            self._log.info("INTENT <{}>".format(quest['intent']))
            intent = quest['intent'][0]

            if intent == 'stop':
                msg.set_stop()

            elif intent == 'play_music':
                song = 'Magnet'

                if 'search_query' in quest:
                    song = quest['search_query'][0]['value']

                self._log.info("PLAYING <{}>".format(song))

                msg.set_action('play')
                msg.set_args(song)
                msg.set_dest(role='audio')

            elif intent == 'find':
                search = Message()
                search.set_sender(self, 'dispatch')
                search.set_destination(role='search')
                search.set_parent_id(msg.id)
                search.set_action('search')

                if 'search_query' in quest:
                    search.set_args(q['value'] for q in quest['search_query'])
                    self._log.info("FINDING <{}>".format(search.args))

                resp = await comm.wait_for_response(search)
                self._log.info("RESPONSE <{}>".format(resp.response))

                msg.set_response(resp.response)

            else:
                self._log.info("Received unknown message")
                msg.set_action('unk')
                msg.set_args("I can't do that")

        elif 'greetings' in quest:
            self._log.info("GREETING")
            msg.set_action('greet')
            msg.set_args("Hello")

        elif 'thanks' in quest:
            self._log.info("GREETING")
            msg.set_action('ack')
            msg.set_args("You're welcome")

        elif 'bye' in quest:
            self._log.info("GOODBYE")
            msg.set_action('bye')
            msg.set_args("Goodbye")
            msg.set_stop()

        else:
            self._log.info("Unknown action")
            msg.set_action('unk')
            msg.set_args("I have no idea what you meant")

        msg.return_to_sender()
        comm.send(msg)
